mod files;
mod identity;
pub mod model;
mod receiver;
mod sender;
mod store;
#[cfg(test)]
mod tests;
mod tls;

pub use files::{validate_manifest, validate_relative};
pub use model::{Device, Peer, Selection, Snapshot, Transfer};
pub use tls::normalize_address;

use anyhow::{bail, Context, Result};
use files::SourcePlan;
use identity::Identity;
use model::{History, Manifest};
use std::{
    collections::{HashMap, HashSet},
    net::{IpAddr, Ipv4Addr, SocketAddr},
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use tokio::sync::{Mutex, RwLock};
use tokio_util::sync::CancellationToken;

pub struct Config {
    pub data_dir: PathBuf,
    pub receive_dir: PathBuf,
    pub port: u16,
    pub name: Option<String>,
    pub discovery: bool,
    pub listen_ip: Ipv4Addr,
}

struct RuntimeState {
    peers: HashMap<String, Peer>,
    transfers: Vec<Transfer>,
    plans: HashMap<String, SourcePlan>,
    warning: Option<String>,
}

pub(crate) struct Incoming {
    manifest: Manifest,
    token: String,
    approved: bool,
    sender_confirmed: bool,
    cancelled: CancellationToken,
    completed: HashSet<usize>,
    active: HashSet<usize>,
    staging: PathBuf,
    text: Option<String>,
    finished: bool,
    created: std::time::Instant,
    last_activity: std::time::Instant,
}

pub(crate) struct Outgoing {
    confirmed: AtomicBool,
    cancelled: CancellationToken,
}

pub struct Core {
    identity: Identity,
    receive_dir: PathBuf,
    port: u16,
    state: RwLock<RuntimeState>,
    incoming: Mutex<HashMap<String, Incoming>>,
    outgoing: Mutex<HashMap<String, Arc<Outgoing>>>,
    nonces: Mutex<HashMap<String, std::time::Instant>>,
    store: store::Store,
    mdns: Option<mdns_sd::ServiceDaemon>,
    server: axum_server::Handle,
    shutdown: CancellationToken,
}

pub fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}
pub(crate) fn is_terminal(status: &str) -> bool {
    matches!(status, "completed" | "failed" | "rejected" | "cancelled")
}

impl Core {
    pub async fn start(config: Config) -> Result<Arc<Self>> {
        let _ = rustls::crypto::ring::default_provider().install_default();
        std::fs::create_dir_all(&config.data_dir)?;
        std::fs::create_dir_all(&config.receive_dir)?;
        let receive_dir = config.receive_dir.canonicalize()?;
        let identity = Identity::load(&config.data_dir, config.name)?;
        let store = store::Store::open(&config.data_dir.join("swoosh.sqlite3"))?;
        let listener = std::net::TcpListener::bind((config.listen_ip, config.port))
            .or_else(|_| std::net::TcpListener::bind((config.listen_ip, 0)))?;
        listener.set_nonblocking(true)?;
        let port = listener.local_addr()?.port();
        let tls = axum_server::tls_rustls::RustlsConfig::from_pem(
            identity.cert_pem.as_bytes().to_vec(),
            identity.key_pem.as_bytes().to_vec(),
        )
        .await?;
        let mdns = if config.discovery {
            mdns_sd::ServiceDaemon::new().ok()
        } else {
            None
        };
        let core = Arc::new(Self {
            identity,
            receive_dir,
            port,
            store,
            mdns,
            state: RwLock::new(RuntimeState {
                peers: HashMap::new(),
                transfers: Vec::new(),
                plans: HashMap::new(),
                warning: None,
            }),
            incoming: Mutex::new(HashMap::new()),
            outgoing: Mutex::new(HashMap::new()),
            nonces: Mutex::new(HashMap::new()),
            server: axum_server::Handle::new(),
            shutdown: CancellationToken::new(),
        });
        let server_core = core.clone();
        let router = receiver::router(core.clone());
        let handle = core.server.clone();
        tokio::spawn(async move {
            if let Err(error) = axum_server::from_tcp_rustls(listener, tls)
                .handle(handle)
                .serve(router.into_make_service())
                .await
            {
                server_core.state.write().await.warning = Some(format!("接收服务停止：{error}"));
            }
        });
        if config.discovery {
            if let Err(error) = core.start_discovery().await {
                core.state.write().await.warning =
                    Some(format!("自动发现不可用，可使用手动连接：{error}"));
            }
        }
        let cleanup_core = core.clone();
        tokio::spawn(async move {
            loop {
                tokio::select! {
                    _ = cleanup_core.shutdown.cancelled() => break,
                    _ = tokio::time::sleep(Duration::from_secs(2)) => cleanup_core.expire_sessions().await,
                }
            }
        });
        Ok(core)
    }

    pub fn device(&self) -> &Device {
        &self.identity.device
    }
    pub fn port(&self) -> u16 {
        self.port
    }

    pub async fn snapshot(&self) -> Result<Snapshot> {
        let state = self.state.read().await;
        let mut peers: Vec<_> = state.peers.values().cloned().collect();
        peers.sort_by(|a, b| a.device.name.cmp(&b.device.name));
        let addresses = local_addresses()
            .into_iter()
            .map(|ip| SocketAddr::new(ip, self.port).to_string())
            .collect();
        Ok(Snapshot {
            device: self.device().clone(),
            addresses,
            receive_dir: self.receive_dir.to_string_lossy().into_owned(),
            peers,
            transfers: state.transfers.clone(),
            history: self.store.list()?,
            network_warning: state.warning.clone(),
        })
    }

    pub async fn select(&self, paths: Vec<PathBuf>) -> Result<Selection> {
        let plan = tokio::task::spawn_blocking(move || files::collect(paths)).await??;
        let selection = plan.selection.clone();
        let mut state = self.state.write().await;
        state.plans.clear();
        state.plans.insert(selection.id.clone(), plan);
        Ok(selection)
    }

    pub async fn clear_selection(&self) {
        self.state.write().await.plans.clear();
    }
    pub fn clear_history(&self) -> Result<()> {
        self.store.clear()
    }

    pub async fn connect(&self, input: &str) -> Result<Peer> {
        self.connect_expected(input, None, false).await
    }

    async fn connect_expected(
        &self,
        input: &str,
        expected: Option<String>,
        discovered: bool,
    ) -> Result<Peer> {
        let address = normalize_address(input).context("连接地址格式错误")?;
        let (client, observed) = tls::client(expected)?;
        let device: Device = client
            .get(format!("https://{address}/v1/info"))
            .send()
            .await
            .context("无法连接，请检查两端是否打开 Swoosh、网络隔离和防火墙")?
            .error_for_status()?
            .json()
            .await?;
        let fingerprint = observed.lock().unwrap().clone().context("未取得设备证书")?;
        if device.fingerprint != fingerprint
            || device.protocol != model::PROTOCOL
            || device.id.len() != 64
            || device.name.len() > 256
            || device.signing_key.len() != 64
            || identity::digest(hex::decode(&device.signing_key)?) != device.id
        {
            bail!("设备信息或协议不匹配");
        }
        if device.id == self.device().id {
            bail!("这是当前设备，请连接另一台设备");
        }
        let peer = Peer {
            device,
            address,
            discovered,
        };
        self.state
            .write()
            .await
            .peers
            .insert(peer.device.id.clone(), peer.clone());
        Ok(peer)
    }

    async fn start_discovery(self: &Arc<Self>) -> Result<()> {
        let mdns = self.mdns.as_ref().context("无法启动 mDNS")?;
        mdns.disable_interface(mdns_sd::IfKind::IPv6)?;
        let info = mdns_sd::ServiceInfo::new(
            model::SERVICE,
            &self.device().id[..16],
            &format!("swoosh-{}.local.", &self.device().id[..16]),
            "",
            self.port,
            &[
                ("id", self.device().id.as_str()),
                ("fingerprint", self.device().fingerprint.as_str()),
                ("version", "1"),
            ][..],
        )?
        .enable_addr_auto();
        mdns.register(info)?;
        let events = mdns.browse(model::SERVICE)?;
        let core = self.clone();
        tokio::spawn(async move {
            loop {
                tokio::select! {
                    _ = core.shutdown.cancelled() => break,
                    event = events.recv_async() => {
                        match event {
                            Ok(mdns_sd::ServiceEvent::ServiceResolved(info)) => {
                                if info.get_property_val_str("id") == Some(core.device().id.as_str()) { continue; }
                                let expected = info.get_property_val_str("fingerprint").map(str::to_owned);
                                if expected.as_ref().is_none_or(|fp| fp.len() != 64) { continue; }
                                for ip in info.get_addresses() {
                                    let address = SocketAddr::new(*ip, info.get_port()).to_string();
                                    if core.connect_expected(&address, expected.clone(), true).await.is_ok() { break; }
                                }
                            }
                            Ok(mdns_sd::ServiceEvent::ServiceRemoved(_, fullname)) => {
                                let mut state = core.state.write().await;
                                state.peers.retain(|id, peer| !peer.discovered || !fullname.starts_with(&id[..16]));
                            }
                            Err(_) => break,
                            _ => {}
                        }
                    }
                }
            }
        });
        Ok(())
    }

    pub async fn confirm_send(&self, id: &str) -> Result<()> {
        let outgoing = self
            .outgoing
            .lock()
            .await
            .get(id)
            .cloned()
            .context("传输已结束")?;
        outgoing.confirmed.store(true, Ordering::SeqCst);
        Ok(())
    }

    pub async fn respond(&self, id: &str, accept: bool) -> Result<()> {
        let mut sessions = self.incoming.lock().await;
        let session = sessions.get_mut(id).context("请求已过期")?;
        if session.cancelled.is_cancelled()
            || session.created.elapsed() >= Duration::from_secs(60)
            || session.approved
        {
            bail!("请求已过期或已处理");
        }
        if !accept {
            session.cancelled.cancel();
            drop(sessions);
            self.finish_task(id, "rejected", None, None, None).await;
            return Ok(());
        }
        if matches!(session.manifest.kind, model::ContentKind::Files) {
            tokio::fs::create_dir(&session.staging).await?;
            for entry in &session.manifest.entries {
                let path = session.staging.join(&entry.path);
                let dir = if entry.directory {
                    path.as_path()
                } else {
                    path.parent().unwrap()
                };
                tokio::fs::create_dir_all(dir).await?;
            }
        }
        session.approved = true;
        drop(sessions);
        self.update_task(id, |task| task.status = "awaiting_sender".into())
            .await;
        Ok(())
    }

    pub async fn cancel(&self, id: &str) -> Result<()> {
        if let Some(outgoing) = self.outgoing.lock().await.get(id).cloned() {
            outgoing.cancelled.cancel();
        }
        let staging = {
            let mut sessions = self.incoming.lock().await;
            sessions.get_mut(id).map(|session| {
                session.cancelled.cancel();
                session.staging.clone()
            })
        };
        self.finish_task(id, "cancelled", None, None, None).await;
        if let Some(path) = staging {
            let _ = tokio::fs::remove_dir_all(path).await;
        }
        Ok(())
    }

    pub async fn dismiss(&self, id: &str) {
        self.state
            .write()
            .await
            .transfers
            .retain(|task| task.id != id || !is_terminal(&task.status));
    }

    async fn update_task(&self, id: &str, update: impl FnOnce(&mut Transfer)) {
        if let Some(task) = self
            .state
            .write()
            .await
            .transfers
            .iter_mut()
            .find(|task| task.id == id)
        {
            if !is_terminal(&task.status) {
                update(task);
            }
        }
    }

    async fn add_task(&self, task: Transfer) {
        let mut state = self.state.write().await;
        if state.transfers.len() >= 20 {
            if let Some(index) = state.transfers.iter().position(|t| is_terminal(&t.status)) {
                state.transfers.remove(index);
            }
        }
        state.transfers.push(task);
    }

    async fn finish_task(
        &self,
        id: &str,
        status: &str,
        error: Option<String>,
        saved: Option<String>,
        text: Option<String>,
    ) {
        let mut state = self.state.write().await;
        let Some(task) = state.transfers.iter_mut().find(|task| task.id == id) else {
            return;
        };
        if is_terminal(&task.status) {
            return;
        }
        task.status = status.into();
        task.error = error;
        task.saved_path = saved;
        task.text = text;
        if status == "completed" {
            task.transferred_bytes = task.total_bytes;
        }
        let history = History {
            id: task.id.clone(),
            direction: task.direction.clone(),
            peer_name: task.peer_name.clone(),
            title: task.title.clone(),
            kind: task.kind.clone(),
            total_bytes: task.total_bytes,
            status: status.into(),
            saved_path: task.saved_path.clone(),
            time: now(),
        };
        if let Err(error) = self.store.save(&history) {
            state.warning = Some(format!("历史记录保存失败：{error}"));
        }
    }

    async fn expire_sessions(&self) {
        let expired: Vec<_> = {
            let sessions = self.incoming.lock().await;
            sessions
                .iter()
                .filter(|(_, session)| {
                    !session.cancelled.is_cancelled()
                        && !session.finished
                        && ((session.created.elapsed() > Duration::from_secs(60)
                            && (!session.approved || !session.sender_confirmed))
                            || session.last_activity.elapsed() > Duration::from_secs(120))
                })
                .map(|(id, _)| id.clone())
                .collect()
        };
        for id in expired {
            let _ = self.cancel(&id).await;
        }
        self.nonces
            .lock()
            .await
            .retain(|_, time| time.elapsed() < Duration::from_secs(600));
        self.incoming.lock().await.retain(|_, session| {
            session.created.elapsed() < Duration::from_secs(600)
                || (!session.cancelled.is_cancelled() && !session.finished)
        });
    }

    pub fn stop(&self) {
        self.shutdown.cancel();
        self.server.graceful_shutdown(Some(Duration::from_secs(1)));
        if let Some(mdns) = &self.mdns {
            let _ = mdns.shutdown();
        }
    }

    pub fn received_path(&self, input: &str) -> Result<PathBuf> {
        let path = Path::new(input)
            .canonicalize()
            .context("文件已移动或删除")?;
        if !path.starts_with(&self.receive_dir) {
            bail!("只能打开 Swoosh 接收目录中的内容");
        }
        Ok(path)
    }
}

fn local_addresses() -> Vec<IpAddr> {
    if_addrs::get_if_addrs()
        .unwrap_or_default()
        .into_iter()
        .filter(|interface| !interface.is_loopback())
        .map(|interface| interface.ip())
        .filter(|ip| {
            ip.is_ipv4() && normalize_address(&SocketAddr::new(*ip, 53318).to_string()).is_ok()
        })
        .collect::<HashSet<_>>()
        .into_iter()
        .collect()
}

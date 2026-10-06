use crate::{
    files::{self, SourcePlan},
    identity,
    model::{
        ContentKind, Direction, Manifest, OfferReply, SessionState, SessionStatus, Transfer,
        TransferStatus, CONFIRM_WINDOW, MAX_ACTIVE_TRANSFERS, PROTOCOL,
    },
    Core, Outcome, Outgoing,
};
use anyhow::{bail, Context, Result};
use bytes::Bytes;
use futures_util::StreamExt;
use std::{
    sync::{
        atomic::{AtomicBool, AtomicU64, Ordering},
        Arc,
    },
    time::{Duration, Instant},
};
use tokio_util::{io::ReaderStream, sync::CancellationToken};
use uuid::Uuid;

impl Core {
    pub async fn send_files(self: &Arc<Self>, peer_id: &str, selection_id: &str) -> Result<String> {
        let plan = self
            .state
            .read()
            .await
            .plans
            .get(selection_id)
            .cloned()
            .context("请重新选择文件")?;
        self.begin_send(peer_id, plan, ContentKind::Files).await
    }

    pub async fn send_text(self: &Arc<Self>, peer_id: &str, text: String) -> Result<String> {
        self.begin_send(peer_id, files::text_plan(text)?, ContentKind::Text)
            .await
    }

    async fn begin_send(
        self: &Arc<Self>,
        peer_id: &str,
        plan: SourcePlan,
        kind: ContentKind,
    ) -> Result<String> {
        let peer = self
            .state
            .read()
            .await
            .peers
            .get(peer_id)
            .cloned()
            .context("设备已离线，请重新连接")?;
        let mut outgoing = self.outgoing.lock().await;
        if outgoing.len() >= MAX_ACTIVE_TRANSFERS {
            bail!("最多同时发送 {MAX_ACTIVE_TRANSFERS} 项内容");
        }
        let id = Uuid::new_v4().to_string();
        let control = Arc::new(Outgoing {
            confirmed: AtomicBool::new(false),
            cancelled: CancellationToken::new(),
        });
        outgoing.insert(id.clone(), control.clone());
        drop(outgoing);
        let manifest = Manifest {
            protocol: PROTOCOL,
            nonce: Uuid::new_v4().to_string(),
            sender: self.device().clone(),
            receiver_fingerprint: peer.device.fingerprint.clone(),
            kind: kind.clone(),
            title: plan.selection.title.clone(),
            entries: plan.selection.entries.clone(),
        };
        crate::validate_manifest(&manifest)?;
        let code = identity::verification_code(&manifest)?;
        self.add_task(Transfer {
            id: id.clone(),
            direction: Direction::Send,
            peer_name: peer.device.name.clone(),
            title: manifest.title.clone(),
            kind,
            status: TransferStatus::Connecting,
            code,
            total_bytes: manifest.total_bytes(),
            transferred_bytes: 0,
            count: manifest.count(),
            error: None,
            saved_path: None,
            text: None,
        })
        .await;
        let core = self.clone();
        let task_id = id.clone();
        tokio::spawn(async move {
            let result = core
                .run_send(&task_id, &peer, manifest, plan, &control)
                .await;
            let (status, outcome) = match result {
                Ok(()) => (TransferStatus::Completed, Outcome::default()),
                Err(error) => {
                    let status = if control.cancelled.is_cancelled() {
                        TransferStatus::Cancelled
                    } else {
                        TransferStatus::Failed
                    };
                    (status, Outcome::error(format!("{error:#}")))
                }
            };
            core.finish_task(&task_id, status, outcome).await;
            core.outgoing.lock().await.remove(&task_id);
        });
        Ok(id)
    }

    async fn run_send(
        self: &Arc<Self>,
        id: &str,
        peer: &crate::Peer,
        manifest: Manifest,
        plan: SourcePlan,
        control: &Outgoing,
    ) -> Result<()> {
        let (client, _) = crate::tls::client(Some(peer.device.fingerprint.clone()))?;
        let base = format!("https://{}/v1", peer.address);
        let offer = self.identity.sign(manifest.clone())?;
        let response = tokio::select! {
            _ = control.cancelled.cancelled() => bail!("传输已取消"),
            response = client.post(format!("{base}/offer")).json(&offer).send() => response?,
        };
        let response = checked(response).await?;
        let reply: OfferReply = response.json().await?;
        if reply.code != identity::verification_code(&manifest)?
            || Uuid::parse_str(&reply.id).is_err()
            || reply.token.len() != 32
        {
            bail!("会话身份核对失败");
        }
        let session_url = format!("{base}/session/{}", reply.id);
        let outcome = self
            .send_session(id, &client, &session_url, &reply.token, plan, control)
            .await;
        if outcome.is_err() {
            // This request is separate from the cancelled data stream so the receiver also stops.
            let _ = tokio::time::timeout(
                Duration::from_secs(3),
                client
                    .post(format!("{session_url}/cancel"))
                    .bearer_auth(&reply.token)
                    .send(),
            )
            .await;
        }
        outcome
    }

    async fn send_session(
        self: &Arc<Self>,
        id: &str,
        client: &reqwest::Client,
        session_url: &str,
        token: &str,
        plan: SourcePlan,
        control: &Outgoing,
    ) -> Result<()> {
        self.set_status(id, TransferStatus::AwaitingConfirmation)
            .await;
        let started = Instant::now();
        let mut confirmed = false;
        loop {
            if started.elapsed() >= CONFIRM_WINDOW {
                bail!("确认超时，请重新发送");
            }
            tokio::select! {
                _ = control.cancelled.cancelled() => bail!("传输已取消"),
                _ = tokio::time::sleep(Duration::from_millis(300)) => {}
            }
            if control.confirmed.load(Ordering::SeqCst) && !confirmed {
                checked(
                    client
                        .post(format!("{session_url}/confirm"))
                        .bearer_auth(token)
                        .send()
                        .await?,
                )
                .await?;
                confirmed = true;
                self.set_status(id, TransferStatus::AwaitingReceiver).await;
            }
            let response = tokio::select! {
                _ = control.cancelled.cancelled() => bail!("传输已取消"),
                response = client.get(session_url).bearer_auth(token).send() => response?,
            };
            let status: SessionStatus = checked(response).await?.json().await?;
            match status.status {
                SessionState::Ready if confirmed => break,
                SessionState::Cancelled => bail!("对方拒绝或取消了传输"),
                SessionState::Waiting => {}
                _ => bail!("接收端会话状态异常"),
            }
        }
        self.set_status(id, TransferStatus::Transferring).await;
        let mut completed = 0u64;
        for (index, entry) in plan.selection.entries.iter().enumerate() {
            if entry.directory {
                continue;
            }
            let progress = Arc::new(AtomicU64::new(0));
            let body = if let Some(text) = &plan.text {
                reqwest::Body::from(Bytes::copy_from_slice(text))
            } else {
                let path = plan.sources.get(&index).context("文件来源已失效")?;
                let file = tokio::fs::File::open(path)
                    .await
                    .context("源文件已移动或无法读取")?;
                if file.metadata().await?.len() != entry.size {
                    bail!("源文件已变化，请重新选择");
                }
                let count = progress.clone();
                let stream = ReaderStream::with_capacity(file, 65536).map(move |chunk| {
                    if let Ok(bytes) = &chunk {
                        count.fetch_add(bytes.len() as u64, Ordering::Relaxed);
                    }
                    chunk
                });
                reqwest::Body::wrap_stream(stream)
            };
            let request = client
                .put(format!("{session_url}/file/{index}"))
                .bearer_auth(token)
                .header(reqwest::header::CONTENT_LENGTH, entry.size)
                .body(body)
                .send();
            tokio::pin!(request);
            let response = loop {
                tokio::select! {
                    _ = control.cancelled.cancelled() => bail!("传输已取消"),
                    response = &mut request => break response?,
                    _ = tokio::time::sleep(Duration::from_millis(150)) => {
                        let current = progress.load(Ordering::Relaxed).min(entry.size);
                        self.set_progress(id, completed + current).await;
                    }
                }
            };
            checked(response).await?;
            completed += entry.size;
            self.set_progress(id, completed).await;
        }
        self.set_status(id, TransferStatus::Verifying).await;

        let response = tokio::select! {
            _ = control.cancelled.cancelled() => bail!("传输已取消"),
            response = client.post(format!("{session_url}/finish")).bearer_auth(token).send() => response?,
        };
        checked(response).await?;
        Ok(())
    }
}

async fn checked(response: reqwest::Response) -> Result<reqwest::Response> {
    if !response.status().is_success() {
        let status = response.status();
        let message = response.text().await.unwrap_or_default();
        bail!(
            "接收端返回 {status}：{}",
            message.chars().take(256).collect::<String>()
        );
    }
    Ok(response)
}

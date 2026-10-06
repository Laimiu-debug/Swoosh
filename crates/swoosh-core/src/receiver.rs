use crate::{
    identity,
    model::{
        ContentKind, Direction, Entry, OfferReply, SessionState, SessionStatus, SignedOffer,
        Transfer, TransferStatus, MAX_ACTIVE_TRANSFERS,
    },
    Core, Incoming, Outcome,
};
use anyhow::{bail, Context, Result};
use axum::{
    body::Body,
    extract::{DefaultBodyLimit, Path, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post, put},
    Json, Router,
};
use futures_util::StreamExt;
use sha2::{Digest, Sha256};
use std::{
    collections::HashSet,
    path::PathBuf,
    sync::Arc,
    time::{Duration, Instant},
};
use tokio::io::AsyncWriteExt;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

struct ApiError(StatusCode, String);

impl ApiError {
    fn new(status: StatusCode, message: &str) -> Self {
        Self(status, message.into())
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (self.0, self.1).into_response()
    }
}

impl From<anyhow::Error> for ApiError {
    fn from(error: anyhow::Error) -> Self {
        Self(StatusCode::BAD_REQUEST, error.to_string())
    }
}

pub(crate) fn router(core: Arc<Core>) -> Router {
    Router::new()
        .route("/v1/info", get(info))
        .route(
            "/v1/offer",
            post(offer).layer(DefaultBodyLimit::max(2 * 1024 * 1024)),
        )
        .route("/v1/session/{id}", get(status))
        .route("/v1/session/{id}/confirm", post(confirm))
        .route("/v1/session/{id}/cancel", post(cancel))
        .route(
            "/v1/session/{id}/file/{index}",
            put(upload).layer(DefaultBodyLimit::disable()),
        )
        .route("/v1/session/{id}/finish", post(finish))
        .with_state(core)
}

async fn info(State(core): State<Arc<Core>>) -> Json<crate::Device> {
    Json(core.device().clone())
}

async fn offer(
    State(core): State<Arc<Core>>,
    Json(offer): Json<SignedOffer>,
) -> Result<Json<OfferReply>, ApiError> {
    identity::verify_offer(&offer, &core.device().fingerprint)?;
    crate::validate_manifest(&offer.manifest)?;
    if offer.manifest.sender.id == core.device().id {
        return Err(ApiError::new(StatusCode::BAD_REQUEST, "不能向自己发送"));
    }
    let mut sessions = core.incoming.lock().await;
    if sessions
        .values()
        .filter(|session| session.is_open())
        .count()
        >= MAX_ACTIVE_TRANSFERS
    {
        return Err(ApiError::new(
            StatusCode::TOO_MANY_REQUESTS,
            "接收队列已满，请稍后重试",
        ));
    }
    let mut nonces = core.nonces.lock().await;
    if nonces.len() >= 8192
        || nonces
            .insert(offer.manifest.nonce.clone(), Instant::now())
            .is_some()
    {
        return Err(ApiError::new(StatusCode::CONFLICT, "重复或过期的发送请求"));
    }
    drop(nonces);
    let id = Uuid::new_v4().to_string();
    let token = Uuid::new_v4().simple().to_string();
    let code = identity::verification_code(&offer.manifest)?;
    // Pin the destination to this request, including when settings change before confirmation.
    let receive_dir = core.receive_dir();
    core.add_task(Transfer {
        id: id.clone(),
        direction: Direction::Receive,
        peer_name: offer.manifest.sender.name.clone(),
        title: offer.manifest.title.clone(),
        kind: offer.manifest.kind.clone(),
        status: TransferStatus::AwaitingConfirmation,
        code: code.clone(),
        total_bytes: offer.manifest.total_bytes(),
        transferred_bytes: 0,
        count: offer.manifest.count(),
        error: None,
        saved_path: None,
        text: None,
    })
    .await;
    sessions.insert(
        id.clone(),
        Incoming {
            manifest: offer.manifest,
            token: token.clone(),
            approved: false,
            sender_confirmed: false,
            cancelled: CancellationToken::new(),
            completed: HashSet::new(),
            active: HashSet::new(),
            staging: receive_dir.join(format!(".swoosh-{id}.partial")),
            receive_dir,
            text: None,
            finished: false,
            created: Instant::now(),
            last_activity: Instant::now(),
        },
    );
    Ok(Json(OfferReply { id, token, code }))
}

fn authorize(session: &Incoming, headers: &HeaderMap) -> Result<(), ApiError> {
    let token = headers
        .get("authorization")
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "));
    if token != Some(session.token.as_str()) {
        return Err(ApiError::new(StatusCode::UNAUTHORIZED, "会话授权无效"));
    }
    Ok(())
}

async fn status(
    State(core): State<Arc<Core>>,
    Path(id): Path<String>,
    headers: HeaderMap,
) -> Result<Json<SessionStatus>, ApiError> {
    let sessions = core.incoming.lock().await;
    let session = sessions.get(&id).context("会话已过期")?;
    authorize(session, &headers)?;
    let status = if session.cancelled.is_cancelled() {
        SessionState::Cancelled
    } else if session.finished {
        SessionState::Completed
    } else if session.approved && session.sender_confirmed {
        SessionState::Ready
    } else {
        SessionState::Waiting
    };
    Ok(Json(SessionStatus { status }))
}

async fn confirm(
    State(core): State<Arc<Core>>,
    Path(id): Path<String>,
    headers: HeaderMap,
) -> Result<StatusCode, ApiError> {
    let mut sessions = core.incoming.lock().await;
    let session = sessions.get_mut(&id).context("会话已过期")?;
    authorize(session, &headers)?;
    if !session.is_open() || !session.confirm_window_open() {
        return Err(ApiError::new(StatusCode::CONFLICT, "会话已结束"));
    }
    session.sender_confirmed = true;
    session.last_activity = Instant::now();
    Ok(StatusCode::NO_CONTENT)
}

async fn cancel(
    State(core): State<Arc<Core>>,
    Path(id): Path<String>,
    headers: HeaderMap,
) -> Result<StatusCode, ApiError> {
    {
        let sessions = core.incoming.lock().await;
        let session = sessions.get(&id).context("会话已过期")?;
        authorize(session, &headers)?;
        if session.finished {
            return Ok(StatusCode::NO_CONTENT);
        }
    }
    core.cancel(&id).await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn upload(
    State(core): State<Arc<Core>>,
    Path((id, index)): Path<(String, usize)>,
    headers: HeaderMap,
    body: Body,
) -> Result<StatusCode, ApiError> {
    let (entry, path, kind, cancelled) = {
        let mut sessions = core.incoming.lock().await;
        let session = sessions.get_mut(&id).context("会话已过期")?;
        authorize(session, &headers)?;
        if !session.approved || !session.sender_confirmed || !session.is_open() {
            return Err(ApiError::new(
                StatusCode::FORBIDDEN,
                "双方尚未确认或传输已结束",
            ));
        }
        let entry = session
            .manifest
            .entries
            .get(index)
            .context("文件索引无效")?
            .clone();
        if entry.directory || session.completed.contains(&index) || session.active.contains(&index)
        {
            return Err(ApiError::new(StatusCode::CONFLICT, "文件已处理"));
        }
        if !session.active.is_empty() {
            return Err(ApiError::new(StatusCode::CONFLICT, "请顺序传输文件"));
        }
        let length = headers
            .get("content-length")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.parse::<u64>().ok());
        if length != Some(entry.size) {
            return Err(ApiError::new(StatusCode::BAD_REQUEST, "文件大小不匹配"));
        }
        session.active.insert(index);
        session.last_activity = Instant::now();
        let path = session.staging.join(&entry.path);
        (
            entry,
            path,
            session.manifest.kind.clone(),
            session.cancelled.clone(),
        )
    };
    core.set_status(&id, TransferStatus::Transferring).await;
    let destination = match kind {
        ContentKind::Files => Sink::File(path),
        ContentKind::Text => Sink::Memory,
    };
    match receive_entry(&core, &id, &entry, destination, body, &cancelled).await {
        Ok(text) => {
            let mut sessions = core.incoming.lock().await;
            let session = sessions.get_mut(&id).context("会话已过期")?;
            if session.cancelled.is_cancelled() {
                return Err(ApiError::new(StatusCode::CONFLICT, "传输已取消"));
            }
            session.active.remove(&index);
            session.completed.insert(index);
            session.last_activity = Instant::now();
            session.text = text;
            let completed = session.completed_bytes();
            drop(sessions);
            core.set_progress(&id, completed).await;
            Ok(StatusCode::NO_CONTENT)
        }
        Err(error) => {
            cancelled.cancel();
            let staging = core
                .incoming
                .lock()
                .await
                .get(&id)
                .map(|session| session.staging.clone());
            core.finish_task(&id, TransferStatus::Failed, Outcome::error(&error))
                .await;
            if let Some(staging) = staging {
                let _ = tokio::fs::remove_dir_all(staging).await;
            }
            Err(error.into())
        }
    }
}

/// Where an uploaded entry is written: a staged file, or memory for text messages.
enum Sink {
    File(PathBuf),
    Memory,
}

/// Streams one entry from the request body, verifying its size and digest.
/// Returns the decoded text for text messages.
async fn receive_entry(
    core: &Core,
    id: &str,
    entry: &Entry,
    sink: Sink,
    body: Body,
    cancelled: &CancellationToken,
) -> Result<Option<String>> {
    let mut file = match &sink {
        Sink::File(path) => Some(
            tokio::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(path)
                .await?,
        ),
        Sink::Memory => None,
    };
    let mut text = Vec::new();
    let mut hash = Sha256::new();
    let mut received = 0u64;
    let mut stream = body.into_data_stream();
    let mut last_progress = Instant::now();
    loop {
        let next = tokio::time::timeout(Duration::from_secs(60), stream.next());
        let chunk = tokio::select! {
            _ = cancelled.cancelled() => bail!("传输已取消"),
            chunk = next => chunk.context("传输超时")?,
        };
        let Some(chunk) = chunk else { break };
        let chunk = chunk?;
        received = received
            .checked_add(chunk.len() as u64)
            .context("文件大小溢出")?;
        if received > entry.size {
            bail!("收到的数据超过清单大小");
        }
        hash.update(&chunk);
        match &mut file {
            Some(file) => file.write_all(&chunk).await?,
            None => text.extend_from_slice(&chunk),
        }
        if last_progress.elapsed() >= Duration::from_millis(100) {
            let completed = {
                let mut sessions = core.incoming.lock().await;
                let session = sessions.get_mut(id).context("会话已过期")?;
                session.last_activity = Instant::now();
                session.completed_bytes()
            };
            core.set_progress(id, completed + received).await;
            last_progress = Instant::now();
        }
    }
    if received != entry.size || hex::encode(hash.finalize()) != entry.sha256 {
        bail!("文件校验失败，请重新发送");
    }
    match file {
        Some(mut file) => {
            file.flush().await?;
            file.sync_all().await?;
            Ok(None)
        }
        None => Ok(Some(
            String::from_utf8(text).context("文字不是有效的 UTF-8")?,
        )),
    }
}

async fn finish(
    State(core): State<Arc<Core>>,
    Path(id): Path<String>,
    headers: HeaderMap,
) -> Result<StatusCode, ApiError> {
    let mut sessions = core.incoming.lock().await;
    let session = sessions.get_mut(&id).context("会话已过期")?;
    authorize(session, &headers)?;
    if session.finished {
        return Ok(StatusCode::NO_CONTENT);
    }
    if session.cancelled.is_cancelled()
        || !session.approved
        || !session.sender_confirmed
        || !session.active.is_empty()
        || session.completed.len() != session.manifest.count()
    {
        return Err(ApiError::new(StatusCode::CONFLICT, "内容尚未完整接收"));
    }
    core.set_status(&id, TransferStatus::Verifying).await;
    let saved_path = if matches!(session.manifest.kind, ContentKind::Files) {
        let destination = session
            .receive_dir
            .join(format!("接收-{}-{}", crate::now(), &id[..8]));
        tokio::fs::rename(&session.staging, &destination)
            .await
            .map_err(anyhow::Error::from)?;
        Some(destination.to_string_lossy().into_owned())
    } else {
        None
    };
    session.finished = true;
    let text = session.text.take();
    drop(sessions);
    core.finish_task(
        &id,
        TransferStatus::Completed,
        Outcome {
            saved_path,
            text,
            ..Outcome::default()
        },
    )
    .await;
    Ok(StatusCode::NO_CONTENT)
}

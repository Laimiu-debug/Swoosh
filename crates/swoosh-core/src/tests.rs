use super::*;
use model::{ContentKind, Entry, OfferReply, PROTOCOL};
use tempfile::TempDir;

async fn device(name: &str) -> (TempDir, Arc<Core>) {
    let dir = TempDir::new().unwrap();
    let core = Core::start(Config {
        data_dir: dir.path().join("state"),
        receive_dir: dir.path().join("received"),
        name: Some(name.into()),
        port: 0,
        discovery: false,
        listen_ip: Ipv4Addr::LOCALHOST,
    })
    .await
    .unwrap();
    (dir, core)
}

async fn pair(sender: &Arc<Core>, receiver: &Arc<Core>) -> Peer {
    sender
        .connect(&format!("127.0.0.1:{}", receiver.port()))
        .await
        .unwrap()
}

async fn wait_task(core: &Core, id: &str, status: &str) -> Transfer {
    tokio::time::timeout(Duration::from_secs(15), async {
        loop {
            let snapshot = core.snapshot().await.unwrap();
            if let Some(task) = snapshot.transfers.iter().find(|task| task.id == id) {
                if task.status == status {
                    return task.clone();
                }
                if is_terminal(&task.status) && !is_terminal(status) {
                    panic!("Unexpected task result: {task:?}");
                }
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    })
    .await
    .unwrap()
}

async fn accept_both(sender: &Arc<Core>, receiver: &Arc<Core>, id: &str) -> String {
    let outgoing = wait_task(sender, id, "awaiting_confirmation").await;
    let incoming = receiver
        .snapshot()
        .await
        .unwrap()
        .transfers
        .into_iter()
        .find(|t| t.status == "awaiting_confirmation")
        .unwrap();
    assert_eq!(outgoing.code, incoming.code);
    receiver.respond(&incoming.id, true).await.unwrap();
    // Receiver confirmation alone must not release the stream.
    tokio::time::sleep(Duration::from_millis(400)).await;
    assert_eq!(
        receiver
            .snapshot()
            .await
            .unwrap()
            .transfers
            .iter()
            .find(|t| t.id == incoming.id)
            .unwrap()
            .transferred_bytes,
        0
    );
    sender.confirm_send(id).await.unwrap();
    incoming.id
}

#[tokio::test]
async fn text_roundtrip_requires_both_confirmations_and_history_omits_text() {
    let (_a, sender) = device("发送电脑").await;
    let (_b, receiver) = device("接收电脑").await;
    let peer = pair(&sender, &receiver).await;
    let text = "嗖！✈️\n中文、换行和 emoji 🥝";
    let id = sender
        .send_text(&peer.device.id, text.into())
        .await
        .unwrap();
    let incoming = accept_both(&sender, &receiver, &id).await;
    wait_task(&sender, &id, "completed").await;
    let result = wait_task(&receiver, &incoming, "completed").await;
    assert_eq!(result.text.as_deref(), Some(text));
    let history = receiver.snapshot().await.unwrap().history;
    assert_eq!(history.len(), 1);
    assert!(!serde_json::to_string(&history).unwrap().contains(text));
    sender.stop();
    receiver.stop();
}

#[tokio::test]
async fn folder_roundtrip_preserves_unicode_zero_bytes_empty_directories_and_collisions() {
    let (source, sender) = device("A").await;
    let (_target, receiver) = device("B").await;
    let root = source.path().join("春游照片");
    std::fs::create_dir_all(root.join("空文件夹")).unwrap();
    std::fs::write(root.join("零字节.txt"), []).unwrap();
    let bytes: Vec<u8> = (0..2 * 1024 * 1024).map(|i| (i % 251) as u8).collect();
    std::fs::write(root.join("原图.bin"), &bytes).unwrap();
    let selection = sender.select(vec![root]).await.unwrap();
    let peer = pair(&sender, &receiver).await;
    let mut destinations = Vec::new();
    for _ in 0..2 {
        let id = sender
            .send_files(&peer.device.id, &selection.id)
            .await
            .unwrap();
        let incoming = accept_both(&sender, &receiver, &id).await;
        wait_task(&sender, &id, "completed").await;
        let received = wait_task(&receiver, &incoming, "completed").await;
        let destination = PathBuf::from(received.saved_path.unwrap());
        assert!(destination.join("春游照片/空文件夹").is_dir());
        assert_eq!(
            std::fs::read(destination.join("春游照片/零字节.txt")).unwrap(),
            Vec::<u8>::new()
        );
        assert_eq!(
            std::fs::read(destination.join("春游照片/原图.bin")).unwrap(),
            bytes
        );
        destinations.push(destination);
    }
    assert_ne!(destinations[0], destinations[1]);
    sender.stop();
    receiver.stop();
}

#[tokio::test]
async fn rejection_and_cancellation_stop_both_sides_without_publishing() {
    let (_source, sender) = device("A").await;
    let (_target, receiver) = device("B").await;
    let peer = pair(&sender, &receiver).await;
    for accept in [false, true] {
        let id = sender
            .send_text(&peer.device.id, "测试".into())
            .await
            .unwrap();
        wait_task(&sender, &id, "awaiting_confirmation").await;
        let incoming = receiver
            .snapshot()
            .await
            .unwrap()
            .transfers
            .into_iter()
            .find(|t| t.status == "awaiting_confirmation")
            .unwrap();
        receiver.respond(&incoming.id, accept).await.unwrap();
        if accept {
            sender.cancel(&id).await.unwrap();
        } else {
            sender.confirm_send(&id).await.unwrap();
        }
        wait_task(&sender, &id, if accept { "cancelled" } else { "failed" }).await;
        if accept {
            wait_task(&receiver, &incoming.id, "cancelled").await;
        }
    }
    assert_eq!(std::fs::read_dir(&receiver.receive_dir).unwrap().count(), 0);
    sender.stop();
    receiver.stop();
}

#[tokio::test]
async fn changed_source_fails_integrity_check_and_removes_partial_content() {
    let (source, sender) = device("A").await;
    let (_target, receiver) = device("B").await;
    let file = source.path().join("mutable.txt");
    std::fs::write(&file, b"original").unwrap();
    let selection = sender.select(vec![file.clone()]).await.unwrap();
    std::fs::write(file, b"modified").unwrap();
    let peer = pair(&sender, &receiver).await;
    let id = sender
        .send_files(&peer.device.id, &selection.id)
        .await
        .unwrap();
    let incoming = accept_both(&sender, &receiver, &id).await;
    wait_task(&sender, &id, "failed").await;
    wait_task(&receiver, &incoming, "failed").await;
    assert_eq!(std::fs::read_dir(&receiver.receive_dir).unwrap().count(), 0);
    sender.stop();
    receiver.stop();
}

#[tokio::test]
async fn protocol_rejects_unauthorized_streams_replays_tampering_and_changed_certificates() {
    let (_source, sender) = device("A").await;
    let (_target, receiver) = device("B").await;
    let peer = pair(&sender, &receiver).await;
    let (client, _) = tls::client(Some(peer.device.fingerprint.clone())).unwrap();
    let base = format!("https://{}", peer.address);
    let manifest = Manifest {
        protocol: PROTOCOL,
        nonce: uuid::Uuid::new_v4().to_string(),
        sender: sender.device().clone(),
        receiver_fingerprint: receiver.device().fingerprint.clone(),
        kind: ContentKind::Text,
        title: "文字".into(),
        entries: vec![Entry {
            path: "message.txt".into(),
            size: 4,
            sha256: identity::digest(b"test"),
            directory: false,
        }],
    };
    let offer = sender.identity.sign(manifest).unwrap();
    let mut tampered = offer.clone();
    tampered.manifest.title = "tampered".into();
    assert_eq!(
        client
            .post(format!("{base}/v1/offer"))
            .json(&tampered)
            .send()
            .await
            .unwrap()
            .status(),
        400
    );
    let reply: OfferReply = client
        .post(format!("{base}/v1/offer"))
        .json(&offer)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(
        client
            .post(format!("{base}/v1/offer"))
            .json(&offer)
            .send()
            .await
            .unwrap()
            .status(),
        409
    );
    let url = format!("{base}/v1/session/{}", reply.id);
    assert_eq!(
        client
            .post(format!("{url}/confirm"))
            .bearer_auth("wrong")
            .send()
            .await
            .unwrap()
            .status(),
        401
    );
    assert_eq!(
        client
            .put(format!("{url}/file/0"))
            .bearer_auth(&reply.token)
            .body("test")
            .send()
            .await
            .unwrap()
            .status(),
        403
    );
    assert_eq!(std::fs::read_dir(&receiver.receive_dir).unwrap().count(), 0);
    let (wrong, _) = tls::client(Some("0".repeat(64))).unwrap();
    assert!(wrong.get(format!("{base}/v1/info")).send().await.is_err());
    sender.stop();
    receiver.stop();
}

#[test]
fn path_validation_covers_windows_reserved_names_and_traversal() {
    for path in [
        "../file",
        "/file",
        "a/../../b",
        "C:/file",
        "a\\b",
        "a//b",
        "a/./b",
        "a/CON.txt",
        "LPT1",
        "file.",
        "file ",
        "a/x:y",
        "a\0b",
    ] {
        assert!(validate_relative(path).is_err(), "accepted {path:?}");
    }
    for path in [
        "春游照片/照片 1.png",
        "a/empty",
        ".config/file",
        "COM10.txt",
    ] {
        validate_relative(path).unwrap();
    }
    assert!(normalize_address("https://example.com:53318").is_err());
    assert!(normalize_address("8.8.8.8:53318").is_err());
    assert_eq!(
        normalize_address("192.168.1.8").unwrap(),
        "192.168.1.8:53318"
    );
}

#[tokio::test]
async fn identity_and_history_survive_restart() {
    let (dir, first) = device("A").await;
    let id = first.device().clone();
    first
        .store
        .save(&History {
            id: "test".into(),
            direction: "receive".into(),
            peer_name: "B".into(),
            title: "文件".into(),
            kind: ContentKind::Files,
            total_bytes: 12,
            status: "completed".into(),
            saved_path: None,
            time: now(),
        })
        .unwrap();
    first.stop();
    let second = Core::start(Config {
        data_dir: dir.path().join("state"),
        receive_dir: dir.path().join("received"),
        name: Some("A".into()),
        port: 0,
        discovery: false,
        listen_ip: Ipv4Addr::LOCALHOST,
    })
    .await
    .unwrap();
    assert_eq!(second.device().id, id.id);
    assert_eq!(second.device().fingerprint, id.fingerprint);
    assert_eq!(second.snapshot().await.unwrap().history.len(), 1);
    second.clear_history().unwrap();
    assert!(second.snapshot().await.unwrap().history.is_empty());
    second.stop();
}

//! Development fixture for exercising the native UI against a real TLS receiver.
//! It binds only to localhost, automatically confirms test transfers, and is not
//! part of the desktop application or installer. Never bind this fixture to LAN.
use std::{collections::HashSet, net::Ipv4Addr, path::PathBuf, time::Duration};
use swoosh_core::{Config, Core};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let root = PathBuf::from(
        std::env::args()
            .nth(1)
            .unwrap_or_else(|| ".local/loopback-receiver".into()),
    );
    let core = Core::start(Config {
        data_dir: root.join("state"),
        receive_dir: root.join("received"),
        name: Some("Swoosh 回环测试设备".into()),
        port: 53319,
        discovery: false,
        listen_ip: Ipv4Addr::LOCALHOST,
    })
    .await?;
    println!("Local test receiver: 127.0.0.1:{}", core.port());
    let mut reported = HashSet::new();
    let started = std::time::Instant::now();
    while started.elapsed() < Duration::from_secs(600) {
        for transfer in core.snapshot().await?.transfers {
            if transfer.status == "awaiting_confirmation" {
                println!("Test confirmation: {}", transfer.code);
                core.respond(&transfer.id, true).await?;
            }
            if transfer.status == "completed" && reported.insert(transfer.id) {
                println!(
                    "Test transfer complete: {} ({} bytes)",
                    transfer.title, transfer.total_bytes
                );
            }
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    core.stop();
    Ok(())
}

use std::{path::PathBuf, sync::Arc};
use swoosh_core::{Config, Core, Peer, Selection, Snapshot, DEFAULT_PORT, TEXT_LIMIT};
use tauri::{Manager, State};
use tauri_plugin_clipboard_manager::ClipboardExt;
use tauri_plugin_dialog::DialogExt;
use tauri_plugin_opener::OpenerExt;

type AppCore = Arc<Core>;
type CommandResult<T> = Result<T, String>;
fn message(error: impl std::fmt::Display) -> String {
    error.to_string()
}

#[tauri::command]
async fn snapshot(core: State<'_, AppCore>) -> CommandResult<Snapshot> {
    core.snapshot().await.map_err(message)
}

#[tauri::command]
async fn choose_files(
    app: tauri::AppHandle,
    core: State<'_, AppCore>,
    folder: bool,
) -> CommandResult<Option<Selection>> {
    let paths = tauri::async_runtime::spawn_blocking(move || {
        let dialog = app.dialog().file();
        if folder {
            dialog.blocking_pick_folder().map(|path| vec![path])
        } else {
            dialog.blocking_pick_files()
        }
    })
    .await
    .map_err(message)?;
    let Some(paths) = paths else {
        return Ok(None);
    };
    let paths = paths
        .into_iter()
        .map(|path| path.into_path().map_err(message))
        .collect::<CommandResult<Vec<_>>>()?;
    core.select(paths).await.map(Some).map_err(message)
}

#[tauri::command]
async fn select_dropped(core: State<'_, AppCore>, paths: Vec<PathBuf>) -> CommandResult<Selection> {
    core.select(paths).await.map_err(message)
}

#[tauri::command]
async fn clear_selection(core: State<'_, AppCore>) -> CommandResult<()> {
    core.clear_selection().await;
    Ok(())
}

#[tauri::command]
async fn choose_receive_dir(
    app: tauri::AppHandle,
    core: State<'_, AppCore>,
) -> CommandResult<Option<String>> {
    let current = core.receive_dir();
    let chosen = tauri::async_runtime::spawn_blocking(move || {
        app.dialog()
            .file()
            .set_title("选择接收文件夹")
            .set_directory(current)
            .blocking_pick_folder()
    })
    .await
    .map_err(message)?;
    let Some(chosen) = chosen else {
        return Ok(None);
    };
    let path = chosen.into_path().map_err(message)?;
    let core = core.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        core.set_receive_dir(path)
            .map(|path| Some(path.to_string_lossy().into_owned()))
            .map_err(message)
    })
    .await
    .map_err(message)?
}

#[tauri::command]
async fn reset_receive_dir(core: State<'_, AppCore>) -> CommandResult<String> {
    let core = core.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        core.reset_receive_dir()
            .map(|path| path.to_string_lossy().into_owned())
            .map_err(message)
    })
    .await
    .map_err(message)?
}

#[tauri::command]
async fn connect_device(core: State<'_, AppCore>, address: String) -> CommandResult<Peer> {
    core.connect(&address).await.map_err(message)
}

#[tauri::command]
async fn send_files(
    core: State<'_, AppCore>,
    peer_id: String,
    selection_id: String,
) -> CommandResult<String> {
    core.inner()
        .send_files(&peer_id, &selection_id)
        .await
        .map_err(message)
}

#[tauri::command]
async fn send_text(
    core: State<'_, AppCore>,
    peer_id: String,
    text: String,
) -> CommandResult<String> {
    core.inner()
        .send_text(&peer_id, text)
        .await
        .map_err(message)
}

#[tauri::command]
async fn confirm_send(core: State<'_, AppCore>, id: String) -> CommandResult<()> {
    core.confirm_send(&id).await.map_err(message)
}

#[tauri::command]
async fn respond_transfer(core: State<'_, AppCore>, id: String, accept: bool) -> CommandResult<()> {
    core.respond(&id, accept).await.map_err(message)
}

#[tauri::command]
async fn cancel_transfer(core: State<'_, AppCore>, id: String) -> CommandResult<()> {
    core.cancel(&id).await.map_err(message)
}

#[tauri::command]
async fn dismiss_transfer(core: State<'_, AppCore>, id: String) -> CommandResult<()> {
    core.dismiss(&id).await;
    Ok(())
}

#[tauri::command]
fn clear_history(core: State<'_, AppCore>) -> CommandResult<()> {
    core.clear_history().map_err(message)
}

#[tauri::command]
fn copy_text(app: tauri::AppHandle, text: String) -> CommandResult<()> {
    if text.len() as u64 > TEXT_LIMIT {
        return Err("文字超过 1 MiB".into());
    }
    app.clipboard().write_text(text).map_err(message)
}

#[tauri::command]
async fn open_received(
    app: tauri::AppHandle,
    core: State<'_, AppCore>,
    path: String,
) -> CommandResult<()> {
    let safe_path = core.received_path(&path).await.map_err(message)?;
    app.opener()
        .open_path(safe_path.to_string_lossy(), None::<&str>)
        .map_err(message)
}

pub fn run() {
    let app = tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let data_dir = app.path().app_local_data_dir()?;
            let receive_dir = app.path().download_dir()?.join("Swoosh");
            let core = tauri::async_runtime::block_on(Core::start(Config {
                data_dir,
                receive_dir,
                port: DEFAULT_PORT,
                name: None,
                discovery: true,
                listen_ip: std::net::Ipv4Addr::UNSPECIFIED,
            }))?;
            app.manage(core);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            snapshot,
            choose_files,
            select_dropped,
            clear_selection,
            choose_receive_dir,
            reset_receive_dir,
            connect_device,
            send_files,
            send_text,
            confirm_send,
            respond_transfer,
            cancel_transfer,
            dismiss_transfer,
            clear_history,
            copy_text,
            open_received
        ])
        .build(tauri::generate_context!())
        .expect("Swoosh 启动失败");
    app.run(|app, event| {
        if matches!(event, tauri::RunEvent::Exit) {
            app.state::<AppCore>().stop();
        }
    });
}

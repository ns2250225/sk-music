mod app;
mod db;
mod models;
mod services;

use app::AppState;
use models::*;
use services::library;
use std::path::PathBuf;
use tauri::{Emitter, Manager};

#[tauri::command]
async fn bootstrap(state: tauri::State<'_, AppState>) -> Result<Bootstrap, String> {
    state.bootstrap().await
}

#[tauri::command]
async fn search_tracks(
    query: String,
    refresh: bool,
    state: tauri::State<'_, AppState>,
    app: tauri::AppHandle,
) -> Result<Vec<Track>, String> {
    let tracks = state.search(&query, refresh).await?;
    let _ = app.emit("search://updated", &tracks);
    Ok(tracks)
}
#[tauri::command]
async fn play_track(
    track_id: String,
    source_id: Option<String>,
    state: tauri::State<'_, AppState>,
    app: tauri::AppHandle,
) -> Result<CommandStatus, String> {
    state.play(&track_id, source_id, app).await
}
#[tauri::command]
async fn pause(state: tauri::State<'_, AppState>) -> Result<(), String> {
    state.player_command("pause", serde_json::json!(true)).await
}
#[tauri::command]
async fn resume(state: tauri::State<'_, AppState>) -> Result<(), String> {
    state
        .player_command("pause", serde_json::json!(false))
        .await
}
#[tauri::command]
async fn seek(position: f64, state: tauri::State<'_, AppState>) -> Result<(), String> {
    state
        .player_command("seek", serde_json::json!(position))
        .await
}
#[tauri::command]
async fn set_volume(volume: f64, state: tauri::State<'_, AppState>) -> Result<(), String> {
    state
        .player_command("volume", serde_json::json!(volume))
        .await
}
#[tauri::command]
async fn next(state: tauri::State<'_, AppState>, app: tauri::AppHandle) -> Result<(), String> {
    state.next(app).await
}
#[tauri::command]
async fn previous(state: tauri::State<'_, AppState>) -> Result<(), String> {
    state.player_command("seek", serde_json::json!(0)).await
}
#[tauri::command]
fn add_to_queue(
    track_id: String,
    state: tauri::State<'_, AppState>,
    app: tauri::AppHandle,
) -> Result<(), String> {
    state.add_queue(&track_id, app)
}
#[tauri::command]
fn remove_from_queue(
    index: usize,
    state: tauri::State<'_, AppState>,
    app: tauri::AppHandle,
) -> Result<(), String> {
    state.remove_queue(index, app)
}
#[tauri::command]
fn clear_queue(state: tauri::State<'_, AppState>, app: tauri::AppHandle) -> Result<(), String> {
    state.clear_queue(app)
}
#[tauri::command]
fn favorite_track(
    track_id: String,
    favorite: bool,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    state.favorite(&track_id, favorite)
}
#[tauri::command]
async fn download_track(
    track_id: String,
    preference: String,
    state: tauri::State<'_, AppState>,
    app: tauri::AppHandle,
) -> Result<(), String> {
    state.download(&track_id, &preference, app).await
}
#[tauri::command]
fn cancel_download(id: String, state: tauri::State<'_, AppState>) -> Result<(), String> {
    state.cancel_download(&id)
}
#[tauri::command]
fn get_downloads(state: tauri::State<'_, AppState>) -> Result<Vec<DownloadItem>, String> {
    state.get_downloads()
}
#[tauri::command]
async fn scan_library(
    path: Option<String>,
    state: tauri::State<'_, AppState>,
) -> Result<Vec<Track>, String> {
    let root = path
        .map(PathBuf::from)
        .unwrap_or_else(|| state.paths.downloads.clone());
    let tracks = library::scan(&root, state.db.clone()).await?;
    state.remember_tracks(&tracks);
    Ok(tracks)
}
#[tauri::command]
fn update_settings(settings: Settings, state: tauri::State<'_, AppState>) -> Result<(), String> {
    state.update_settings(settings)
}
#[tauri::command]
fn get_settings(state: tauri::State<'_, AppState>) -> Settings {
    state.settings.read().clone()
}
#[tauri::command]
async fn reconnect(state: tauri::State<'_, AppState>) -> Result<String, String> {
    Ok(state.reconnect().await)
}
#[tauri::command]
fn clear_cache(state: tauri::State<'_, AppState>) -> Result<u64, String> {
    state.clear_cache()
}
#[tauri::command]
fn complete_onboarding(
    accepted_copyright: bool,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    if !accepted_copyright {
        return Err("必须确认版权提示".into());
    }
    state.db.set_setting("onboarded", "true")
}

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let state = AppState::new(app.handle())?;
            state.monitor_connection(app.handle().clone());
            app.manage(state);
            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { .. } = event {
                window.state::<AppState>().stop_player();
            }
        })
        .invoke_handler(tauri::generate_handler![
            bootstrap,
            search_tracks,
            play_track,
            pause,
            resume,
            seek,
            set_volume,
            next,
            previous,
            add_to_queue,
            remove_from_queue,
            clear_queue,
            favorite_track,
            download_track,
            cancel_download,
            get_downloads,
            scan_library,
            update_settings,
            get_settings,
            reconnect,
            clear_cache,
            complete_onboarding
        ])
        .run(tauri::generate_context!())
        .expect("SoulMusic failed to start");
}

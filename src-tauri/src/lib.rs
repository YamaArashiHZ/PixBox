mod http;
mod pixiv_api;
mod compress;
mod download;

use pixiv_api::AppState;
use std::sync::Mutex;
use tauri::Manager;
use tauri::AppHandle;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_store::Builder::default().build())
        .setup(|app| {
            let handle = app.handle().clone();
            let proxy = load_proxy_setting(&handle);
            let client = http::create_client(&proxy)
                .map_err(|e| format!("Failed to create HTTP client: {e}"))?;

            app.manage(AppState {
                access_token: Mutex::new(None),
                client: Mutex::new(Some(client)),
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            pixiv_api::start_oauth,
            pixiv_api::get_login_status,
            pixiv_api::logout,
            pixiv_api::fetch_feed,
            pixiv_api::get_image_data,
            pixiv_api::save_images,
            pixiv_api::test_proxy,
            get_settings,
            set_settings,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

pub fn load_proxy_setting(app: &tauri::AppHandle) -> String {
    let path = app
        .path()
        .app_data_dir()
        .ok()
        .map(|d| d.join("settings.json"));

    if let Some(p) = path {
        if let Ok(data) = std::fs::read_to_string(&p) {
            if let Ok(json) = serde_json::from_str::<serde_json::Value>(&data) {
                return json
                    .get("proxy")
                    .and_then(|v| v.as_str())
                    .unwrap_or("http://127.0.0.1:7897")
                    .to_string();
            }
        }
    }
    "http://127.0.0.1:7897".to_string()
}

#[tauri::command]
fn get_settings(app: AppHandle) -> String {
    let path = app
        .path()
        .app_data_dir()
        .ok()
        .map(|d| d.join("settings.json"));

    let defaults = serde_json::json!({
        "proxy": "http://127.0.0.1:7897",
        "save_dir": "",
        "compress_enabled": true,
        "compress_separate": true,
        "compress_dir": "",
        "compress_max_mb": 6,
        "user": null
    });

    if let Some(p) = path {
        if let Ok(data) = std::fs::read_to_string(&p) {
            if let Ok(existing) = serde_json::from_str::<serde_json::Value>(&data) {
                return existing.to_string();
            }
        }
    }

    defaults.to_string()
}

#[tauri::command]
fn set_settings(app: AppHandle, settings: String) {
    let dir = app.path().app_data_dir().ok();
    if let Some(d) = dir {
        std::fs::create_dir_all(&d).ok();
        let path = d.join("settings.json");
        std::fs::write(&path, &settings).ok();
    }
}

use base64::{engine::general_purpose::STANDARD as BASE64, engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::sync::{Arc, Mutex};
use tauri::ipc::Channel;
use tauri::{AppHandle, Manager, WebviewUrl, WebviewWindowBuilder};
use tokio::sync::oneshot;

use crate::http;
use crate::compress;
use crate::download;

pub const CLIENT_ID: &str = "MOBrBDS8blbauoSck0ZfDbtuzpyT";
pub const CLIENT_SECRET: &str = "lsACyCD94FhDUtGTXi3QzcFE2uU1hqtDaKeqrdwj";

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PixivUser {
    pub id: String,
    pub name: String,
    pub account: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct TokenResponse {
    access_token: String,
    refresh_token: String,
    #[serde(default)]
    expires_in: u64,
    #[serde(default)]
    token_type: String,
    user: PixivUser,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TokenData {
    pub refresh_token: String,
    pub user: PixivUser,
}

pub struct AppState {
    pub access_token: Mutex<Option<String>>,
    pub client: Mutex<Option<Client>>,
}

pub fn generate_pkce() -> (String, String) {
    let mut bytes = [0u8; 32];
    getrandom::getrandom(&mut bytes).expect("Failed to generate random bytes");
    let verifier = URL_SAFE_NO_PAD.encode(&bytes);
    let mut hasher = Sha256::new();
    hasher.update(verifier.as_bytes());
    let challenge = URL_SAFE_NO_PAD.encode(hasher.finalize());
    (verifier, challenge)
}

fn extract_code_from_url(url: &str) -> Option<String> {
    let prefix = "pixiv://account/login?code=";
    if let Some(pos) = url.find(prefix) {
        let code_part = &url[pos + prefix.len()..];
        let end = code_part.find('&').unwrap_or(code_part.len());
        let code = code_part[..end].to_string();
        if !code.is_empty() {
            return Some(code);
        }
    }
    None
}

async fn exchange_token(client: &Client, code: &str, verifier: &str) -> Result<TokenResponse, String> {
    let params = [
        ("client_id", CLIENT_ID),
        ("client_secret", CLIENT_SECRET),
        ("grant_type", "authorization_code"),
        ("code", code),
        ("code_verifier", verifier),
        ("redirect_uri", "https://app-api.pixiv.net/web/v1/users/auth/pixiv/callback"),
    ];

    let resp = client
        .post("https://oauth.secure.pixiv.net/auth/token")
        .form(&params)
        .send()
        .await
        .map_err(|e| format!("Token request failed: {e}"))?;

    let status = resp.status();
    let text = resp.text().await.map_err(|e| format!("Response read error: {e}"))?;

    if !status.is_success() {
        return Err(format!("Token exchange failed ({}): {}", status, text));
    }

    let token: TokenResponse = serde_json::from_str(&text).map_err(|e| format!("Token parse error: {e}"))?;
    Ok(token)
}

pub async fn refresh_access_token(client: &Client, refresh_token: &str) -> Result<TokenResponse, String> {
    let params = [
        ("client_id", CLIENT_ID),
        ("client_secret", CLIENT_SECRET),
        ("grant_type", "refresh_token"),
        ("refresh_token", refresh_token),
    ];

    let resp = client
        .post("https://oauth.secure.pixiv.net/auth/token")
        .form(&params)
        .send()
        .await
        .map_err(|e| format!("Token refresh failed: {e}"))?;

    let status = resp.status();
    let text = resp.text().await.map_err(|e| format!("Response read error: {e}"))?;

    if !status.is_success() {
        return Err(format!("Token refresh failed ({}): {}", status, text));
    }

    let token: TokenResponse = serde_json::from_str(&text).map_err(|e| format!("Token parse error: {e}"))?;
    Ok(token)
}

pub fn load_token_data(app: &AppHandle) -> Option<TokenData> {
    let path = app.path().app_data_dir().ok()?.join("tokens.json");
    let data = std::fs::read_to_string(&path).ok()?;
    serde_json::from_str(&data).ok()
}

pub fn save_token_data(app: &AppHandle, data: &TokenData) -> Result<(), String> {
    let dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let path = dir.join("tokens.json");
    let json = serde_json::to_string_pretty(data).map_err(|e| e.to_string())?;
    std::fs::write(&path, json).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn start_oauth(app: AppHandle, state: tauri::State<'_, AppState>) -> Result<PixivUser, String> {
    let (verifier, challenge) = generate_pkce();

    let login_url = format!(
        "https://app-api.pixiv.net/web/v1/login?code_challenge={}&code_challenge_method=S256&client=pixiv-android",
        challenge
    );

    let url: url::Url = login_url.parse().map_err(|e: url::ParseError| e.to_string())?;

    let (tx, rx) = oneshot::channel::<Result<String, String>>();
    let tx = Arc::new(Mutex::new(Some(tx)));

    let tx_nav = tx.clone();
    let window = WebviewWindowBuilder::new(&app, "oauth", WebviewUrl::External(url))
        .title("Pixiv 登录")
        .inner_size(800.0, 700.0)
        .center()
        .on_navigation(move |nav_url| {
            let s = nav_url.as_str();
            eprintln!("[OAuth nav] {}", s);
            if let Some(code) = extract_code_from_url(s) {
                eprintln!("[OAuth code] {}", code);
                if let Some(tx) = tx_nav.lock().unwrap().take() {
                    let _ = tx.send(Ok(code));
                }
                return false;
            }
            true
        })
        .build()
        .map_err(|e| e.to_string())?;

    let tx_close = tx.clone();
    let window_handle = window.clone();
    window.on_window_event(move |event| {
        eprintln!("[OAuth event] {:?}", event);
        if let tauri::WindowEvent::Destroyed = event {
            if let Some(tx) = tx_close.lock().unwrap().take() {
                let _ = tx.send(Err("用户取消了登录".into()));
            }
        }
    });

    let code = match rx.await {
        Ok(Ok(code)) => code,
        Ok(Err(e)) => {
            eprintln!("[OAuth] login cancelled: {e}");
            window_handle.close().ok();
            return Err(e);
        }
        Err(_) => {
            eprintln!("[OAuth] channel error");
            return Err("登录流程异常".into());
        }
    };

    window_handle.close().ok();

    eprintln!("[OAuth] exchanging token for code: {}", &code[..code.len().min(8)]);

    let token = {
        let client = state.client.lock().unwrap().clone().ok_or("HTTP 客户端未初始化")?;
        let result = exchange_token(&client, &code, &verifier).await;
        result?
    };

    eprintln!("[OAuth] token received, user: {}", token.user.name);

    let token_data = TokenData {
        refresh_token: token.refresh_token.clone(),
        user: token.user.clone(),
    };
    save_token_data(&app, &token_data)?;

    *state.access_token.lock().unwrap() = Some(token.access_token);

    Ok(token.user)
}

#[tauri::command]
pub async fn get_login_status(app: AppHandle) -> Result<Option<PixivUser>, String> {
    Ok(load_token_data(&app).map(|d| d.user))
}

#[tauri::command]
pub async fn logout(app: AppHandle, state: tauri::State<'_, AppState>) -> Result<(), String> {
    let dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    let path = dir.join("tokens.json");
    if path.exists() {
        std::fs::remove_file(&path).map_err(|e| e.to_string())?;
    }
    *state.access_token.lock().unwrap() = None;
    Ok(())
}

#[derive(Debug, Deserialize)]
struct PixivFeedResponse {
    illusts: Vec<PixivIllust>,
    next_url: Option<String>,
}

#[derive(Debug, Deserialize)]
struct PixivIllust {
    id: u64,
    title: String,
    user: PixivIllustUser,
    page_count: u32,
    is_bookmarked: bool,
    image_urls: IllustImageUrls,
    meta_single_page: Option<MetaSinglePage>,
    meta_pages: Option<Vec<MetaPage>>,
}

#[derive(Debug, Deserialize)]
struct PixivIllustUser {
    id: u64,
    name: String,
    account: String,
}

#[derive(Debug, Deserialize)]
struct IllustImageUrls {
    square_medium: Option<String>,
    medium: Option<String>,
    large: Option<String>,
}

#[derive(Debug, Deserialize)]
struct MetaSinglePage {
    original_image_url: Option<String>,
}

#[derive(Debug, Deserialize)]
struct MetaPage {
    image_urls: MetaImageUrls,
}

#[derive(Debug, Deserialize)]
struct MetaImageUrls {
    original: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct FeedItem {
    pub key: String,
    pub illust_id: u64,
    pub page: u32,
    pub thumb_b64: String,
    pub large_url: String,
    pub original_url: String,
    pub title: String,
    pub artist: String,
    pub is_bookmarked: bool,
}

#[derive(Debug, Serialize)]
pub struct FeedPage {
    pub items: Vec<FeedItem>,
    pub next_url: Option<String>,
}

fn expand_illust(illust: &PixivIllust) -> Vec<(u32, String, String, String)> {
    let thumb = illust
        .image_urls
        .square_medium
        .as_deref()
        .unwrap_or("");

    let default_large = illust.image_urls.large.as_deref().unwrap_or("");

    if illust.page_count <= 1 {
        let original = illust
            .meta_single_page
            .as_ref()
            .and_then(|m| m.original_image_url.as_deref())
            .unwrap_or(default_large);
        let large = illust
            .image_urls
            .large
            .as_deref()
            .unwrap_or_else(|| illust.image_urls.medium.as_deref().unwrap_or(""));
        vec![(
            0,
            thumb.to_string(),
            large.to_string(),
            original.to_string(),
        )]
    } else if let Some(pages) = &illust.meta_pages {
        pages
            .iter()
            .enumerate()
            .map(|(i, page)| {
                let original = page.image_urls.original.as_deref().unwrap_or("");
                let large = illust
                    .image_urls
                    .large
                    .as_deref()
                    .unwrap_or_else(|| illust.image_urls.medium.as_deref().unwrap_or(""));
                (
                    i as u32,
                    thumb.to_string(),
                    large.to_string(),
                    original.to_string(),
                )
            })
            .collect()
    } else {
        vec![]
    }
}

async fn get_or_refresh_token(app: &AppHandle, state: &AppState) -> Result<String, String> {
    if let Some(token) = state.access_token.lock().unwrap().clone() {
        return Ok(token);
    }

    let token_data = load_token_data(app).ok_or("未登录，请先登录")?;
    let client = state.client.lock().unwrap().clone().ok_or("HTTP 客户端未初始化")?;
    let result = refresh_access_token(&client, &token_data.refresh_token).await;

    match result {
        Ok(token_resp) => {
            let new_data = TokenData {
                refresh_token: token_resp.refresh_token.clone(),
                user: token_resp.user.clone(),
            };
            save_token_data(app, &new_data)?;
            let token = token_resp.access_token.clone();
            *state.access_token.lock().unwrap() = Some(token.clone());
            Ok(token)
        }
        Err(e) => {
            let dir = app.path().app_data_dir().map_err(|d| d.to_string()).unwrap_or_default();
            let path = std::path::Path::new(&dir).join("tokens.json");
            std::fs::remove_file(&path).ok();
            *state.access_token.lock().unwrap() = None;
            Err(format!("登录已过期，请重新登录: {e}"))
        }
    }
}

async fn download_thumbnail(
    client: &Client,
    url: &str,
) -> Vec<u8> {
    if url.is_empty() {
        return vec![];
    }
    match client.get(url).send().await {
        Ok(resp) => resp.bytes().await.map(|b| b.to_vec()).unwrap_or_default(),
        Err(_) => vec![],
    }
}

#[tauri::command]
pub async fn fetch_feed(
    app: AppHandle,
    state: tauri::State<'_, AppState>,
    kind: String,
    next_url: Option<String>,
) -> Result<FeedPage, String> {
    let token = get_or_refresh_token(&app, &state).await?;
    let proxy = crate::load_proxy_setting(&app);
    let api_client = state.client.lock().unwrap().clone().ok_or("HTTP 客户端未初始化")?;

    let img_client = http::create_client_with_referer(proxy.as_deref())?;

    let url = next_url.unwrap_or_else(|| match kind.as_str() {
        "recommended" => "https://app-api.pixiv.net/v1/illust/recommended".to_string(),
        _ => "https://app-api.pixiv.net/v2/illust/follow?restrict=public".to_string(),
    });

    let resp = api_client
        .get(&url)
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .await
        .map_err(|e| format!("API request failed: {e}"))?;

        let status = resp.status();
    let text = resp.text().await.map_err(|e| format!("Response read: {e}"))?;




    if !status.is_success() {
        return Err(format!("API returned {}: {}", status, text));
    }

    let feed: PixivFeedResponse =
        serde_json::from_str(&text).map_err(|e| format!("Parse error: {e}"))?;

    let mut items = Vec::new();
    for illust in &feed.illusts {
        let expanded = expand_illust(illust);
        let artist = illust.user.name.clone();
        let title = illust.title.clone();
        let is_bookmarked = illust.is_bookmarked;

        for (page, thumb_url, large_url, original_url) in expanded {
            let key = if illust.page_count > 1 {
                format!("{}_p{}", illust.id, page)
            } else {
                format!("{}_p0", illust.id)
            };

            let thumb_data = download_thumbnail(&img_client, &thumb_url).await;
            let thumb_b64 = if thumb_data.is_empty() {
                String::new()
            } else {
                BASE64.encode(&thumb_data)
            };

            items.push(FeedItem {
                key,
                illust_id: illust.id,
                page,
                thumb_b64,
                large_url,
                original_url,
                title: title.clone(),
                artist: artist.clone(),
                is_bookmarked,
            });
        }
    }

    Ok(FeedPage {
        items,
        next_url: feed.next_url,
    })
}

#[tauri::command]
pub async fn get_image_data(
    app: AppHandle,
    state: tauri::State<'_, AppState>,
    url: String,
) -> Result<String, String> {
    let proxy = crate::load_proxy_setting(&app);
    let client = http::create_client_with_referer(proxy.as_deref())?;

    let resp = client
        .get(&url)
        .send()
        .await
        .map_err(|e| format!("Download failed: {e}"))?;

    let data = resp
        .bytes()
        .await
        .map_err(|e| format!("Read failed: {e}"))?;

    Ok(BASE64.encode(&data))
}

#[derive(Debug, Deserialize)]
struct Settings {
    #[serde(default)]
    proxy_enabled: bool,
    #[serde(default = "default_proxy")]
    proxy: String,
    #[serde(default)]
    save_dir: String,
    #[serde(default)]
    compress_enabled: bool,
    #[serde(default)]
    compress_separate: bool,
    #[serde(default)]
    compress_dir: String,
    #[serde(default = "default_max_mb")]
    compress_max_mb: f64,
}

fn default_proxy() -> String {
    "http://127.0.0.1:7897".into()
}
fn default_max_mb() -> f64 {
    6.0
}

fn load_settings(app: &AppHandle) -> Settings {
    let path = app
        .path()
        .app_data_dir()
        .ok()
        .map(|d| d.join("settings.json"));

    if let Some(p) = path {
        if let Ok(data) = std::fs::read_to_string(&p) {
            if let Ok(s) = serde_json::from_str(&data) {
                return s;
            }
        }
    }
    Settings {
        proxy_enabled: true,
        proxy: default_proxy(),
        save_dir: String::new(),
        compress_enabled: true,
        compress_separate: true,
        compress_dir: String::new(),
        compress_max_mb: default_max_mb(),
    }
}

#[derive(Debug, Deserialize)]
pub struct SaveItem {
    pub key: String,
    pub illust_id: u64,
    pub original_url: String,
}

#[derive(Debug, Serialize, Clone)]
pub struct ProgressEvent {
    pub current: usize,
    pub total: usize,
    pub percent: u32,
    pub key: String,
    pub status: String,
}

#[derive(Debug, Serialize)]
pub struct SaveReport {
    pub success: usize,
    pub failed: usize,
    pub errors: Vec<String>,
}

async fn bookmark_add(state: &AppState, token: &str, illust_id: u64) {
    let client = state.client.lock().unwrap().clone();
    if let Some(client) = client {
        let _ = client
            .post("https://app-api.pixiv.net/v1/illust/bookmark/add")
            .header("Authorization", format!("Bearer {token}"))
            .form(&[("illust_id", illust_id.to_string())])
            .send()
            .await;
    }
}

#[tauri::command]
pub async fn save_images(
    app: AppHandle,
    state: tauri::State<'_, AppState>,
    items: Vec<SaveItem>,
    on_progress: Channel<ProgressEvent>,
) -> Result<SaveReport, String> {
    let settings = load_settings(&app);

    if settings.save_dir.is_empty() {
        return Err("请先在设置中配置原图保存路径".into());
    }

    let token = get_or_refresh_token(&app, &state).await?;
    let img_client = http::create_client_with_referer(
        if settings.proxy_enabled { Some(&settings.proxy) } else { None }
    )?;

    let today = chrono::Local::now().format("%Y-%m-%d").to_string();
    let total = items.len();
    let mut success = 0;
    let mut errors: Vec<String> = Vec::new();

    for (i, item) in items.iter().enumerate() {
        let key = &item.key;

        on_progress
            .send(ProgressEvent {
                current: i,
                total,
                percent: ((i as f64 / total as f64) * 100.0) as u32,
                key: key.clone(),
                status: "downloading".into(),
            })
            .map_err(|e| e.to_string())?;

        let (data, ext) = match download::download_image(&img_client, &item.original_url).await {
            Ok(d) => d,
            Err(e) => {
                errors.push(format!("{key}: {e}"));
                continue;
            }
        };

        if ext == "gif" {
            errors.push(format!("{key}: GIF skipped (v1 limitation)"));
            continue;
        }

        on_progress
            .send(ProgressEvent {
                current: i,
                total,
                percent: ((i as f64 / total as f64) * 100.0) as u32,
                key: key.clone(),
                status: "saving".into(),
            })
            .map_err(|e| e.to_string())?;

        let save_dir = std::path::Path::new(&settings.save_dir).join(&today);
        std::fs::create_dir_all(&save_dir).map_err(|e| e.to_string())?;
        let save_path = save_dir.join(format!("{}.{}", key, ext));
        if let Err(e) = std::fs::write(&save_path, &data) {
            errors.push(format!("{key}: write failed - {e}"));
            continue;
        }

        if settings.compress_enabled {
            on_progress
                .send(ProgressEvent {
                    current: i,
                    total,
                    percent: ((i as f64 / total as f64) * 100.0) as u32,
                    key: key.clone(),
                    status: "compressing".into(),
                })
                .map_err(|e| e.to_string())?;

            let compressed = compress::compress(&data, settings.compress_max_mb);
            let compress_dir = if settings.compress_separate {
                std::path::Path::new(&settings.compress_dir).join(&today)
            } else {
                save_dir.clone()
            };
            if let Err(e) = std::fs::create_dir_all(&compress_dir) {
                errors.push(format!("{key}: {e}"));
                continue;
            }
            let compress_path = compress_dir.join(format!("{}.jpg", key));
            if let Err(e) = std::fs::write(&compress_path, &compressed) {
                errors.push(format!("{key}: compress write failed - {e}"));
                continue;
            }
        }

        on_progress
            .send(ProgressEvent {
                current: i,
                total,
                percent: ((i as f64 / total as f64) * 100.0) as u32,
                key: key.clone(),
                status: "bookmark".into(),
            })
            .map_err(|e| e.to_string())?;

        bookmark_add(&state, &token, item.illust_id).await;

        if i < total - 1 {
            tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
        }

        on_progress
            .send(ProgressEvent {
                current: i + 1,
                total,
                percent: (((i + 1) as f64 / total as f64) * 100.0) as u32,
                key: key.clone(),
                status: "done".into(),
            })
            .map_err(|e| e.to_string())?;

        success += 1;
    }

    Ok(SaveReport {
        success,
        failed: errors.len(),
        errors,
    })
}

#[tauri::command]
pub async fn test_proxy(proxy: String) -> Result<u64, String> {
    let client = http::create_client(Some(&proxy))?;
    let start = std::time::Instant::now();

    let _resp = client
        .get("https://app-api.pixiv.net/v1/illust/recommended")
        .send()
        .await
        .map_err(|e| format!("Connection failed: {e}"))?;

    let elapsed = start.elapsed().as_millis() as u64;
    Ok(elapsed)
}

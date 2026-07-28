use base64::{engine::general_purpose::STANDARD as BASE64, engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::sync::{Arc, Mutex};
use tauri::ipc::Channel;
use tauri::{AppHandle, Manager, Emitter, WebviewUrl, WebviewWindowBuilder};
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
    /// Web Discovery 用的 PHPSESSID（HttpOnly cookie，登录时从 Webview 抓取）
    #[serde(default)]
    pub phpsessid: Option<String>,
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

    eprintln!("[OAuth] exchanging token for code: {}", &code[..code.len().min(8)]);

    // 登录成功后跳转 www.pixiv.net，建立/读取 Web 会话 cookie（Discovery 需要）
    let phpsessid = capture_phpsessid_from_window(&window_handle).await;
    window_handle.close().ok();

    let token = {
        let client = state.client.lock().unwrap().clone().ok_or("HTTP 客户端未初始化")?;
        let result = exchange_token(&client, &code, &verifier).await;
        result?
    };

    eprintln!("[OAuth] token received, user: {}", token.user.name);
    if phpsessid.is_some() {
        eprintln!("[OAuth] PHPSESSID captured");
    } else {
        eprintln!("[OAuth] PHPSESSID missing — Web Discovery may require re-login");
    }

    let token_data = TokenData {
        refresh_token: token.refresh_token.clone(),
        user: token.user.clone(),
        phpsessid,
    };
    save_token_data(&app, &token_data)?;

    *state.access_token.lock().unwrap() = Some(token.access_token);

    Ok(token.user)
}

/// 从 OAuth Webview 的 cookie jar 读取 www.pixiv.net 的 PHPSESSID
async fn capture_phpsessid_from_window(window: &tauri::WebviewWindow) -> Option<String> {
    let home = url::Url::parse("https://www.pixiv.net/").ok()?;
    let discovery = url::Url::parse("https://www.pixiv.net/discovery").ok()?;
    let cookie_url = url::Url::parse("https://www.pixiv.net").ok()?;

    let _ = window.navigate(home);
    for attempt in 0..30 {
        tokio::time::sleep(std::time::Duration::from_millis(350)).await;
        if attempt == 12 {
            let _ = window.navigate(discovery.clone());
        }
        let win = window.clone();
        let url = cookie_url.clone();
        let cookies = match tokio::task::spawn_blocking(move || win.cookies_for_url(url)).await {
            Ok(Ok(c)) => c,
            _ => continue,
        };
        for c in cookies {
            if c.name() == "PHPSESSID" {
                let v = c.value().to_string();
                // 有效会话通常为 "{userId}_{token}"
                if v.contains('_') && v.len() > 10 {
                    return Some(v);
                }
            }
        }
    }
    None
}

fn load_phpsessid(app: &AppHandle) -> Option<String> {
    load_token_data(app)?.phpsessid.filter(|s| !s.is_empty())
}

fn save_phpsessid(app: &AppHandle, phpsessid: String) -> Result<(), String> {
    let mut data = load_token_data(app).ok_or("未登录")?;
    data.phpsessid = Some(phpsessid);
    save_token_data(app, &data)
}

/// 已有 OAuth 登录但缺少 PHPSESSID 时，用隐藏 Webview 尝试从持久化 cookie 配置恢复
async fn bootstrap_phpsessid(app: &AppHandle) -> Result<String, String> {
    if let Some(s) = load_phpsessid(app) {
        return Ok(s);
    }

    if let Some(existing) = app.get_webview_window("web-session") {
        existing.close().ok();
    }
    let url: url::Url = "https://www.pixiv.net/discovery"
        .parse()
        .map_err(|e: url::ParseError| e.to_string())?;
    let window = WebviewWindowBuilder::new(app, "web-session", WebviewUrl::External(url))
        .title("Pixiv Web Session")
        .inner_size(400.0, 300.0)
        .visible(false)
        .build()
        .map_err(|e| format!("创建 Web 会话窗口失败: {e}"))?;

    let sid = capture_phpsessid_from_window(&window).await;
    window.close().ok();

    match sid {
        Some(s) => {
            save_phpsessid(app, s.clone())?;
            Ok(s)
        }
        None => Err(
            "无法获取网页登录态（PHPSESSID）。请退出后重新登录，以便启用与网页端一致的推荐流。"
                .into(),
        ),
    }
}

fn create_web_ajax_client(proxy: Option<&str>, phpsessid: &str) -> Result<Client, String> {
    use reqwest::header::{HeaderMap, HeaderValue, COOKIE, REFERER, USER_AGENT};

    let mut headers = HeaderMap::new();
    headers.insert(
        USER_AGENT,
        HeaderValue::from_static(
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/122.0.0.0 Safari/537.36",
        ),
    );
    headers.insert(REFERER, HeaderValue::from_static("https://www.pixiv.net/"));
    headers.insert(
        COOKIE,
        HeaderValue::from_str(&format!("PHPSESSID={phpsessid}"))
            .map_err(|e| format!("Invalid PHPSESSID: {e}"))?,
    );

    let mut builder = Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .default_headers(headers);

    if let Some(p) = proxy {
        if !p.is_empty() {
            let pxy = reqwest::Proxy::all(p).map_err(|e| e.to_string())?;
            builder = builder.proxy(pxy);
        }
    }

    builder.build().map_err(|e| e.to_string())
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
    x_restrict: Option<u32>,
    illust_ai_type: Option<u32>,
    #[serde(default)]
    width: u32,
    #[serde(default)]
    height: u32,
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
    square_medium: Option<String>,
    medium: Option<String>,
    large: Option<String>,
    original: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct FeedItem {
    pub key: String,
    pub illust_id: u64,
    pub page: u32,
    pub page_count: u32,
    pub thumb_b64: String,
    pub large_url: String,
    pub original_url: String,
    pub title: String,
    pub artist: String,
    pub is_bookmarked: bool,
    #[serde(default)]
    pub width: u32,
    #[serde(default)]
    pub height: u32,
    #[serde(default)]
    pub x_restrict: u32,
    #[serde(default)]
    pub illust_ai_type: u32,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
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
                let thumb = page
                    .image_urls
                    .square_medium
                    .as_deref()
                    .unwrap_or_else(|| page.image_urls.medium.as_deref().unwrap_or(""));
                let original = page.image_urls.original.as_deref().unwrap_or("");
                let large = page
                    .image_urls
                    .large
                    .as_deref()
                    .unwrap_or_else(|| page.image_urls.medium.as_deref().unwrap_or(""));
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
                phpsessid: token_data.phpsessid.clone(),
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

fn de_id_string_or_u64<'de, D>(deserializer: D) -> Result<u64, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::de::{self, Visitor};
    use std::fmt;

    struct IdVisitor;
    impl<'de> Visitor<'de> for IdVisitor {
        type Value = u64;
        fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
            f.write_str("string or integer id")
        }
        fn visit_u64<E: de::Error>(self, v: u64) -> Result<u64, E> {
            Ok(v)
        }
        fn visit_i64<E: de::Error>(self, v: i64) -> Result<u64, E> {
            if v < 0 {
                return Err(E::custom("negative id"));
            }
            Ok(v as u64)
        }
        fn visit_str<E: de::Error>(self, v: &str) -> Result<u64, E> {
            v.parse().map_err(E::custom)
        }
    }
    deserializer.deserialize_any(IdVisitor)
}

#[derive(Debug, Deserialize)]
struct DiscoveryAjaxResponse {
    error: bool,
    #[serde(default)]
    message: String,
    body: Option<DiscoveryBody>,
}

#[derive(Debug, Deserialize)]
struct DiscoveryBody {
    thumbnails: DiscoveryThumbnails,
}

#[derive(Debug, Deserialize)]
struct DiscoveryThumbnails {
    #[serde(default)]
    illust: Vec<DiscoveryIllust>,
}

#[derive(Debug, Deserialize)]
struct DiscoveryIllust {
    #[serde(deserialize_with = "de_id_string_or_u64")]
    id: u64,
    title: String,
    #[serde(rename = "userName", default)]
    user_name: String,
    #[serde(rename = "xRestrict", default)]
    x_restrict: u32,
    #[serde(rename = "pageCount", default)]
    page_count: u32,
    #[serde(default)]
    width: u32,
    #[serde(default)]
    height: u32,
    #[serde(default)]
    url: Option<String>,
    #[serde(rename = "bookmarkData")]
    bookmark_data: Option<serde_json::Value>,
    #[serde(default)]
    tags: Vec<String>,
    #[serde(rename = "aiType")]
    ai_type: Option<u32>,
}

#[derive(Debug, Deserialize)]
struct IllustDetailResponse {
    illust: PixivIllust,
}

async fn fetch_illust_detail(client: &Client, token: &str, id: u64) -> Result<PixivIllust, String> {
    let url = format!("https://app-api.pixiv.net/v1/illust/detail?illust_id={id}");
    let resp = client
        .get(&url)
        .header("Authorization", format!("Bearer {token}"))
        .send()
        .await
        .map_err(|e| format!("illust detail request failed: {e}"))?;
    let status = resp.status();
    let text = resp.text().await.map_err(|e| format!("illust detail read: {e}"))?;
    if !status.is_success() {
        return Err(format!("illust detail {}: {}", status, text));
    }
    let parsed: IllustDetailResponse =
        serde_json::from_str(&text).map_err(|e| format!("illust detail parse: {e}"))?;
    Ok(parsed.illust)
}

fn discovery_ai_type(illust: &DiscoveryIllust) -> u32 {
    if let Some(t) = illust.ai_type {
        if t >= 2 {
            return 2;
        }
    }
    if illust.tags.iter().any(|t| {
        let lower = t.to_lowercase();
        lower == "ai生成" || lower == "ai-generated" || t == "AI生成"
    }) {
        2
    } else {
        0
    }
}

/// 从 Discovery 缩略图 URL 推导 large / original（detail 失败时的兜底）
fn urls_from_discovery_thumb(thumb: &str, page: u32) -> (String, String) {
    // .../img-master/img/YYYY/MM/DD/hh/mm/ss/{id}_p0_square1200.jpg
    // -> img-master ... _pN_master1200.jpg
    // -> img-original ... _pN.jpg
    if let Some(idx) = thumb.find("/img-master/img/") {
        let rest = &thumb[idx + "/img-master/img/".len()..];
        if let Some(fname) = rest.rsplit('/').next() {
            let id_part = fname
                .split("_p")
                .next()
                .unwrap_or("")
                .to_string();
            let date_path = rest.trim_end_matches(fname).trim_end_matches('/');
            if !id_part.is_empty() && !date_path.is_empty() {
                let large = format!(
                    "https://i.pximg.net/img-master/img/{date_path}/{id_part}_p{page}_master1200.jpg"
                );
                let original = format!(
                    "https://i.pximg.net/img-original/img/{date_path}/{id_part}_p{page}.jpg"
                );
                return (large, original);
            }
        }
    }
    (thumb.to_string(), thumb.to_string())
}

async fn fetch_discovery_feed(
    app: &AppHandle,
    state: &AppState,
    content_mode: Option<String>,
) -> Result<FeedPage, String> {
    let token = get_or_refresh_token(app, state).await?;
    let proxy = crate::load_proxy_setting(app);
    let api_client = state
        .client
        .lock()
        .unwrap()
        .clone()
        .ok_or("HTTP 客户端未初始化")?;

    let mode = match content_mode.as_deref() {
        Some("safe") => "safe",
        Some("r18") => "r18",
        _ => "all",
    };

    let phpsessid = bootstrap_phpsessid(app).await?;
    let web_client = create_web_ajax_client(proxy.as_deref(), &phpsessid)?;
    let url = format!(
        "https://www.pixiv.net/ajax/discovery/artworks?mode={mode}&limit=60"
    );

    let resp = web_client
        .get(&url)
        .send()
        .await
        .map_err(|e| format!("Discovery request failed: {e}"))?;
    let status = resp.status();
    let text = resp
        .text()
        .await
        .map_err(|e| format!("Discovery response read: {e}"))?;

    if !status.is_success() {
        // cookie 失效时清掉并提示重登
        if status.as_u16() == 401 || status.as_u16() == 403 || text.contains("login") {
            if let Some(mut data) = load_token_data(app) {
                data.phpsessid = None;
                let _ = save_token_data(app, &data);
            }
            return Err("网页登录态已失效，请重新登录以使用推荐流".into());
        }
        return Err(format!("Discovery returned {}: {}", status, &text[..text.len().min(300)]));
    }

    let parsed: DiscoveryAjaxResponse =
        serde_json::from_str(&text).map_err(|e| format!("Discovery parse error: {e}"))?;
    if parsed.error {
        return Err(format!("Discovery error: {}", parsed.message));
    }
    let thumbs = parsed
        .body
        .map(|b| b.thumbnails.illust)
        .unwrap_or_default();

    // 并发用 App API 拉详情（原图 / 多页 / AI / 收藏状态）
    let semaphore = Arc::new(tokio::sync::Semaphore::new(6));
    let mut handles = Vec::new();
    for d in &thumbs {
        let client = api_client.clone();
        let token = token.clone();
        let id = d.id;
        let disc = DiscoveryIllust {
            id: d.id,
            title: d.title.clone(),
            user_name: d.user_name.clone(),
            x_restrict: d.x_restrict,
            page_count: d.page_count,
            width: d.width,
            height: d.height,
            url: d.url.clone(),
            bookmark_data: d.bookmark_data.clone(),
            tags: d.tags.clone(),
            ai_type: d.ai_type,
        };
        let sem = semaphore.clone();
        handles.push(tokio::spawn(async move {
            let _permit = sem.acquire().await.ok();
            let detail = fetch_illust_detail(&client, &token, id).await.ok();
            (disc, detail)
        }));
    }

    let mut items = Vec::new();
    let mut thumb_tasks: Vec<(String, String)> = Vec::new();

    for h in handles {
        let (disc, detail) = match h.await {
            Ok(v) => v,
            Err(_) => continue,
        };

        if let Some(illust) = detail {
            let expanded = expand_illust(&illust);
            for (page, thumb_url, large_url, original_url) in expanded {
                let key = format!("{}_p{}", illust.id, page);
                thumb_tasks.push((key.clone(), thumb_url));
                items.push(FeedItem {
                    key,
                    illust_id: illust.id,
                    page,
                    page_count: illust.page_count,
                    thumb_b64: String::new(),
                    large_url,
                    original_url,
                    title: illust.title.clone(),
                    artist: illust.user.name.clone(),
                    is_bookmarked: illust.is_bookmarked,
                    width: illust.width,
                    height: illust.height,
                    x_restrict: illust.x_restrict.unwrap_or(0),
                    illust_ai_type: illust.illust_ai_type.unwrap_or(0),
                });
            }
        } else {
            // detail 失败：用 Discovery 摘要 + URL 推导（仅封面页）
            let thumb = disc.url.clone().unwrap_or_default();
            let (large, original) = urls_from_discovery_thumb(&thumb, 0);
            let key = format!("{}_p0", disc.id);
            let ai = discovery_ai_type(&disc);
            let bookmarked = disc.bookmark_data.is_some();
            thumb_tasks.push((key.clone(), thumb));
            items.push(FeedItem {
                key,
                illust_id: disc.id,
                page: 0,
                page_count: disc.page_count.max(1),
                thumb_b64: String::new(),
                large_url: large,
                original_url: original,
                title: disc.title,
                artist: disc.user_name,
                is_bookmarked: bookmarked,
                width: disc.width,
                height: disc.height,
                x_restrict: disc.x_restrict,
                illust_ai_type: ai,
            });
        }
    }

    let result = FeedPage {
        items,
        // Discovery 无稳定 next_url；用哨兵标记可继续请求新一批
        next_url: Some(format!("discovery:{mode}")),
    };

    let img_client = http::create_client_with_referer(proxy.as_deref())?;
    let emit_app = app.clone();
    tokio::spawn(async move {
        let _ = download_thumbnails_bg(img_client, thumb_tasks, emit_app).await;
    });

    Ok(result)
}

#[tauri::command]
pub async fn fetch_feed(
    app: AppHandle,
    state: tauri::State<'_, AppState>,
    kind: String,
    next_url: Option<String>,
    content_mode: Option<String>,
) -> Result<FeedPage, String> {
    let is_discovery = kind == "recommended"
        || next_url
            .as_deref()
            .map(|u| u.starts_with("discovery:"))
            .unwrap_or(false);

    if is_discovery {
        let mode = if let Some(u) = next_url.as_deref() {
            if let Some(m) = u.strip_prefix("discovery:") {
                Some(m.to_string())
            } else {
                content_mode
            }
        } else {
            content_mode
        };
        return fetch_discovery_feed(&app, &state, mode).await;
    }

    let token = get_or_refresh_token(&app, &state).await?;
    let proxy = crate::load_proxy_setting(&app);
    let api_client = state.client.lock().unwrap().clone().ok_or("HTTP 客户端未初始化")?;

    let is_initial_load = next_url.is_none();

    let url = next_url.unwrap_or_else(|| {
        let base = match kind.as_str() {
            "ranking" => "https://app-api.pixiv.net/v1/illust/ranking?mode=day_r18".to_string(),
            _ => "https://app-api.pixiv.net/v2/illust/follow?restrict=public".to_string(),
        };
        base
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
    let mut thumb_tasks: Vec<(String, String)> = Vec::new();
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

            thumb_tasks.push((key.clone(), thumb_url));

            items.push(FeedItem {
                key,
                illust_id: illust.id,
                page,
                page_count: illust.page_count,
                thumb_b64: String::new(),
                large_url,
                original_url,
                title: title.clone(),
                artist: artist.clone(),
                is_bookmarked,
                width: illust.width,
                height: illust.height,
                x_restrict: illust.x_restrict.unwrap_or(0),
                illust_ai_type: illust.illust_ai_type.unwrap_or(0),
            });
        }
    }

    let result = FeedPage {
        items,
        next_url: feed.next_url,
    };

    let img_client = http::create_client_with_referer(proxy.as_deref())?;
    let cache_kind = if is_initial_load && kind == "following" { Some(kind.clone()) } else { None };
    let cache_app = app.clone();
    let emit_app = app.clone();
    let mut cache_page = result.clone();
    tokio::spawn(async move {
        let b64_map = download_thumbnails_bg(img_client, thumb_tasks, emit_app).await;
        for item in &mut cache_page.items {
            if let Some(b64) = b64_map.get(&item.key) {
                item.thumb_b64 = b64.clone();
            }
        }
        if let Some(k) = cache_kind {
            save_feed_cache(&cache_app, &k, &cache_page);
        }
    });

    Ok(result)
}

async fn download_thumbnails_bg(
    client: reqwest::Client,
    tasks: Vec<(String, String)>,
    app: AppHandle,
) -> std::collections::HashMap<String, String> {
    let semaphore = std::sync::Arc::new(tokio::sync::Semaphore::new(8));
    let results = std::sync::Arc::new(std::sync::Mutex::new(std::collections::HashMap::new()));
    let mut handles = Vec::new();
    for (key, url) in tasks {
        let client = client.clone();
        let app = app.clone();
        let sem = semaphore.clone();
        let results = results.clone();
        handles.push(tokio::spawn(async move {
            let _permit = sem.acquire().await;
            let data = download_thumbnail(&client, &url).await;
            let b64 = if data.is_empty() {
                String::new()
            } else {
                BASE64.encode(&data)
            };
            app.emit("thumbnail-event", ThumbProgress { key: key.clone(), thumb_b64: b64.clone() }).ok();
            results.lock().unwrap().insert(key, b64);
        }));
    }
    for h in handles {
        h.await.ok();
    }
    Arc::try_unwrap(results)
        .map(|m| m.into_inner().unwrap())
        .unwrap_or_else(|arc| arc.lock().unwrap().clone())
}

#[tauri::command]
pub fn get_cache_size(app: AppHandle) -> Result<u64, String> {
    let dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    let mut total = 0u64;
    for kind in &["following"] {
        let path = dir.join(format!("feed_cache_{}.json", kind));
        if let Ok(meta) = std::fs::metadata(&path) {
            total += meta.len();
        }
    }
    // 大图缓存目录
    if let Ok(rd) = std::fs::read_dir(dir.join("image_cache")) {
        for entry in rd.flatten() {
            if let Ok(meta) = entry.metadata() {
                if meta.is_file() {
                    total += meta.len();
                }
            }
        }
    }
    Ok(total)
}

#[tauri::command]
pub fn clear_cache(app: AppHandle) -> Result<(), String> {
    let dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    for kind in &["following"] {
        let path = dir.join(format!("feed_cache_{}.json", kind));
        std::fs::remove_file(&path).ok();
    }
    std::fs::remove_dir_all(dir.join("image_cache")).ok();
    Ok(())
}

fn cache_path(app: &AppHandle, kind: &str) -> std::path::PathBuf {
    app.path()
        .app_data_dir()
        .unwrap_or_default()
        .join(format!("feed_cache_{}.json", kind))
}

fn save_feed_cache(app: &AppHandle, kind: &str, feed: &FeedPage) {
    if let Ok(json) = serde_json::to_string(&feed) {
        let path = cache_path(app, kind);
        std::fs::write(&path, json).ok();
    }
}

#[tauri::command]
pub fn load_cached_feed(app: AppHandle, kind: String) -> Result<FeedPage, String> {
    let path = cache_path(&app, &kind);
    let data = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
    serde_json::from_str(&data).map_err(|e| e.to_string())
}

// ---- 大图磁盘缓存（app_data_dir/image_cache/，每图一文件，LRU 淘汰）----

fn image_cache_dir(app: &AppHandle) -> Option<std::path::PathBuf> {
    app.path().app_data_dir().ok().map(|d| d.join("image_cache"))
}

// 文件名取 URL 最后一段（pixiv 的 {illust_id}_p{n}.jpg 全局唯一），非法则退回 URL 哈希
fn cache_file_name(url: &str) -> String {
    let seg = url.rsplit('/').next().unwrap_or("");
    let seg = seg.split(['?', '#']).next().unwrap_or("");
    let sanitized: String = seg
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-'))
        .collect();
    if sanitized.contains('.') && sanitized.len() <= 120 {
        sanitized
    } else {
        let mut hasher = Sha256::new();
        hasher.update(url.as_bytes());
        let digest = URL_SAFE_NO_PAD.encode(hasher.finalize());
        format!("img_{}", &digest[..16])
    }
}

// 超过上限时按最后访问时间（mtime）从旧到新删除
async fn enforce_image_cache_limit(app: &AppHandle, dir: &std::path::Path) {
    let Some(limit_mb) = load_settings(app).image_cache_limit_mb else {
        return; // 无上限
    };
    let limit_bytes = (limit_mb.max(0.0) * 1_048_576.0) as u64;

    let mut entries: Vec<(std::path::PathBuf, u64, std::time::SystemTime)> = Vec::new();
    if let Ok(mut rd) = tokio::fs::read_dir(dir).await {
        while let Ok(Some(entry)) = rd.next_entry().await {
            if let Ok(meta) = entry.metadata().await {
                if meta.is_file() {
                    let mtime = meta.modified().unwrap_or(std::time::SystemTime::UNIX_EPOCH);
                    entries.push((entry.path(), meta.len(), mtime));
                }
            }
        }
    }

    let mut total: u64 = entries.iter().map(|e| e.1).sum();
    if total <= limit_bytes {
        return;
    }

    entries.sort_by_key(|e| e.2);
    for (path, size, _) in entries {
        if total <= limit_bytes {
            break;
        }
        if tokio::fs::remove_file(&path).await.is_ok() {
            total -= size;
        }
    }
}

#[tauri::command]
pub async fn enforce_cache_limit(app: AppHandle) -> Result<(), String> {
    if let Some(dir) = image_cache_dir(&app) {
        enforce_image_cache_limit(&app, &dir).await;
    }
    Ok(())
}

#[tauri::command]
pub async fn get_image_data(
    app: AppHandle,
    state: tauri::State<'_, AppState>,
    url: String,
) -> Result<String, String> {
    let cache_dir = image_cache_dir(&app);
    let cache_path = cache_dir.as_ref().map(|d| d.join(cache_file_name(&url)));

    // 命中磁盘缓存：直接读盘返回；重写一遍更新 mtime，作为 LRU 的"最后访问时间"
    if let Some(path) = &cache_path {
        if let Ok(data) = tokio::fs::read(path).await {
            tokio::fs::write(path, &data).await.ok();
            return Ok(BASE64.encode(&data));
        }
    }

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

    // 写入缓存并执行限额清理
    if let (Some(dir), Some(path)) = (cache_dir, cache_path) {
        if tokio::fs::create_dir_all(&dir).await.is_ok()
            && tokio::fs::write(&path, &data).await.is_ok()
        {
            enforce_image_cache_limit(&app, &dir).await;
        }
    }

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
    use_subdir: bool,
    #[serde(default = "default_subdir_pattern")]
    subdir_pattern: String,
    #[serde(default)]
    compress_enabled: bool,
    #[serde(default)]
    compress_separate: bool,
    #[serde(default)]
    compress_dir: String,
    #[serde(default)]
    compress_use_subdir: bool,
    #[serde(default = "default_subdir_pattern")]
    compress_subdir_pattern: String,
    #[serde(default = "default_max_mb")]
    compress_max_mb: f64,
    #[serde(default = "default_image_cache_limit_mb")]
    image_cache_limit_mb: Option<f64>,
}

fn default_proxy() -> String {
    "http://127.0.0.1:7897".into()
}
fn default_max_mb() -> f64 {
    6.0
}
fn default_image_cache_limit_mb() -> Option<f64> {
    Some(200.0)
}
fn default_subdir_pattern() -> String {
    "%yy_%mm%dd_%HH%MM".into()
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
        use_subdir: true,
        subdir_pattern: default_subdir_pattern(),
        compress_enabled: true,
        compress_separate: true,
        compress_dir: String::new(),
        compress_use_subdir: true,
        compress_subdir_pattern: default_subdir_pattern(),
        compress_max_mb: default_max_mb(),
        image_cache_limit_mb: default_image_cache_limit_mb(),
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
    pub downloaded_bytes: u64,
    pub total_bytes: u64,
}

#[derive(Debug, Serialize, Clone)]
pub struct ThumbProgress {
    pub key: String,
    pub thumb_b64: String,
}

#[derive(Debug, Serialize)]
pub struct SaveReport {
    pub success: usize,
    pub failed: usize,
    pub errors: Vec<String>,
}

async fn bookmark_add(state: &AppState, token: &str, illust_id: u64) -> Result<(), String> {
    let client = state.client.lock().unwrap().clone();
    if let Some(client) = client {
        let resp = client
            .post("https://app-api.pixiv.net/v2/illust/bookmark/add")
            .header("Authorization", format!("Bearer {token}"))
            .form(&[
                ("illust_id", illust_id.to_string()),
                ("restrict", "public".to_string()),
            ])
            .send()
            .await
            .map_err(|e| format!("bookmark add request failed: {e}"))?;

        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        if !status.is_success() {
            eprintln!("bookmark_add failed ({}): {}", status, text);
            return Err(format!("bookmark add failed ({}): {}", status, text));
        }
        eprintln!("bookmark_add OK illust_id={illust_id} body={text}");
    }
    Ok(())
}

async fn bookmark_delete(state: &AppState, token: &str, illust_id: u64) -> Result<(), String> {
    let client = state.client.lock().unwrap().clone();
    if let Some(client) = client {
        let resp = client
            .post("https://app-api.pixiv.net/v1/illust/bookmark/delete")
            .header("Authorization", format!("Bearer {token}"))
            .form(&[("illust_id", illust_id.to_string())])
            .send()
            .await
            .map_err(|e| format!("bookmark delete request failed: {e}"))?;

        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        if !status.is_success() {
            eprintln!("bookmark_delete failed ({}): {}", status, text);
            return Err(format!("bookmark delete failed ({}): {}", status, text));
        }
        eprintln!("bookmark_delete OK illust_id={illust_id} body={text}");
    }
    Ok(())
}

#[tauri::command]
pub async fn toggle_bookmark(
    app: AppHandle,
    state: tauri::State<'_, AppState>,
    illust_id: u64,
    is_bookmarked: bool,
) -> Result<(), String> {
    let token = get_or_refresh_token(&app, &state).await?;
    if is_bookmarked {
        bookmark_delete(&state, &token, illust_id).await?;
    } else {
        bookmark_add(&state, &token, illust_id).await?;
    }
    Ok(())
}

fn build_subdir(pattern: &str) -> String {
    use chrono::{Datelike, Timelike};
    let now = chrono::Local::now();
    let chars: Vec<char> = pattern.chars().collect();
    let mut result = String::new();
    let mut i = 0;
    while i < chars.len() {
        if chars[i] == '%' && i + 1 < chars.len() {
            let c = chars[i + 1];
            // 统计同一字母的连续重复次数
            let mut len = 1;
            while i + 1 + len < chars.len() && chars[i + 1 + len] == c {
                len += 1;
            }
            let token = match (c, len) {
                ('y', 2) => Some(format!("{:02}", now.year() % 100)),
                ('y', 4) => Some(format!("{:04}", now.year())),
                ('m', 1) => Some(format!("{}", now.month())),
                ('m', 2) => Some(format!("{:02}", now.month())),
                ('d', 1) => Some(format!("{}", now.day())),
                ('d', 2) => Some(format!("{:02}", now.day())),
                ('H', 1) => Some(format!("{}", now.hour())),
                ('H', 2) => Some(format!("{:02}", now.hour())),
                ('M', 1) => Some(format!("{}", now.minute())),
                ('M', 2) => Some(format!("{:02}", now.minute())),
                ('S', 1) => Some(format!("{}", now.second())),
                ('S', 2) => Some(format!("{:02}", now.second())),
                _ => None,
            };
            if let Some(rep) = token {
                result.push_str(&rep);
                i += 1 + len;
            } else {
                // 未识别的占位符，原样输出
                result.push('%');
                i += 1;
            }
        } else {
            result.push(chars[i]);
            i += 1;
        }
    }
    result
        .replace('/', "_")
        .replace('\\', "_")
        .replace("..", "_")
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

    let save_subdir = if settings.use_subdir && !settings.subdir_pattern.is_empty() {
        Some(build_subdir(&settings.subdir_pattern))
    } else {
        None
    };

    let compress_subdir = if settings.compress_use_subdir && !settings.compress_subdir_pattern.is_empty() {
        Some(build_subdir(&settings.compress_subdir_pattern))
    } else {
        None
    };
    let total = items.len();
    let mut success = 0;
    let mut errors: Vec<String> = Vec::new();

    on_progress
        .send(ProgressEvent {
            current: 0,
            total,
            percent: 0,
            key: String::new(),
            status: "preparing".into(),
            downloaded_bytes: 0,
            total_bytes: 0,
        })
        .map_err(|e| e.to_string())?;

    let mut file_sizes: Vec<u64> = Vec::with_capacity(total);
    for item in &items {
        match download::get_content_length(&img_client, &item.original_url).await {
            Ok(len) => file_sizes.push(len),
            Err(e) => {
                eprintln!("[save_images] HEAD size for {}: {e}", item.key);
                file_sizes.push(0);
            }
        }
    }
    let total_bytes: u64 = file_sizes.iter().sum();

    let mut downloaded_bytes: u64 = 0;

    for (i, item) in items.iter().enumerate() {
        let key = &item.key;
        let file_total = file_sizes[i];

        on_progress
            .send(ProgressEvent {
                current: i,
                total,
                percent: if total_bytes > 0 { ((downloaded_bytes as f64 / total_bytes as f64) * 100.0) as u32 } else { ((i as f64 / total as f64) * 100.0) as u32 },
                key: key.clone(),
                status: "downloading".into(),
                downloaded_bytes,
                total_bytes,
            })
            .map_err(|e| e.to_string())?;

        let (data, ext, actual_size) = match download::download_image_streaming(
            &img_client,
            &item.original_url,
            |file_downloaded, _file_total| {
                let _ = on_progress.send(ProgressEvent {
                    current: i,
                    total,
                    percent: if total_bytes > 0 {
                        ((downloaded_bytes + file_downloaded) as f64 / total_bytes as f64 * 100.0) as u32
                    } else {
                        ((i as f64 + file_downloaded as f64 / 1u64.max(file_total) as f64) / total as f64 * 100.0) as u32
                    },
                    key: key.clone(),
                    status: "downloading".into(),
                    downloaded_bytes: downloaded_bytes + file_downloaded,
                    total_bytes,
                });
            },
        ).await {
            Ok(d) => d,
            Err(e) => {
                errors.push(format!("{key}: {e}"));
                continue;
            }
        };

        downloaded_bytes += actual_size;

        if ext == "gif" {
            errors.push(format!("{key}: GIF skipped (v1 limitation)"));
            continue;
        }

        on_progress
            .send(ProgressEvent {
                current: i,
                total,
                percent: if total_bytes > 0 { ((downloaded_bytes as f64 / total_bytes as f64) * 100.0) as u32 } else { ((i as f64 / total as f64) * 100.0) as u32 },
                key: key.clone(),
                status: "saving".into(),
                downloaded_bytes,
                total_bytes,
            })
            .map_err(|e| e.to_string())?;

        let mut save_path_buf = std::path::Path::new(&settings.save_dir).to_path_buf();
        if let Some(ref subdir) = save_subdir {
            save_path_buf.push(subdir);
        }
        let save_dir = &save_path_buf;
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
                    percent: if total_bytes > 0 { ((downloaded_bytes as f64 / total_bytes as f64) * 100.0) as u32 } else { ((i as f64 / total as f64) * 100.0) as u32 },
                    key: key.clone(),
                    status: "compressing".into(),
                    downloaded_bytes,
                    total_bytes,
                })
                .map_err(|e| e.to_string())?;

            let compressed = compress::compress(&data, settings.compress_max_mb);
            let compress_dir = if settings.compress_separate {
                let mut path_buf = std::path::Path::new(&settings.compress_dir).to_path_buf();
                if let Some(ref subdir) = compress_subdir {
                    path_buf.push(subdir);
                }
                path_buf
            } else {
                save_path_buf.clone()
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
                percent: if total_bytes > 0 { ((downloaded_bytes as f64 / total_bytes as f64) * 100.0) as u32 } else { ((i as f64 / total as f64) * 100.0) as u32 },
                key: key.clone(),
                status: "bookmark".into(),
                downloaded_bytes,
                total_bytes,
            })
            .map_err(|e| e.to_string())?;

        let _ = bookmark_add(&state, &token, item.illust_id).await;

        if i < total - 1 {
            tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
        }

        on_progress
            .send(ProgressEvent {
                current: i + 1,
                total,
                percent: if total_bytes > 0 { ((downloaded_bytes as f64 / total_bytes as f64) * 100.0) as u32 } else { ((i + 1) as f64 / total as f64 * 100.0) as u32 },
                key: key.clone(),
                status: "done".into(),
                downloaded_bytes,
                total_bytes,
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

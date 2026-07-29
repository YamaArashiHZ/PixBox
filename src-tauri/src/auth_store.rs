use serde::{Deserialize, Serialize};
use tauri::AppHandle;
use tauri::Manager;

const SERVICE: &str = "com.pixbox.app";
const REFRESH_TOKEN_ENTRY: &str = "pixiv-refresh-token";
const PHPSESSID_ENTRY: &str = "pixiv-phpsessid";
const METADATA_FILE: &str = "tokens.json";
const CREDENTIAL_VERSION: u32 = 1;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PixivUser {
    pub id: String,
    pub name: String,
    pub account: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AuthMetadata {
    pub user: PixivUser,
    #[serde(default)]
    pub credential_version: u32,
}

pub struct AuthSecrets {
    pub refresh_token: String,
    pub phpsessid: Option<String>,
}

#[derive(Debug)]
pub enum AuthStoreError {
    Keyring(String),
    Io(String),
    Serde(String),
    Migration(String),
    IncompleteCredentials(String),
}

impl std::fmt::Display for AuthStoreError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AuthStoreError::Keyring(e) => write!(f, "安全存储错误: {e}"),
            AuthStoreError::Io(e) => write!(f, "文件读写错误: {e}"),
            AuthStoreError::Serde(e) => write!(f, "数据解析错误: {e}"),
            AuthStoreError::Migration(e) => write!(f, "凭据迁移失败，登录数据已保留: {e}"),
            AuthStoreError::IncompleteCredentials(e) => write!(f, "凭据不完整: {e}"),
        }
    }
}

fn metadata_path(app: &AppHandle) -> Result<std::path::PathBuf, AuthStoreError> {
    app.path()
        .app_data_dir()
        .map(|d| d.join(METADATA_FILE))
        .map_err(|e| AuthStoreError::Io(e.to_string()))
}

fn make_entry(account: &str) -> Result<keyring::Entry, AuthStoreError> {
    keyring::Entry::new(SERVICE, account)
        .map_err(|e| AuthStoreError::Keyring(format!("系统安全存储不可用: {e}")))
}

// ── Read/write individual secrets ──

fn read_refresh_token() -> Result<Option<String>, AuthStoreError> {
    let entry = make_entry(REFRESH_TOKEN_ENTRY)?;
    match entry.get_password() {
        Ok(pw) => Ok(Some(pw)),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(e) => Err(AuthStoreError::Keyring(format!("读取 refresh token 失败: {e}"))),
    }
}

fn set_refresh_token(token: &str) -> Result<(), AuthStoreError> {
    let entry = make_entry(REFRESH_TOKEN_ENTRY)?;
    entry
        .set_password(token)
        .map_err(|e| AuthStoreError::Keyring(format!("保存 refresh token 失败: {e}")))
}

fn delete_refresh_token() -> Result<(), AuthStoreError> {
    let entry = make_entry(REFRESH_TOKEN_ENTRY)?;
    match entry.delete_credential() {
        Ok(()) => Ok(()),
        Err(keyring::Error::NoEntry) => Ok(()),
        Err(e) => Err(AuthStoreError::Keyring(format!("删除 refresh token 失败: {e}"))),
    }
}

fn read_phpsessid() -> Result<Option<String>, AuthStoreError> {
    let entry = make_entry(PHPSESSID_ENTRY)?;
    match entry.get_password() {
        Ok(pw) => Ok(Some(pw)),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(e) => Err(AuthStoreError::Keyring(format!("读取 PHPSESSID 失败: {e}"))),
    }
}

fn set_phpsessid(sid: &str) -> Result<(), AuthStoreError> {
    let entry = make_entry(PHPSESSID_ENTRY)?;
    entry
        .set_password(sid)
        .map_err(|e| AuthStoreError::Keyring(format!("保存 PHPSESSID 失败: {e}")))
}

fn delete_phpsessid() -> Result<(), AuthStoreError> {
    let entry = make_entry(PHPSESSID_ENTRY)?;
    match entry.delete_credential() {
        Ok(()) => Ok(()),
        Err(keyring::Error::NoEntry) => Ok(()),
        Err(e) => Err(AuthStoreError::Keyring(format!("删除 PHPSESSID 失败: {e}"))),
    }
}

// ── Metadata file ──

pub fn load_metadata(app: &AppHandle) -> Result<Option<AuthMetadata>, AuthStoreError> {
    let path = metadata_path(app)?;
    if !path.exists() {
        return Ok(None);
    }
    let data =
        std::fs::read_to_string(&path).map_err(|e| AuthStoreError::Io(e.to_string()))?;
    let meta: AuthMetadata =
        serde_json::from_str(&data).map_err(|e| AuthStoreError::Serde(e.to_string()))?;
    Ok(Some(meta))
}

fn save_metadata(app: &AppHandle, meta: &AuthMetadata) -> Result<(), AuthStoreError> {
    let path = metadata_path(app)?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| AuthStoreError::Io(e.to_string()))?;
    }
    let json =
        serde_json::to_string_pretty(meta).map_err(|e| AuthStoreError::Serde(e.to_string()))?;
    let tmp_path = path.with_extension("tmp");
    std::fs::write(&tmp_path, &json).map_err(|e| AuthStoreError::Io(e.to_string()))?;
    std::fs::rename(&tmp_path, &path).map_err(|e| AuthStoreError::Io(e.to_string()))
}

fn delete_metadata(app: &AppHandle) -> Result<(), AuthStoreError> {
    let path = metadata_path(app)?;
    if path.exists() {
        std::fs::remove_file(&path).map_err(|e| AuthStoreError::Io(e.to_string()))?;
    }
    Ok(())
}

// ── Public API ──

/// 加载完整登录状态（元数据 + 密钥库凭据）。
/// 如果检测到旧格式 tokens.json，返回 Migration 错误以触发迁移。
pub fn load_auth(
    app: &AppHandle,
) -> Result<Option<(PixivUser, AuthSecrets)>, AuthStoreError> {
    let path = metadata_path(app)?;
    if path.exists() {
        let raw =
            std::fs::read_to_string(&path).map_err(|e| AuthStoreError::Io(e.to_string()))?;
        let value: serde_json::Value =
            serde_json::from_str(&raw).map_err(|e| AuthStoreError::Serde(e.to_string()))?;
        if value.get("refresh_token").is_some() || value.get("phpsessid").is_some() {
            return Err(AuthStoreError::Migration(
                "检测到旧版凭据格式，需要先执行迁移".into(),
            ));
        }
    }

    let meta = match load_metadata(app)? {
        Some(m) => m,
        None => {
            let rt_exists = read_refresh_token()?.is_some();
            let sid_exists = read_phpsessid()?.is_some();
            if rt_exists || sid_exists {
                return Err(AuthStoreError::IncompleteCredentials(
                    "系统密钥库中存在残留凭据但缺少登录元数据".into(),
                ));
            }
            return Ok(None);
        }
    };

    let refresh_token = read_refresh_token()?
        .ok_or_else(|| AuthStoreError::IncompleteCredentials("refresh token 缺失".into()))?;
    let phpsessid = read_phpsessid()?;
    Ok(Some((
        meta.user,
        AuthSecrets {
            refresh_token,
            phpsessid,
        },
    )))
}

/// 保存登录状态：先写密钥库，再原子写元数据。
/// 元数据写入失败时回滚密钥库写入。
pub fn save_auth(
    app: &AppHandle,
    user: PixivUser,
    secrets: &AuthSecrets,
) -> Result<(), AuthStoreError> {
    set_refresh_token(&secrets.refresh_token)?;
    if let Some(ref sid) = secrets.phpsessid {
        set_phpsessid(sid)?;
    } else {
        delete_phpsessid().ok();
    }

    let meta = AuthMetadata {
        user,
        credential_version: CREDENTIAL_VERSION,
    };
    match save_metadata(app, &meta) {
        Ok(()) => Ok(()),
        Err(e) => {
            delete_refresh_token().ok();
            delete_phpsessid().ok();
            Err(e)
        }
    }
}

/// 仅在 token 刷新成功后更新 refresh token。
pub fn update_refresh_token(refresh_token: &str) -> Result<(), AuthStoreError> {
    set_refresh_token(refresh_token)
}

/// 更新或删除 PHPSESSID。
pub fn update_phpsessid(phpsessid: Option<&str>) -> Result<(), AuthStoreError> {
    match phpsessid {
        Some(sid) => set_phpsessid(sid),
        None => delete_phpsessid(),
    }
}

/// 删除全部凭据和元数据。收集所有错误一并返回。
pub fn delete_auth(app: &AppHandle) -> Result<(), String> {
    let mut errors: Vec<String> = Vec::new();

    if let Err(e) = delete_refresh_token() {
        errors.push(format!("refresh token: {e}"));
    }
    if let Err(e) = delete_phpsessid() {
        errors.push(format!("PHPSESSID: {e}"));
    }
    if let Err(e) = delete_metadata(app) {
        errors.push(format!("登录数据: {e}"));
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors.join("; "))
    }
}

// ── 旧数据迁移 ──

#[derive(Debug, Serialize, Deserialize)]
struct LegacyTokenData {
    refresh_token: String,
    user: PixivUser,
    #[serde(default)]
    phpsessid: Option<String>,
}

pub enum MigrationOutcome {
    AlreadyCurrent,
    Migrated,
    NotNeeded,
}

/// 一次性幂等迁移：旧 tokens.json → 密钥库 + 新元数据文件。
/// 失败时旧文件保持原样，不产生 .bak 且不删除凭据。
pub fn migrate_legacy_auth(app: &AppHandle) -> Result<MigrationOutcome, AuthStoreError> {
    let path = metadata_path(app)?;
    if !path.exists() {
        return Ok(MigrationOutcome::NotNeeded);
    }

    let raw =
        std::fs::read_to_string(&path).map_err(|e| AuthStoreError::Io(e.to_string()))?;
    let value: serde_json::Value =
        serde_json::from_str(&raw).map_err(|e| AuthStoreError::Serde(e.to_string()))?;

    if value.get("refresh_token").is_none() && value.get("phpsessid").is_none() {
        return Ok(MigrationOutcome::AlreadyCurrent);
    }

    let legacy: LegacyTokenData = serde_json::from_str(&raw)
        .map_err(|e| AuthStoreError::Serde(format!("无法解析旧登录数据: {e}")))?;

    // 1. 写 refresh token 到密钥库
    set_refresh_token(&legacy.refresh_token)?;

    // 2. 写 PHPSESSID（若有）
    if let Some(ref sid) = legacy.phpsessid {
        if let Err(e) = set_phpsessid(sid) {
            delete_refresh_token().ok();
            return Err(e);
        }
    }

    // 3. 验证密钥库写入
    let verify_rt = read_refresh_token()?;
    if verify_rt.as_deref() != Some(&legacy.refresh_token) {
        delete_refresh_token().ok();
        delete_phpsessid().ok();
        return Err(AuthStoreError::Migration("refresh token 验证失败".into()));
    }
    if let Some(ref sid) = legacy.phpsessid {
        let verify_sid = read_phpsessid()?;
        if verify_sid.as_deref() != Some(sid.as_str()) {
            delete_refresh_token().ok();
            delete_phpsessid().ok();
            return Err(AuthStoreError::Migration("PHPSESSID 验证失败".into()));
        }
    }

    // 4. 原子写入新元数据
    let meta = AuthMetadata {
        user: legacy.user,
        credential_version: CREDENTIAL_VERSION,
    };
    save_metadata(app, &meta)?;

    Ok(MigrationOutcome::Migrated)
}

/// 检查 tokens.json 是否为旧格式（含 refresh_token 或 phpsessid）。
pub fn is_legacy_format(app: &AppHandle) -> bool {
    let Ok(path) = metadata_path(app) else {
        return false;
    };
    if !path.exists() {
        return false;
    }
    let Ok(raw) = std::fs::read_to_string(&path) else {
        return false;
    };
    let Ok(value) = serde_json::from_str::<serde_json::Value>(&raw) else {
        return false;
    };
    value.get("refresh_token").is_some() || value.get("phpsessid").is_some()
}

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::{
    borrow::Cow,
    collections::VecDeque,
    io::Cursor,
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    time::Duration,
};

use anyhow::{anyhow, Context, Result};
use arboard::Clipboard;
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use chrono::Utc;
use regex::Regex;
use reqwest::Client;
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sha2::{Digest, Sha256};
use tauri::{
    AppHandle, CustomMenuItem, GlobalShortcutManager, Manager, State, SystemTray, SystemTrayEvent,
    SystemTrayMenu,
};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ClientSettings {
    api_base: String,
    token: String,
    device_id: String,
    device_name: String,
    device_fingerprint: String,
    #[serde(default)]
    account_name: String,
    #[serde(default)]
    offline_mode: bool,
    poll_interval_ms: u64,
    paused: bool,
    privacy_mode: String,
    app_rules: Vec<String>,
    mask_rules: Vec<String>,
    webhook_urls: Vec<String>,
    global_shortcut: String,
}

impl Default for ClientSettings {
    fn default() -> Self {
        Self {
            api_base: "https://paste-api.dangolabs.top".to_string(),
            token: String::new(),
            device_id: String::new(),
            device_name: hostname(),
            device_fingerprint: Uuid::new_v4().to_string(),
            account_name: String::new(),
            offline_mode: false,
            poll_interval_ms: 800,
            paused: false,
            privacy_mode: "off".to_string(),
            app_rules: vec![],
            mask_rules: vec![
                r"(\d{3})\d{4}(\d{4})".to_string(),
                r"\b\d{16,19}\b".to_string(),
                r"(?i)(token|secret|password)\s*[:=]\s*\S+".to_string(),
            ],
            webhook_urls: vec![],
            global_shortcut: "CommandOrControl+Shift+V".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
struct LocalItem {
    id: String,
    item_type: String,
    content: Option<String>,
    file_path: Option<String>,
    object_path: Option<String>,
    mime_type: Option<String>,
    source_app: Option<String>,
    synced: bool,
    created_at: String,
}

struct AppState {
    db_path: PathBuf,
    data_dir: PathBuf,
    settings: Mutex<ClientSettings>,
    paused: AtomicBool,
    suppressed_hashes: Mutex<VecDeque<String>>,
    registered_shortcut: Mutex<Option<String>>,
}

fn main() {
    let tray_menu = SystemTrayMenu::new()
        .add_item(CustomMenuItem::new("open".to_string(), "打开主界面"))
        .add_item(CustomMenuItem::new("pause".to_string(), "暂停监听"))
        .add_item(CustomMenuItem::new("sync".to_string(), "手动同步"))
        .add_item(CustomMenuItem::new("prefs".to_string(), "偏好设置"))
        .add_item(CustomMenuItem::new("quit".to_string(), "退出"));

    tauri::Builder::default()
        .system_tray(SystemTray::new().with_menu(tray_menu))
        .setup(|app| {
            let data_dir = app_data_dir()?;
            std::fs::create_dir_all(data_dir.join("images"))?;
            let db_path = data_dir.join("web-paste.sqlite3");
            init_db(&db_path)?;
            let settings = load_settings(&db_path)?;
            let global_shortcut = settings.global_shortcut.clone();
            let state = Arc::new(AppState {
                db_path,
                data_dir,
                paused: AtomicBool::new(settings.paused),
                settings: Mutex::new(settings),
                suppressed_hashes: Mutex::new(VecDeque::new()),
                registered_shortcut: Mutex::new(None),
            });
            app.manage(state.clone());
            if let Err(err) = register_shortcut(&app.handle(), &state, &global_shortcut) {
                eprintln!("register shortcut failed: {err}");
            }
            tauri::async_runtime::spawn(clipboard_loop(state.clone(), app.handle()));
            tauri::async_runtime::spawn(sync_loop(state, app.handle()));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_history,
            get_image_preview_data_url,
            get_client_settings,
            login_with_password,
            use_offline_mode,
            logout_client,
            save_client_settings,
            set_paused,
            force_sync,
            recopy_item,
            clear_history
        ])
        .on_window_event(|event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event.event() {
                api.prevent_close();
                let _ = event.window().hide();
            }
        })
        .on_system_tray_event(handle_tray_event)
        .run(tauri::generate_context!())
        .expect("error while running Web Paste");
}

#[tauri::command]
fn get_client_settings(state: State<'_, Arc<AppState>>) -> Result<ClientSettings, String> {
    Ok(state.settings.lock().map_err(to_string)?.clone())
}

#[derive(Debug, Deserialize)]
struct AuthResponse {
    token: String,
    user: AuthUser,
    device: Option<AuthDevice>,
}

#[derive(Debug, Deserialize)]
struct AuthUser {
    email: String,
}

#[derive(Debug, Deserialize)]
struct AuthDevice {
    id: String,
}

#[tauri::command]
async fn login_with_password(
    state: State<'_, Arc<AppState>>,
    api_base: String,
    username: String,
    password: String,
) -> Result<ClientSettings, String> {
    let api_base = api_base.trim().trim_end_matches('/').to_string();
    let username = username.trim().to_string();
    if api_base.is_empty() {
        return Err("服务端地址不能为空".to_string());
    }
    if username.is_empty() || password.is_empty() {
        return Err("用户名和密码不能为空".to_string());
    }

    let current = state.settings.lock().map_err(to_string)?.clone();
    let device_name = first_non_empty(&current.device_name, &hostname());
    let fingerprint = first_non_empty(&current.device_fingerprint, &Uuid::new_v4().to_string());
    let client = Client::new();
    let res = client
        .post(format!("{api_base}/api/auth/login"))
        .json(&json!({
            "email": username,
            "password": password,
            "device": {
                "fingerprint": fingerprint,
                "name": device_name,
                "platform": std::env::consts::OS
            }
        }))
        .send()
        .await
        .map_err(to_string)?;
    let status = res.status();
    let body = res.text().await.map_err(to_string)?;
    if !status.is_success() {
        return Err(error_message_from_body(&body, "登录失败"));
    }
    let auth: AuthResponse = serde_json::from_str(&body).map_err(to_string)?;
    let device_id = auth
        .device
        .map(|device| device.id)
        .ok_or_else(|| "服务端没有返回设备 ID".to_string())?;

    let mut next = current;
    next.api_base = api_base;
    next.token = auth.token;
    next.device_id = device_id;
    next.device_name = device_name;
    next.device_fingerprint = fingerprint;
    next.account_name = auth.user.email;
    next.offline_mode = false;
    save_settings_to_db(&state.db_path, &next).map_err(to_string)?;
    state.paused.store(next.paused, Ordering::Relaxed);
    *state.settings.lock().map_err(to_string)? = next.clone();
    Ok(next)
}

#[tauri::command]
fn use_offline_mode(state: State<'_, Arc<AppState>>) -> Result<ClientSettings, String> {
    let mut next = state.settings.lock().map_err(to_string)?.clone();
    next.offline_mode = true;
    next.token.clear();
    next.device_id.clear();
    next.account_name.clear();
    save_settings_to_db(&state.db_path, &next).map_err(to_string)?;
    *state.settings.lock().map_err(to_string)? = next.clone();
    Ok(next)
}

#[tauri::command]
fn logout_client(state: State<'_, Arc<AppState>>) -> Result<ClientSettings, String> {
    let mut next = state.settings.lock().map_err(to_string)?.clone();
    next.offline_mode = false;
    next.token.clear();
    next.device_id.clear();
    next.account_name.clear();
    save_settings_to_db(&state.db_path, &next).map_err(to_string)?;
    *state.settings.lock().map_err(to_string)? = next.clone();
    Ok(next)
}

#[tauri::command]
fn save_client_settings(
    app: AppHandle,
    state: State<'_, Arc<AppState>>,
    mut next: ClientSettings,
) -> Result<(), String> {
    next.global_shortcut = first_non_empty(&next.global_shortcut, "CommandOrControl+Shift+V");
    register_shortcut(&app, state.inner(), &next.global_shortcut).map_err(to_string)?;
    save_settings_to_db(&state.db_path, &next).map_err(to_string)?;
    state.paused.store(next.paused, Ordering::Relaxed);
    *state.settings.lock().map_err(to_string)? = next;
    Ok(())
}

#[tauri::command]
fn set_paused(state: State<'_, Arc<AppState>>, paused: bool) -> Result<(), String> {
    set_paused_inner(&state, paused).map_err(to_string)
}

#[tauri::command]
async fn force_sync(state: State<'_, Arc<AppState>>) -> Result<bool, String> {
    force_sync_inner(state.inner().clone()).await.map_err(to_string)
}

#[tauri::command]
fn get_history(
    state: State<'_, Arc<AppState>>,
    query: String,
    limit: i64,
    offset: i64,
) -> Result<Vec<LocalItem>, String> {
    let conn = Connection::open(&state.db_path).map_err(to_string)?;
    let like = format!("%{}%", query.trim());
    let mut stmt = conn
        .prepare(
            r#"
            SELECT id, item_type, content, file_path, object_path, mime_type, source_app, synced, created_at
            FROM local_items
            WHERE ?1 = '%%'
               OR COALESCE(content, '') LIKE ?1
               OR COALESCE(file_path, '') LIKE ?1
               OR COALESCE(source_app, '') LIKE ?1
            ORDER BY datetime(created_at) DESC
            LIMIT ?2 OFFSET ?3
            "#,
        )
        .map_err(to_string)?;
    let rows = stmt
        .query_map(params![like, limit.clamp(1, 200), offset.max(0)], |row| {
            Ok(LocalItem {
                id: row.get(0)?,
                item_type: row.get(1)?,
                content: row.get(2)?,
                file_path: row.get(3)?,
                object_path: row.get(4)?,
                mime_type: row.get(5)?,
                source_app: row.get(6)?,
                synced: row.get::<_, i64>(7)? == 1,
                created_at: row.get(8)?,
            })
        })
        .map_err(to_string)?;

    let mut out = Vec::new();
    for row in rows {
        out.push(row.map_err(to_string)?);
    }
    Ok(out)
}

#[tauri::command]
fn get_image_preview_data_url(state: State<'_, Arc<AppState>>, id: String) -> Result<String, String> {
    let conn = Connection::open(&state.db_path).map_err(to_string)?;
    let (item_type, object_path, mime_type) = conn
        .query_row(
            "SELECT item_type, object_path, mime_type FROM local_items WHERE id = ?1",
            params![id],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, Option<String>>(1)?,
                    row.get::<_, Option<String>>(2)?,
                ))
            },
        )
        .map_err(to_string)?;
    if item_type != "image" {
        return Err("record is not an image".to_string());
    }

    let path = PathBuf::from(object_path.ok_or_else(|| "image path is empty".to_string())?);
    let canonical = path.canonicalize().map_err(to_string)?;
    let images_dir = state.data_dir.join("images").canonicalize().map_err(to_string)?;
    if !canonical.starts_with(images_dir) {
        return Err("image path is outside app data directory".to_string());
    }
    let bytes = std::fs::read(canonical).map_err(to_string)?;
    let mut mime = first_non_empty(mime_type.as_deref().unwrap_or(""), "image/png");
    let preview = match image::load_from_memory(&bytes) {
        Ok(image) => {
            let thumb = image.thumbnail(320, 240);
            let mut out = Cursor::new(Vec::new());
            thumb.write_to(&mut out, image::ImageFormat::Png)
                .map_err(to_string)?;
            mime = "image/png".to_string();
            out.into_inner()
        }
        Err(_) => bytes,
    };
    Ok(format!("data:{mime};base64,{}", BASE64.encode(preview)))
}

#[tauri::command]
fn recopy_item(state: State<'_, Arc<AppState>>, id: String) -> Result<(), String> {
    let conn = Connection::open(&state.db_path).map_err(to_string)?;
    let item = conn
        .query_row(
            "SELECT item_type, content, file_path, object_path FROM local_items WHERE id = ?1",
            params![id],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, Option<String>>(1)?,
                    row.get::<_, Option<String>>(2)?,
                    row.get::<_, Option<String>>(3)?,
                ))
            },
        )
        .map_err(to_string)?;
    let mut clipboard = Clipboard::new().map_err(to_string)?;
    match item.0.as_str() {
        "image" => {
            let path = item.3.ok_or_else(|| "image path is empty".to_string())?;
            let image = image::open(path).map_err(to_string)?.to_rgba8();
            let (width, height) = image.dimensions();
            let bytes = image.into_raw();
            suppress_clipboard_hash(state.inner(), hash_bytes(&bytes)).map_err(to_string)?;
            clipboard
                .set_image(arboard::ImageData {
                    width: width as usize,
                    height: height as usize,
                    bytes: Cow::Owned(bytes),
                })
                .map_err(to_string)?;
        }
        "file_path" => {
            let value = item.2.unwrap_or_default();
            suppress_clipboard_hash(state.inner(), hash_bytes(value.as_bytes())).map_err(to_string)?;
            clipboard.set_text(value).map_err(to_string)?;
        }
        _ => {
            let value = item.1.unwrap_or_default();
            suppress_clipboard_hash(state.inner(), hash_bytes(value.as_bytes())).map_err(to_string)?;
            clipboard.set_text(value).map_err(to_string)?;
        }
    }
    Ok(())
}

#[tauri::command]
fn clear_history(state: State<'_, Arc<AppState>>) -> Result<(), String> {
    let conn = Connection::open(&state.db_path).map_err(to_string)?;
    conn.execute("DELETE FROM local_items", []).map_err(to_string)?;
    let images_dir = state.data_dir.join("images");
    if images_dir.exists() {
        std::fs::remove_dir_all(&images_dir).map_err(to_string)?;
    }
    std::fs::create_dir_all(images_dir).map_err(to_string)?;
    Ok(())
}

async fn clipboard_loop(state: Arc<AppState>, app: AppHandle) {
    let mut clipboard = match Clipboard::new() {
        Ok(value) => value,
        Err(err) => {
            eprintln!("clipboard init failed: {err}");
            return;
        }
    };
    let mut last_hash = String::new();
    loop {
        let settings = state.settings.lock().map(|value| value.clone()).unwrap_or_default();
        if state.paused.load(Ordering::Relaxed) || !app_allowed(&settings) {
            if let Some(hash) = current_clipboard_hash(&mut clipboard) {
                last_hash = hash.clone();
                let _ = take_suppressed_hash(&state, &hash);
            }
        } else {
            let mut handled_file_list = false;
            #[cfg(target_os = "windows")]
            {
                if let Some(paths) = clipboard_file_paths() {
                    handled_file_list = true;
                    let hash = format!("file:{}", hash_bytes(paths.as_bytes()));
                    if hash != last_hash {
                        last_hash = hash;
                        if !take_suppressed_hash(&state, &last_hash) {
                            if let Err(err) = persist_file_paths(&state, &settings, paths).await {
                                eprintln!("persist file paths failed: {err}");
                            } else {
                                emit_history_changed(&app);
                            }
                        }
                    }
                }
            }

            if !handled_file_list {
                if let Ok(text) = clipboard.get_text() {
                    let hash = hash_bytes(text.as_bytes());
                    if !text.trim().is_empty() && hash != last_hash {
                        last_hash = hash;
                        if !take_suppressed_hash(&state, &last_hash) {
                            if let Err(err) = persist_text(&state, &settings, text).await {
                                eprintln!("persist text failed: {err}");
                            } else {
                                emit_history_changed(&app);
                            }
                        }
                    }
                }
                if let Ok(image) = clipboard.get_image() {
                    let hash = hash_bytes(&image.bytes);
                    if hash != last_hash {
                        last_hash = hash;
                        if !take_suppressed_hash(&state, &last_hash) {
                            if let Err(err) = persist_image(&state, &settings, image).await {
                                eprintln!("persist image failed: {err}");
                            } else {
                                emit_history_changed(&app);
                            }
                        }
                    }
                }
            }
        }
        tokio::time::sleep(Duration::from_millis(settings.poll_interval_ms.max(300))).await;
    }
}

fn current_clipboard_hash(clipboard: &mut Clipboard) -> Option<String> {
    #[cfg(target_os = "windows")]
    {
        if let Some(paths) = clipboard_file_paths() {
            return Some(format!("file:{}", hash_bytes(paths.as_bytes())));
        }
    }
    if let Ok(text) = clipboard.get_text() {
        if !text.trim().is_empty() {
            return Some(hash_bytes(text.as_bytes()));
        }
    }
    if let Ok(image) = clipboard.get_image() {
        return Some(hash_bytes(&image.bytes));
    }
    None
}

async fn sync_loop(state: Arc<AppState>, app: AppHandle) {
    loop {
        let pending_before = unsynced_count(&state.db_path).unwrap_or(0);
        if let Err(err) = force_sync_inner(state.clone()).await {
            eprintln!("sync failed: {err}");
        } else if pending_before > 0 {
            emit_history_changed(&app);
        }
        tokio::time::sleep(Duration::from_secs(10)).await;
    }
}

async fn persist_text(state: &Arc<AppState>, settings: &ClientSettings, text: String) -> Result<()> {
    let source_app = active_process_name();
    let (content, masked) = apply_masks(&text, &settings.mask_rules)?;
    let (item_type, content, file_path) = if let Some(path) = detect_file_path(&content) {
        ("file_path".to_string(), None, Some(path))
    } else {
        ("text".to_string(), Some(content), None)
    };
    let item = insert_local_item(
        state,
        item_type,
        content,
        file_path,
        None,
        None,
        source_app,
        masked,
    )?;
    trigger_webhooks(settings, &item).await;
    let _ = force_sync_inner(state.clone()).await;
    Ok(())
}

async fn persist_file_paths(
    state: &Arc<AppState>,
    settings: &ClientSettings,
    file_path: String,
) -> Result<()> {
    let item = insert_local_item(
        state,
        "file_path".to_string(),
        None,
        Some(file_path),
        None,
        None,
        active_process_name(),
        false,
    )?;
    trigger_webhooks(settings, &item).await;
    let _ = force_sync_inner(state.clone()).await;
    Ok(())
}

async fn persist_image(
    state: &Arc<AppState>,
    settings: &ClientSettings,
    image: arboard::ImageData<'_>,
) -> Result<()> {
    let id = Uuid::new_v4().to_string();
    let path = state.data_dir.join("images").join(format!("{id}.png"));
    let buffer = image::ImageBuffer::<image::Rgba<u8>, _>::from_raw(
        image.width as u32,
        image.height as u32,
        image.bytes.into_owned(),
    )
    .ok_or_else(|| anyhow!("invalid image buffer"))?;
    buffer.save(&path)?;
    let item = insert_local_item(
        state,
        "image".to_string(),
        None,
        None,
        Some(path.to_string_lossy().to_string()),
        Some("image/png".to_string()),
        active_process_name(),
        false,
    )?;
    trigger_webhooks(settings, &item).await;
    let _ = force_sync_inner(state.clone()).await;
    Ok(())
}

fn insert_local_item(
    state: &Arc<AppState>,
    item_type: String,
    content: Option<String>,
    file_path: Option<String>,
    object_path: Option<String>,
    mime_type: Option<String>,
    source_app: Option<String>,
    masked: bool,
) -> Result<LocalItem> {
    let id = Uuid::new_v4().to_string();
    let created_at = Utc::now().to_rfc3339();
    let conn = Connection::open(&state.db_path)?;
    conn.execute(
        r#"
        INSERT INTO local_items (
          id, item_type, content, file_path, object_path, mime_type, source_app, masked, synced, created_at
        )
        VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, 0, ?9)
        "#,
        params![
            id,
            item_type,
            content,
            file_path,
            object_path,
            mime_type,
            source_app,
            if masked { 1 } else { 0 },
            created_at
        ],
    )?;
    Ok(LocalItem {
        id,
        item_type,
        content,
        file_path,
        object_path,
        mime_type,
        source_app,
        synced: false,
        created_at,
    })
}

async fn force_sync_inner(state: Arc<AppState>) -> Result<bool> {
    let settings = state.settings.lock().map_err(|_| anyhow!("settings lock poisoned"))?.clone();
    if settings.offline_mode
        || settings.token.trim().is_empty()
        || settings.api_base.trim().is_empty()
        || settings.device_id.trim().is_empty()
    {
        return Ok(false);
    }

    let pending = pending_items(&state.db_path, 50)?;
    if pending.is_empty() {
        return Ok(true);
    }

    let mut records = Vec::new();
    for item in pending {
        let mut record = json!({
            "id": item.id,
            "device_id": settings.device_id,
            "type": item.item_type,
            "content": item.content,
            "file_path": item.file_path,
            "mime_type": item.mime_type,
            "source_app": item.source_app,
            "client_created_at": item.created_at
        });
        if item.item_type == "image" {
            let path = item.object_path.as_ref().context("image path is empty")?;
            let bytes = std::fs::read(path)?;
            record["image_base64"] = json!(format!(
                "data:{};base64,{}",
                item.mime_type.clone().unwrap_or_else(|| "image/png".to_string()),
                BASE64.encode(bytes)
            ));
        }
        records.push(record);
    }

    let client = Client::new();
    let url = format!("{}/api/sync/batch", settings.api_base.trim_end_matches('/'));
    let res = client
        .post(url)
        .bearer_auth(settings.token)
        .json(&json!({ "records": records }))
        .send()
        .await?;
    if !res.status().is_success() {
        return Ok(false);
    }
    let body: serde_json::Value = res.json().await?;
    let ids: Vec<String> = body
        .get("items")
        .and_then(|items| items.as_array())
        .map(|items| {
            items
                .iter()
                .filter_map(|item| item.get("id").and_then(|id| id.as_str()).map(str::to_string))
                .collect()
        })
        .unwrap_or_default();
    mark_synced(&state.db_path, &ids)?;
    Ok(true)
}

fn pending_items(db_path: &Path, limit: i64) -> Result<Vec<LocalItem>> {
    let conn = Connection::open(db_path)?;
    let mut stmt = conn.prepare(
        r#"
        SELECT id, item_type, content, file_path, object_path, mime_type, source_app, synced, created_at
        FROM local_items
        WHERE synced = 0
        ORDER BY datetime(created_at) ASC
        LIMIT ?1
        "#,
    )?;
    let rows = stmt.query_map(params![limit], |row| {
        Ok(LocalItem {
            id: row.get(0)?,
            item_type: row.get(1)?,
            content: row.get(2)?,
            file_path: row.get(3)?,
            object_path: row.get(4)?,
            mime_type: row.get(5)?,
            source_app: row.get(6)?,
            synced: row.get::<_, i64>(7)? == 1,
            created_at: row.get(8)?,
        })
    })?;
    let mut out = Vec::new();
    for row in rows {
        out.push(row?);
    }
    Ok(out)
}

fn mark_synced(db_path: &Path, ids: &[String]) -> Result<()> {
    if ids.is_empty() {
        return Ok(());
    }
    let conn = Connection::open(db_path)?;
    for id in ids {
        conn.execute("UPDATE local_items SET synced = 1 WHERE id = ?1", params![id])?;
    }
    Ok(())
}

fn unsynced_count(db_path: &Path) -> Result<i64> {
    let conn = Connection::open(db_path)?;
    let count = conn.query_row(
        "SELECT COUNT(*) FROM local_items WHERE synced = 0",
        [],
        |row| row.get(0),
    )?;
    Ok(count)
}

fn emit_history_changed(app: &AppHandle) {
    let _ = app.emit_all("clipboard-history-changed", json!({ "changed": true }));
}

async fn trigger_webhooks(settings: &ClientSettings, item: &LocalItem) {
    if settings.webhook_urls.is_empty() {
        return;
    }
    let client = Client::new();
    let payload = json!({
        "id": item.id,
        "type": item.item_type,
        "content": item.content,
        "file_path": item.file_path,
        "object_path": item.object_path,
        "source_app": item.source_app,
        "created_at": item.created_at
    });
    for url in &settings.webhook_urls {
        let client = client.clone();
        let url = url.clone();
        let payload = payload.clone();
        tauri::async_runtime::spawn(async move {
            let _ = client.post(url).json(&payload).send().await;
        });
    }
}

fn app_allowed(settings: &ClientSettings) -> bool {
    let mode = settings.privacy_mode.as_str();
    if mode == "off" {
        return true;
    }
    let Some(process) = active_process_name() else {
        return mode != "whitelist";
    };
    let process = process.to_lowercase();
    let matched = settings
        .app_rules
        .iter()
        .any(|rule| process.contains(&rule.to_lowercase()));
    match mode {
        "blacklist" => !matched,
        "whitelist" => matched,
        _ => true,
    }
}

fn apply_masks(input: &str, rules: &[String]) -> Result<(String, bool)> {
    let mut current = input.to_string();
    let mut masked = false;
    for rule in rules {
        let regex = Regex::new(rule)?;
        if regex.is_match(&current) {
            current = regex.replace_all(&current, "***").to_string();
            masked = true;
        }
    }
    Ok((current, masked))
}

fn detect_file_path(text: &str) -> Option<String> {
    let paths: Vec<String> = text
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(str::to_string)
        .collect();
    if paths.is_empty() || paths.len() > 50 {
        return None;
    }
    if paths.iter().all(|line| Path::new(line).is_absolute()) {
        return Some(paths.join("\n"));
    }
    None
}

fn active_process_name() -> Option<String> {
    #[cfg(target_os = "macos")]
    {
        command_output(
            "osascript",
            &[
                "-e",
                "tell application \"System Events\" to get name of first application process whose frontmost is true",
            ],
        )
    }
    #[cfg(target_os = "linux")]
    {
        let pid = command_output("xdotool", &["getwindowfocus", "getwindowpid"])?;
        command_output("ps", &["-p", pid.trim(), "-o", "comm="])
    }
    #[cfg(target_os = "windows")]
    {
        windows_active_process_name()
    }
}

#[cfg(target_os = "windows")]
fn clipboard_file_paths() -> Option<String> {
    use std::{ffi::c_void, os::windows::ffi::OsStringExt};

    type Hwnd = *mut c_void;
    type Handle = *mut c_void;
    type Hdrop = *mut c_void;

    const CF_HDROP: u32 = 15;
    const QUERY_ALL_FILES: u32 = 0xFFFF_FFFF;

    #[link(name = "user32")]
    unsafe extern "system" {
        fn OpenClipboard(hwnd: Hwnd) -> i32;
        fn CloseClipboard() -> i32;
        fn IsClipboardFormatAvailable(format: u32) -> i32;
        fn GetClipboardData(format: u32) -> Handle;
    }

    #[link(name = "shell32")]
    unsafe extern "system" {
        fn DragQueryFileW(hdrop: Hdrop, file: u32, buffer: *mut u16, buffer_len: u32) -> u32;
    }

    unsafe {
        if IsClipboardFormatAvailable(CF_HDROP) == 0 {
            return None;
        }
        if OpenClipboard(std::ptr::null_mut()) == 0 {
            return None;
        }

        let result = (|| {
            let handle = GetClipboardData(CF_HDROP);
            if handle.is_null() {
                return None;
            }

            let count = DragQueryFileW(handle as Hdrop, QUERY_ALL_FILES, std::ptr::null_mut(), 0);
            if count == 0 || count > 50 {
                return None;
            }

            let mut paths = Vec::with_capacity(count as usize);
            for index in 0..count {
                let len = DragQueryFileW(handle as Hdrop, index, std::ptr::null_mut(), 0);
                if len == 0 {
                    continue;
                }
                let mut buffer = vec![0u16; len as usize + 1];
                let written =
                    DragQueryFileW(handle as Hdrop, index, buffer.as_mut_ptr(), len + 1);
                if written == 0 {
                    continue;
                }
                let value = std::ffi::OsString::from_wide(&buffer[..written as usize])
                    .to_string_lossy()
                    .trim()
                    .to_string();
                if !value.is_empty() {
                    paths.push(value);
                }
            }

            if paths.is_empty() {
                None
            } else {
                Some(paths.join("\n"))
            }
        })();

        CloseClipboard();
        result
    }
}

#[cfg(target_os = "windows")]
fn windows_active_process_name() -> Option<String> {
    use std::ffi::c_void;

    type Hwnd = *mut c_void;
    type Handle = *mut c_void;

    const PROCESS_QUERY_LIMITED_INFORMATION: u32 = 0x1000;

    #[link(name = "user32")]
    unsafe extern "system" {
        fn GetForegroundWindow() -> Hwnd;
        fn GetWindowThreadProcessId(hwnd: Hwnd, process_id: *mut u32) -> u32;
    }

    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn OpenProcess(desired_access: u32, inherit_handle: i32, process_id: u32) -> Handle;
        fn QueryFullProcessImageNameW(
            process: Handle,
            flags: u32,
            exe_name: *mut u16,
            size: *mut u32,
        ) -> i32;
        fn CloseHandle(handle: Handle) -> i32;
    }

    unsafe {
        let hwnd = GetForegroundWindow();
        if hwnd.is_null() {
            return None;
        }

        let mut process_id = 0u32;
        GetWindowThreadProcessId(hwnd, &mut process_id);
        if process_id == 0 {
            return None;
        }

        let process = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, process_id);
        if process.is_null() {
            return None;
        }

        let mut buffer = vec![0u16; 32_768];
        let mut size = buffer.len() as u32;
        let ok = QueryFullProcessImageNameW(process, 0, buffer.as_mut_ptr(), &mut size);
        CloseHandle(process);
        if ok == 0 || size == 0 {
            return None;
        }

        let path = String::from_utf16_lossy(&buffer[..size as usize]);
        Path::new(&path)
            .file_stem()
            .or_else(|| Path::new(&path).file_name())
            .map(|name| name.to_string_lossy().to_string())
            .filter(|name| !name.trim().is_empty())
    }
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
fn command_output(cmd: &str, args: &[&str]) -> Option<String> {
    let output = std::process::Command::new(cmd).args(args).output().ok()?;
    if !output.status.success() {
        return None;
    }
    let value = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if value.is_empty() {
        None
    } else {
        Some(value)
    }
}

fn set_paused_inner(state: &Arc<AppState>, paused: bool) -> Result<()> {
    let mut settings = state.settings.lock().map_err(|_| anyhow!("settings lock poisoned"))?;
    settings.paused = paused;
    save_settings_to_db(&state.db_path, &settings)?;
    state.paused.store(paused, Ordering::Relaxed);
    Ok(())
}

fn handle_tray_event(app: &AppHandle, event: SystemTrayEvent) {
    match event {
        SystemTrayEvent::DoubleClick { .. } => open_main_window_with_view(app, "history"),
        SystemTrayEvent::MenuItemClick { id, .. } => match id.as_str() {
            "open" => open_main_window_with_view(app, "history"),
            "prefs" => open_main_window_with_view(app, "settings"),
            "pause" => {
                let state = app.state::<Arc<AppState>>();
                let paused = !state.paused.load(Ordering::Relaxed);
                if set_paused_inner(&state, paused).is_ok() {
                    let title = if paused { "恢复监听" } else { "暂停监听" };
                    let _ = app.tray_handle().get_item("pause").set_title(title);
                }
            }
            "sync" => {
                let state = app.state::<Arc<AppState>>().inner().clone();
                tauri::async_runtime::spawn(async move {
                    let _ = force_sync_inner(state).await;
                });
            }
            "quit" => app.exit(0),
            _ => {}
        },
        _ => {}
    }
}

fn open_main_window_with_view(app: &AppHandle, view: &str) {
    if let Some(window) = app.get_window("main") {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
    }
    let _ = app.emit_all("show-main", json!({ "view": view }));
}

fn toggle_main_window(app: &AppHandle) {
    if let Some(window) = app.get_window("main") {
        let visible = window.is_visible().unwrap_or(false);
        let minimized = window.is_minimized().unwrap_or(false);
        if visible && !minimized {
            let _ = window.hide();
            return;
        }
    }
    open_main_window_with_view(app, "history");
}

fn register_shortcut(app: &AppHandle, state: &Arc<AppState>, shortcut: &str) -> Result<()> {
    let shortcut = first_non_empty(shortcut, "CommandOrControl+Shift+V");
    let current = state
        .registered_shortcut
        .lock()
        .map_err(|_| anyhow!("shortcut lock poisoned"))?
        .clone();
    if current.as_deref() == Some(shortcut.as_str()) {
        return Ok(());
    }

    let callback_app = app.clone();
    app.global_shortcut_manager()
        .register(&shortcut, move || {
            toggle_main_window(&callback_app);
        })
        .with_context(|| format!("快捷键不可用或已被占用: {shortcut}"))?;

    if let Some(previous) = current {
        let _ = app.global_shortcut_manager().unregister(&previous);
    }
    *state
        .registered_shortcut
        .lock()
        .map_err(|_| anyhow!("shortcut lock poisoned"))? = Some(shortcut);
    Ok(())
}

fn init_db(db_path: &Path) -> Result<()> {
    let conn = Connection::open(db_path)?;
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS local_items (
            id text PRIMARY KEY,
            item_type text NOT NULL,
            content text,
            file_path text,
            object_path text,
            mime_type text,
            source_app text,
            masked integer NOT NULL DEFAULT 0,
            synced integer NOT NULL DEFAULT 0,
            created_at text NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_local_items_created ON local_items(datetime(created_at) DESC);
        CREATE INDEX IF NOT EXISTS idx_local_items_synced ON local_items(synced, datetime(created_at));

        CREATE TABLE IF NOT EXISTS settings (
            key text PRIMARY KEY,
            value text NOT NULL
        );
        "#,
    )?;
    Ok(())
}

fn load_settings(db_path: &Path) -> Result<ClientSettings> {
    let conn = Connection::open(db_path)?;
    let loaded: Result<String, _> = conn.query_row(
        "SELECT value FROM settings WHERE key = 'client_settings'",
        [],
        |row| row.get(0),
    );
    match loaded {
        Ok(value) => Ok(serde_json::from_str(&value)?),
        Err(_) => {
            let settings = ClientSettings::default();
            save_settings_to_db(db_path, &settings)?;
            Ok(settings)
        }
    }
}

fn save_settings_to_db(db_path: &Path, settings: &ClientSettings) -> Result<()> {
    let conn = Connection::open(db_path)?;
    let value = serde_json::to_string(settings)?;
    conn.execute(
        r#"
        INSERT INTO settings (key, value)
        VALUES ('client_settings', ?1)
        ON CONFLICT(key) DO UPDATE SET value = excluded.value
        "#,
        params![value],
    )?;
    Ok(())
}

fn app_data_dir() -> Result<PathBuf> {
    dirs::data_dir()
        .map(|dir| dir.join("web-paste"))
        .ok_or_else(|| anyhow!("could not resolve data directory"))
}

fn hash_bytes(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
}

fn suppress_clipboard_hash(state: &Arc<AppState>, hash: String) -> Result<()> {
    let mut hashes = state
        .suppressed_hashes
        .lock()
        .map_err(|_| anyhow!("suppressed hash lock poisoned"))?;
    hashes.push_back(hash);
    while hashes.len() > 16 {
        hashes.pop_front();
    }
    Ok(())
}

fn take_suppressed_hash(state: &Arc<AppState>, hash: &str) -> bool {
    let Ok(mut hashes) = state.suppressed_hashes.lock() else {
        return false;
    };
    let Some(index) = hashes.iter().position(|value| value == hash) else {
        return false;
    };
    hashes.remove(index);
    true
}

fn hostname() -> String {
    std::env::var("COMPUTERNAME")
        .or_else(|_| std::env::var("HOSTNAME"))
        .unwrap_or_else(|_| "Desktop".to_string())
}

fn first_non_empty(value: &str, fallback: &str) -> String {
    let value = value.trim();
    if value.is_empty() {
        fallback.to_string()
    } else {
        value.to_string()
    }
}

fn error_message_from_body(body: &str, fallback: &str) -> String {
    serde_json::from_str::<serde_json::Value>(body)
        .ok()
        .and_then(|value| value.get("error").and_then(|error| error.as_str()).map(str::to_string))
        .filter(|message| !message.trim().is_empty())
        .unwrap_or_else(|| fallback.to_string())
}

fn to_string<E: std::fmt::Display>(err: E) -> String {
    err.to_string()
}

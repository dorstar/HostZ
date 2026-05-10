#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use anyhow::Result;
use std::sync::{Arc, Mutex, atomic::AtomicBool};
use tauri::{Emitter, Manager};

use hostz::db::{swhdb::SwhDb, cfgdb::CfgDb};
use hostz::utils::platform;

/// 应用全局状态（Arc 包装以支持跨线程共享）
pub struct AppState {
    pub swhdb: Arc<Mutex<SwhDb>>,
    pub cfgdb: Arc<Mutex<CfgDb>>,
    pub hotkey_manager: hostz::services::hotkey::HotkeyManager,
}

// ── 基础命令 ──

#[tauri::command]
fn ping() -> String {
    "pong".to_string()
}

#[tauri::command]
fn get_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

#[tauri::command]
fn get_basic_data(state: tauri::State<AppState>) -> Result<String, String> {
    let swhdb = state.swhdb.lock().map_err(|e| e.to_string())?;
    let list = swhdb.get_list().map_err(|e| e.to_string())?;
    let trashcan = swhdb.get_trashcan().map_err(|e| e.to_string())?;
    let version = [0, 1, 0, 0u16];

    let data = hostz::models::hosts::HostsBasicData {
        list,
        trashcan,
        version,
    };
    serde_json::to_string(&data).map_err(|e| e.to_string())
}

// ── hosts 内容命令 ──

#[tauri::command]
fn get_hosts_content(state: tauri::State<AppState>, id: String) -> Result<String, String> {
    let swhdb = state.swhdb.lock().map_err(|e| e.to_string())?;
    let list = swhdb.get_list().map_err(|e| e.to_string())?;
    hostz::core::content_parser::get_content_of_hosts(&swhdb, &list, &id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn set_hosts_content(state: tauri::State<AppState>, id: String, content: String) -> Result<(), String> {
    let swhdb = state.swhdb.lock().map_err(|e| e.to_string())?;
    swhdb.set_content(&id, &content).map_err(|e| e.to_string())
}

// ── 列表管理命令 ──

#[tauri::command]
fn get_list(state: tauri::State<AppState>) -> Result<String, String> {
    let swhdb = state.swhdb.lock().map_err(|e| e.to_string())?;
    let list = swhdb.get_list().map_err(|e| e.to_string())?;
    serde_json::to_string(&list).map_err(|e| e.to_string())
}

#[tauri::command]
fn set_list(state: tauri::State<AppState>, list_json: String) -> Result<(), String> {
    let list: Vec<hostz::models::hosts::HostsListObject> =
        serde_json::from_str(&list_json).map_err(|e| e.to_string())?;
    let swhdb = state.swhdb.lock().map_err(|e| e.to_string())?;
    swhdb.set_list(&list).map_err(|e| e.to_string())
}

// ── 开关与系统应用命令 ──

#[tauri::command]
fn toggle_hosts(state: tauri::State<AppState>, id: String) -> Result<bool, String> {
    let swhdb = state.swhdb.lock().map_err(|e| e.to_string())?;
    let cfgdb = state.cfgdb.lock().map_err(|e| e.to_string())?;
    hostz::core::hosts_manager::toggle_item(&swhdb, &cfgdb, &id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn apply_to_system(state: tauri::State<AppState>) -> Result<(), String> {
    let swhdb = state.swhdb.lock().map_err(|e| e.to_string())?;
    let cfgdb = state.cfgdb.lock().map_err(|e| e.to_string())?;
    hostz::core::hosts_manager::apply_to_system(&swhdb, &cfgdb)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn get_system_hosts_content() -> Result<String, String> {
    hostz::core::hosts_manager::get_system_hosts_content()
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn set_system_hosts_content(content: String) -> Result<(), String> {
    let path = platform::system_hosts_path();
    let normalized = content.replace("\r\n", "\n");
    let normalized = if cfg!(target_os = "windows") {
        normalized.replace('\n', "\r\n")
    } else {
        normalized
    };
    hostz::core::privilege::write_system_hosts(&path, &normalized)
        .map_err(|e| e.to_string())
}

// ── 远程同步命令 ──

#[tauri::command]
fn refresh_remote(state: tauri::State<AppState>, id: String) -> Result<(), String> {
    hostz::core::hosts_manager::refresh_remote_async(&state.swhdb, &id)
        .map_err(|e| e.to_string())
}

// ── 配置命令 ──

#[tauri::command]
fn config_get(state: tauri::State<AppState>, key: String) -> Result<Option<String>, String> {
    let cfgdb = state.cfgdb.lock().map_err(|e| e.to_string())?;
    cfgdb.get(&key).map_err(|e| e.to_string())
}

#[tauri::command]
fn config_all(state: tauri::State<AppState>) -> Result<String, String> {
    let cfgdb = state.cfgdb.lock().map_err(|e| e.to_string())?;
    let config = cfgdb.load_config().map_err(|e| e.to_string())?;
    serde_json::to_string(&config).map_err(|e| e.to_string())
}

#[tauri::command]
fn config_update(
    state: tauri::State<AppState>,
    app_handle: tauri::AppHandle,
    partial_json: String,
) -> Result<String, String> {
    let partial: serde_json::Value =
        serde_json::from_str(&partial_json).map_err(|e| e.to_string())?;
    let cfgdb = state.cfgdb.lock().map_err(|e| e.to_string())?;
    let (_old, new_config, changed_keys) = cfgdb
        .apply_config_update(&partial)
        .map_err(|e| e.to_string())?;

    handle_config_side_effects(&app_handle, &new_config, &changed_keys);
    serde_json::to_string(&new_config).map_err(|e| e.to_string())
}

fn handle_config_side_effects(
    app_handle: &tauri::AppHandle,
    config: &hostz::models::config::AppConfig,
    changed_keys: &[String],
) {
    let changed_set: std::collections::HashSet<&str> =
        changed_keys.iter().map(|s| s.as_str()).collect();

    if changed_set.contains("theme") {
        let _ = app_handle.emit("config_theme_changed", &config.theme);
    }
    if changed_set.contains("locale") {
        let _ = app_handle.emit("config_locale_changed", &config.locale);
    }
    if changed_set.contains("showTitleOnTray") {
        let _ = app_handle.emit("config_tray_title_changed", &config.show_title_on_tray);
    }
    if changed_set.contains("hideDockIcon") && cfg!(target_os = "macos") {
        if let Some(window) = app_handle.get_webview_window("main") {
            let _ = window.set_visible_on_all_workspaces(!config.hide_dock_icon);
        }
    }
    let _ = app_handle.emit("config_updated", serde_json::json!({
        "changed_keys": changed_keys,
    }));
}

// ── 条目管理命令 ──

#[tauri::command]
fn add_item(state: tauri::State<AppState>, title: String, item_type: String, url: Option<String>, refresh_interval: Option<i64>) -> Result<String, String> {
    let swhdb = state.swhdb.lock().map_err(|e| e.to_string())?;
    let mut list = swhdb.get_list().map_err(|e| e.to_string())?;
    let id = uuid::Uuid::new_v4().to_string();
    let type_ = match item_type.as_str() {
        "remote" => hostz::models::hosts::HostsType::Remote,
        "group" => hostz::models::hosts::HostsType::Group,
        _ => hostz::models::hosts::HostsType::Local,
    };
    let item = hostz::models::hosts::HostsListObject {
        id: id.clone(), title, type_, on: false,
        url, refresh_interval,
        order: list.len() as i32,
        ..Default::default()
    };
    list.push(item);
    swhdb.set_list(&list).map_err(|e| e.to_string())?;
    Ok(id)
}

// ── 回收站命令 ──

#[tauri::command]
fn move_to_trashcan(state: tauri::State<AppState>, id: String) -> Result<(), String> {
    let swhdb = state.swhdb.lock().map_err(|e| e.to_string())?;
    hostz::core::trash::move_to_trashcan(&swhdb, &id).map_err(|e| e.to_string())
}

#[tauri::command]
fn restore_from_trashcan(state: tauri::State<AppState>, id: String) -> Result<(), String> {
    let swhdb = state.swhdb.lock().map_err(|e| e.to_string())?;
    hostz::core::trash::restore_from_trashcan(&swhdb, &id).map_err(|e| e.to_string())
}

#[tauri::command]
fn permanently_delete(state: tauri::State<AppState>, id: String) -> Result<(), String> {
    let swhdb = state.swhdb.lock().map_err(|e| e.to_string())?;
    hostz::core::trash::permanently_delete(&swhdb, &id).map_err(|e| e.to_string())
}

#[tauri::command]
fn clear_trashcan(state: tauri::State<AppState>) -> Result<(), String> {
    let swhdb = state.swhdb.lock().map_err(|e| e.to_string())?;
    hostz::core::trash::clear_trashcan(&swhdb).map_err(|e| e.to_string())
}

#[tauri::command]
fn get_trashcan_list(state: tauri::State<AppState>) -> Result<String, String> {
    let swhdb = state.swhdb.lock().map_err(|e| e.to_string())?;
    let list = swhdb.get_trashcan().map_err(|e| e.to_string())?;
    serde_json::to_string(&list).map_err(|e| e.to_string())
}

// ── 历史记录命令 ──

#[tauri::command]
fn get_history(state: tauri::State<AppState>, limit: Option<i32>) -> Result<String, String> {
    let swhdb = state.swhdb.lock().map_err(|e| e.to_string())?;
    let entries = swhdb.get_history(limit.unwrap_or(50)).map_err(|e| e.to_string())?;
    serde_json::to_string(&entries).map_err(|e| e.to_string())
}

#[tauri::command]
fn clear_history(state: tauri::State<AppState>) -> Result<(), String> {
    let swhdb = state.swhdb.lock().map_err(|e| e.to_string())?;
    // delete all by trimming to 0
    swhdb.trim_history(0).map_err(|e| e.to_string())
}

// ── 导入导出命令 ──

#[tauri::command]
fn export_data(state: tauri::State<AppState>) -> Result<String, String> {
    let swhdb = state.swhdb.lock().map_err(|e| e.to_string())?;
    let json = swhdb.to_json().map_err(|e| e.to_string())?;
    serde_json::to_string_pretty(&json).map_err(|e| e.to_string())
}

#[tauri::command]
fn export_to_file(state: tauri::State<AppState>, path: String) -> Result<(), String> {
    let swhdb = state.swhdb.lock().map_err(|e| e.to_string())?;
    let json = swhdb.to_json().map_err(|e| e.to_string())?;
    let content = serde_json::to_string_pretty(&json).map_err(|e| e.to_string())?;
    std::fs::write(&path, &content).map_err(|e| e.to_string())
}

#[tauri::command]
fn import_data(state: tauri::State<AppState>, json_str: String) -> Result<(), String> {
    let data: serde_json::Value = serde_json::from_str(&json_str).map_err(|e| e.to_string())?;
    let swhdb = state.swhdb.lock().map_err(|e| e.to_string())?;
    swhdb.load_json(&data).map_err(|e| e.to_string())
}

#[tauri::command]
fn read_file(path: String) -> Result<String, String> {
    std::fs::read_to_string(&path).map_err(|e| e.to_string())
}

// ── 快捷键命令 ──

#[tauri::command]
fn get_hotkeys(state: tauri::State<AppState>) -> Result<String, String> {
    let defs = state.hotkey_manager.get_definitions();
    serde_json::to_string(&defs).map_err(|e| e.to_string())
}

// ── 搜索替换命令 ──

#[tauri::command]
fn find_by(state: tauri::State<AppState>, query: String, is_regexp: bool, is_ignore_case: bool) -> Result<String, String> {
    let swhdb = state.swhdb.lock().map_err(|e| e.to_string())?;
    let options = hostz::core::search::FindOptions { is_regexp, is_ignore_case };
    let results = hostz::core::search::find_by(&swhdb, &query, options)
        .map_err(|e| e.to_string())?;
    serde_json::to_string(&results).map_err(|e| e.to_string())
}

#[tauri::command]
fn find_and_replace_all(
    state: tauri::State<AppState>,
    query: String,
    replacement: String,
    is_regexp: bool,
    is_ignore_case: bool,
) -> Result<usize, String> {
    let swhdb = state.swhdb.lock().map_err(|e| e.to_string())?;
    let options = hostz::core::search::FindOptions { is_regexp, is_ignore_case };
    hostz::core::search::find_and_replace_all(&swhdb, &query, &replacement, options)
        .map_err(|e| e.to_string())
}

// ── 入口 ──

fn main() -> Result<()> {
    platform::ensure_data_dirs()?;

    let swhdb = SwhDb::open()?;
    let cfgdb = CfgDb::open()?;

    let swhdb = Arc::new(Mutex::new(swhdb));
    let cfgdb = Arc::new(Mutex::new(cfgdb));

    // 克隆 Arc 用于 cron 线程
    let cron_swhdb = Arc::clone(&swhdb);
    let cron_cfgdb = Arc::clone(&cfgdb);

    let app_state = AppState {
        swhdb,
        cfgdb,
        hotkey_manager: hostz::services::hotkey::HotkeyManager::new(),
    };

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(app_state)
        .setup(move |app| {
            // 关闭窗口 → 隐藏到托盘而非退出
            if let Some(window) = app.get_webview_window("main") {
                let w = window.clone();
                window.on_window_event(move |event| {
                    if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                        api.prevent_close();
                        let _ = w.hide();
                    }
                });
            }

            // 创建系统托盘（根据语言设置显示文本）
            let is_en = {
                let state = app.state::<AppState>();
                let cfgdb = state.cfgdb.lock().unwrap();
                cfgdb.load_config().map(|c| c.locale.as_deref() == Some("en")).unwrap_or(false)
            };
            if let Err(e) = hostz::services::tray::create_tray(app, is_en) {
                eprintln!("托盘创建失败: {}", e);
            }

            // 启动后台调度器（远程刷新）
            let cron_running = Arc::new(AtomicBool::new(true));
            hostz::services::cron::start(
                cron_swhdb,
                cron_cfgdb,
                app.handle().clone(),
                cron_running,
            );

            // 启动定期更新检查
            let update_running = Arc::new(AtomicBool::new(true));
            hostz::services::update::start_periodic_check(
                app.handle().clone(),
                update_running,
            );

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            ping,
            get_version,
            add_item,
            get_basic_data,
            get_hosts_content,
            set_hosts_content,
            get_list,
            set_list,
            toggle_hosts,
            apply_to_system,
            get_system_hosts_content,
            set_system_hosts_content,
            refresh_remote,
            config_get,
            config_all,
            config_update,
            move_to_trashcan,
            restore_from_trashcan,
            permanently_delete,
            clear_trashcan,
            get_trashcan_list,
            get_history,
            clear_history,
            export_data,
            export_to_file,
            import_data,
            read_file,
            get_hotkeys,
            find_by,
            find_and_replace_all,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");

    Ok(())
}

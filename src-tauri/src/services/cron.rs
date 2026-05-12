use std::sync::{Arc, Mutex, atomic::{AtomicBool, Ordering}};
use std::thread;
use std::time::Duration;
use tauri::{AppHandle, Emitter};

use crate::core::hosts_manager;
use crate::db::{cfgdb::CfgDb, swhdb::SwhDb};
use crate::models::hosts::HostsType;

use serde_json;

/// 启动后台调度器
pub fn start(
    swhdb: Arc<Mutex<SwhDb>>,
    cfgdb: Arc<Mutex<CfgDb>>,
    app_handle: AppHandle,
    running: Arc<AtomicBool>,
) {
    thread::spawn(move || {
        thread::sleep(Duration::from_secs(5));

        while running.load(Ordering::Relaxed) {
            let _ = check_refresh(&swhdb, &cfgdb, &app_handle);

            for _ in 0..60 {
                if !running.load(Ordering::Relaxed) {
                    return;
                }
                thread::sleep(Duration::from_secs(1));
            }
        }
    });
}

/// 遍历远程条目，对到达刷新时间的执行拉取，并自动应用到系统 hosts
fn check_refresh(
    swhdb: &Arc<Mutex<SwhDb>>,
    cfgdb: &Arc<Mutex<CfgDb>>,
    app_handle: &AppHandle,
) -> Result<(), String> {
    let ids_to_refresh = {
        let db = swhdb.lock().map_err(|e| e.to_string())?;
        let list = db.get_list().map_err(|e| e.to_string())?;
        let now_ms = chrono::Utc::now().timestamp_millis();
        let mut ids = Vec::new();

        for item in &list {
            if item.type_ != HostsType::Remote { continue; }

            let interval = match item.refresh_interval {
                Some(i) if i > 0 => i,
                _ => continue,
            };

            if item.url.as_ref().map_or(true, |u| !u.starts_with("http")) { continue; }

            let last_refresh = item.last_refresh_ms.unwrap_or(0);
            if (now_ms - last_refresh) / 1000 >= interval {
                ids.push(item.id.clone());
            }
        }
        ids
    };

    if ids_to_refresh.is_empty() {
        let _ = app_handle.emit("cron_tick", serde_json::json!({
            "pending": 0,
            "now_ms": chrono::Utc::now().timestamp_millis(),
        }));
        return Ok(());
    }

    let mut any_refreshed = false;
    for id in &ids_to_refresh {
        match hosts_manager::refresh_remote_async(swhdb, id) {
            Ok(_) => {
                any_refreshed = true;
                let _ = app_handle.emit("hosts_refreshed", id);
            }
            Err(e) => {
                let _ = app_handle.emit("cron_error", serde_json::json!({
                    "id": id,
                    "message": e.to_string(),
                }));
            }
        }
    }

    // 自动应用到系统 hosts
    if any_refreshed {
        let db = swhdb.lock().map_err(|e| e.to_string())?;
        let cfg = cfgdb.lock().map_err(|e| e.to_string())?;
        if let Err(e) = hosts_manager::apply_to_system(&db, &cfg) {
            let _ = app_handle.emit("cron_error", serde_json::json!({
                "id": null,
                "message": format!("应用到系统失败: {}", e),
            }));
        }
        let _ = app_handle.emit("reload_list", ());
    }

    let _ = app_handle.emit("cron_tick", serde_json::json!({
        "pending": ids_to_refresh.len(),
        "refreshed": any_refreshed,
        "now_ms": chrono::Utc::now().timestamp_millis(),
    }));

    Ok(())
}
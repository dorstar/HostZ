use std::sync::{Arc, Mutex, atomic::{AtomicBool, Ordering}};
use std::thread;
use std::time::Duration;
use tauri::{AppHandle, Emitter};

use crate::core::{content_parser, hosts_manager};
use crate::db::{cfgdb::CfgDb, swhdb::SwhDb};
use crate::models::hosts::HostsType;

/// 启动后台调度器
///
/// 每 60 秒循环检查：
/// - check_refresh: 遍历 remote 类型条目，满足条件的拉取 URL 更新内容
/// - check_update:  每小时一次检查应用更新（Phase 7 实现）
pub fn start(
    swhdb: Arc<Mutex<SwhDb>>,
    cfgdb: Arc<Mutex<CfgDb>>,
    app_handle: AppHandle,
    running: Arc<AtomicBool>,
) {
    thread::spawn(move || {
        // 初始延迟 5 秒
        thread::sleep(Duration::from_secs(5));

        while running.load(Ordering::Relaxed) {
            // 检查远程刷新
            if let Err(e) = check_refresh(&swhdb, &cfgdb, &app_handle) {
                let _ = app_handle.emit("cron_error", e);
            }

            // check_update 在 Phase 7 实现

            // 分段 sleep，支持优雅退出
            for _ in 0..60 {
                if !running.load(Ordering::Relaxed) {
                    return;
                }
                thread::sleep(Duration::from_secs(1));
            }
        }
    });
}

/// 遍历所有 remote 类型、已启用、设置了刷新间隔的条目，
/// 对到达刷新时间的条目执行拉取更新
fn check_refresh(
    swhdb: &Arc<Mutex<SwhDb>>,
    cfgdb: &Arc<Mutex<CfgDb>>,
    app_handle: &AppHandle,
) -> Result<(), String> {
    // 收集需要刷新的条目 ID
    let ids_to_refresh = {
        let swhdb = swhdb.lock().map_err(|e| e.to_string())?;
        let list = swhdb.get_list().map_err(|e| e.to_string())?;
        let flat = content_parser::flatten(&list);

        let now_ms = chrono::Utc::now().timestamp_millis();
        let mut ids = Vec::new();

        for item in &flat {
            if item.type_ != HostsType::Remote || !item.on {
                continue;
            }

            let interval = match item.refresh_interval {
                Some(i) if i > 0 => i,
                _ => continue,
            };

            // 跳过非 HTTP URL
            if item.url.as_ref().map_or(true, |u| !u.starts_with("http")) {
                continue;
            }

            let last_refresh = item.last_refresh_ms.unwrap_or(0);
            if (now_ms - last_refresh) / 1000 >= interval {
                ids.push(item.id.clone());
            }
        }

        ids
    };

    if ids_to_refresh.is_empty() {
        return Ok(());
    }

    let mut any_refreshed = false;

    for id in &ids_to_refresh {
        let refresh_result = hosts_manager::refresh_remote_async(swhdb, id);

        match refresh_result {
            Ok(_) => {
                any_refreshed = true;
                let _ = app_handle.emit("hosts_refreshed", id);
            }
            Err(e) => {
                let _ = app_handle.emit("hosts_refresh_failed", serde_json::json!({
                    "id": id,
                    "error": e.to_string(),
                }));
            }
        }
    }

    // 应用变更到系统 hosts
    if any_refreshed {
        let swhdb = swhdb.lock().map_err(|e| e.to_string())?;
        let cfgdb = cfgdb.lock().map_err(|e| e.to_string())?;
        hosts_manager::apply_to_system(&swhdb, &cfgdb)
            .map_err(|e| e.to_string())?;

        let _ = app_handle.emit("reload_list", ());
    }

    Ok(())
}

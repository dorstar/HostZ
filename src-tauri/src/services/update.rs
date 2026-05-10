use std::sync::{Arc, Mutex, atomic::{AtomicBool, Ordering}};
use tauri::{AppHandle, Emitter};

use crate::services::notification;

/// 检查应用更新
///
/// 当前为占位实现，每小时最多调用一次。
/// 后续 Phase 集成 Tauri updater 插件进行完整的更新检查。
///
/// 返回 true 表示有新版本可用。
pub fn check_update(
    _app_handle: &AppHandle,
    last_check: &Arc<Mutex<i64>>,
) -> bool {
    let now = chrono::Utc::now().timestamp();
    let mut last = last_check.lock().unwrap();

    // 每小时最多检查一次
    if now - *last < 3600 {
        return false;
    }
    *last = now;

    // TODO: Phase 7/9 集成 Tauri updater
    // let result = app_handle.updater().check().await;

    false
}

/// 在后台线程中定期检查更新
pub fn start_periodic_check(
    app_handle: AppHandle,
    running: Arc<AtomicBool>,
) {
    let last_check = Arc::new(Mutex::new(0i64));

    std::thread::spawn(move || {
        // 初始延迟 5 秒
        std::thread::sleep(std::time::Duration::from_secs(5));

        while running.load(Ordering::Relaxed) {
            if check_update(&app_handle, &last_check) {
                let _ = app_handle.emit("new_version", ());
                notification::update_available("新版本");
            }

            // 每 60 秒检查一次
            for _ in 0..60 {
                if !running.load(Ordering::Relaxed) {
                    return;
                }
                std::thread::sleep(std::time::Duration::from_secs(1));
            }
        }
    });
}

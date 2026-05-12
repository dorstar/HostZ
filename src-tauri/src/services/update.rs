use std::sync::{Arc, Mutex, atomic::{AtomicBool, Ordering}};
use tauri::{AppHandle, Emitter};

use crate::services::notification;
use crate::db::cfgdb::CfgDb;

/// GitHub 仓库信息
const GITHUB_OWNER: &str = "dorstar";
const GITHUB_REPO: &str = "hostz";

/// 从 GitHub API 解析的更新信息
#[derive(Debug, serde::Serialize)]
pub struct UpdateInfo {
    pub has_update: bool,
    pub current_version: String,
    pub latest_version: String,
    pub download_url: String,
    pub release_notes: Option<String>,
}

/// 比较两个语义化版本号
/// 返回: 0=相等, 正数=latest更新, 负数=current更新
fn compare_versions(latest: &str, current: &str) -> i32 {
    let parse = |v: &str| -> Vec<u32> {
        v.trim_start_matches('v')
            .split('.')
            .filter_map(|s| s.parse().ok())
            .collect()
    };

    let lv = parse(latest);
    let cv = parse(current);

    let max_len = lv.len().max(cv.len());
    for i in 0..max_len {
        let l = lv.get(i).unwrap_or(&0);
        let c = cv.get(i).unwrap_or(&0);
        if l != c {
            return *l as i32 - *c as i32;
        }
    }
    0
}

/// 从 GitHub Releases API 检查更新
pub fn check_update_from_github(current_version: &str) -> Result<UpdateInfo, String> {
    let api_url = format!(
        "https://api.github.com/repos/{}/{}/releases/latest",
        GITHUB_OWNER, GITHUB_REPO
    );

    let resp = ureq::get(&api_url)
        .set("User-Agent", &format!("HostZ/{}", current_version))
        .set("Accept", "application/vnd.github+json")
        .timeout(std::time::Duration::from_secs(10))
        .call()
        .map_err(|e| format!("网络请求失败: {}", e))?;

    let body = resp
        .into_string()
        .map_err(|e| format!("读取响应失败: {}", e))?;

    let json: serde_json::Value = serde_json::from_str(&body)
        .map_err(|e| format!("解析响应失败: {}", e))?;

    let tag_name = json["tag_name"]
        .as_str()
        .unwrap_or(current_version);
    let latest_version = tag_name.trim_start_matches('v');

    let html_url = json["html_url"]
        .as_str()
        .unwrap_or("https://github.com/dorstar/hostz/releases");

    let body = json["body"].as_str().map(|s| s.to_string());

    let has_update = compare_versions(latest_version, current_version) > 0;

    Ok(UpdateInfo {
        has_update,
        current_version: current_version.to_string(),
        latest_version: latest_version.to_string(),
        download_url: html_url.to_string(),
        release_notes: body,
    })
}

/// 检查应用更新
///
/// 根据配置决定是否检查更新，每小时最多调用一次。
///
/// 返回 UpdateInfo 表示检查结果。
pub fn check_update(
    cfgdb: &Arc<Mutex<CfgDb>>,
    last_check: &Arc<Mutex<i64>>,
) -> Result<UpdateInfo, String> {
    let now = chrono::Utc::now().timestamp();

    // 检查是否在冷却时间内
    {
        let last = last_check.lock().map_err(|e| e.to_string())?;
        if now - *last < 3600 {
            // 仍在冷却期内，返回"无更新"避免打扰
            let current = env!("CARGO_PKG_VERSION");
            return Ok(UpdateInfo {
                has_update: false,
                current_version: current.to_string(),
                latest_version: current.to_string(),
                download_url: String::new(),
                release_notes: None,
            });
        }
    }

    // 更新检查时间戳
    {
        let mut last = last_check.lock().map_err(|e| e.to_string())?;
        *last = now;
    }

    // 检查配置是否启用自动更新
    let should_check = {
        if let Ok(cfg) = cfgdb.lock() {
            cfg.load_config()
                .map(|c| c.auto_download_update)
                .unwrap_or(true)
        } else {
            true
        }
    };

    if !should_check {
        let current = env!("CARGO_PKG_VERSION");
        return Ok(UpdateInfo {
            has_update: false,
            current_version: current.to_string(),
            latest_version: current.to_string(),
            download_url: String::new(),
            release_notes: None,
        });
    }

    let current = env!("CARGO_PKG_VERSION");
    check_update_from_github(current)
}

/// 在后台线程中定期检查更新
pub fn start_periodic_check(
    app_handle: AppHandle,
    cfgdb: Arc<Mutex<CfgDb>>,
    running: Arc<AtomicBool>,
) {
    let last_check = Arc::new(Mutex::new(0i64));

    std::thread::spawn(move || {
        // 初始延迟 5 秒
        std::thread::sleep(std::time::Duration::from_secs(5));

        while running.load(Ordering::Relaxed) {
            match check_update(&cfgdb, &last_check) {
                Ok(info) => {
                    if info.has_update {
                        // 发送 new_version 事件到前端，触发弹窗
                        let _ = app_handle.emit("new_version", serde_json::json!({
                            "current": info.current_version,
                            "latest": info.latest_version,
                            "url": info.download_url,
                            "notes": info.release_notes,
                        }));
                        // 同时发送桌面通知
                        notification::update_available(&info.latest_version);
                    }
                }
                Err(e) => {
                    eprintln!("检查更新失败: {}", e);
                }
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

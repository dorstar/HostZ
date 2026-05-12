use std::sync::{Arc, Mutex};

use anyhow::Result;
use crate::core::content_parser;
use crate::core::privilege;
use crate::db::cfgdb::CfgDb;
use crate::db::swhdb::SwhDb;
use crate::models::config::WriteMode;
use crate::models::hosts::{HostsHistoryObject, HostsListObject};

/// 系统 hosts 内容标记（append 模式用）
const HOSTS_CONTENT_MARKER: &str = "# --- SWITCHHOSTS_CONTENT_START ---";

/// 收集所有已启用的 hosts 内容并写入系统 hosts 文件
pub fn apply_to_system(swhdb: &SwhDb, cfgdb: &CfgDb) -> Result<()> {
    let list = swhdb.get_list()?;
    let content = content_parser::get_enabled_content(swhdb, &list)?;

    // 规范化换行符为 LF
    let content = content.replace("\r\n", "\n");

    // 按配置去除重复记录
    let content = if cfgdb.load_config().map(|c| c.remove_duplicate_records).unwrap_or(false) {
        content_parser::remove_duplicate_lines(&content)
    } else {
        content
    };

    // 读取当前系统 hosts 并规范化为 LF
    let system_path = crate::utils::platform::system_hosts_path();
    let current_content = std::fs::read_to_string(&system_path)
        .unwrap_or_default()
        .replace("\r\n", "\n");

    // 根据写入模式处理内容
    let write_mode = get_write_mode(cfgdb);
    let final_content = match write_mode {
        WriteMode::Append => make_append_content(&current_content, &content),
        WriteMode::Overwrite => content,
    };

    // 检查内容是否有变化（简单比较，在恢复平台换行符之前）
    if current_content == final_content {
        return Ok(());
    }

    // 恢复平台换行符
    let final_content = if cfg!(target_os = "windows") {
        final_content.replace('\n', "\r\n")
    } else {
        final_content
    };

    // 写入系统 hosts
    privilege::write_system_hosts(&system_path, &final_content)?;

    // 记录写入历史
    let mode_label = match write_mode {
        WriteMode::Append => "追加模式",
        WriteMode::Overwrite => "覆盖模式",
    };
    let history_entry = HostsHistoryObject {
        id: uuid::Uuid::new_v4().to_string(),
        content: final_content.clone(),
        add_time_ms: chrono::Utc::now().timestamp_millis(),
        label: Some(mode_label.to_string()),
    };
    swhdb.add_history(&history_entry)?;

    // 裁剪过旧的历史记录
    let limit = get_history_limit(cfgdb);
    swhdb.trim_history(limit)?;

    Ok(())
}

/// append 模式：在已有系统 hosts 中查找标记，替换标记之后的内容；
/// 若未找到标记，则追加标记和新内容
fn make_append_content(current: &str, new_content: &str) -> String {
    if let Some(pos) = current.find(HOSTS_CONTENT_MARKER) {
        let before = &current[..pos];
        format!("{}{}\n{}", before, HOSTS_CONTENT_MARKER, new_content)
    } else {
        let sep = if current.is_empty() || current.ends_with('\n') {
            ""
        } else {
            "\n"
        };
        format!("{}{}\n{}\n{}", current, sep, HOSTS_CONTENT_MARKER, new_content)
    }
}

/// 从配置数据库读取写入模式
fn get_write_mode(cfgdb: &CfgDb) -> WriteMode {
    cfgdb.load_config()
        .map(|c| c.write_mode)
        .unwrap_or_default()
}

/// 从配置数据库读取历史记录上限
fn get_history_limit(cfgdb: &CfgDb) -> i32 {
    cfgdb.load_config()
        .map(|c| c.history_limit)
        .unwrap_or(50)
}

/// 拉取远程 hosts 内容
fn fetch_remote_content(url: &str) -> Result<String> {
    let response = ureq::get(url)
        .set("User-Agent", &format!("HostZ/{}", env!("CARGO_PKG_VERSION")))
        .timeout(std::time::Duration::from_secs(30))
        .call()
        .map_err(|e| anyhow::anyhow!("HTTP 请求失败: {}", e))?;
    response.into_string()
        .map_err(|e| anyhow::anyhow!("读取响应失败: {}", e))
}

/// 切换单个条目的启用状态（仅更新数据库，不写系统 hosts）
pub fn toggle_item(swhdb: &SwhDb, cfgdb: &CfgDb, id: &str) -> Result<bool> {
    let list = swhdb.get_list()?;
    let config = cfgdb.load_config()?;
    let choice_mode = config.choice_mode;
    let multi_switch_all = config.multi_chose_folder_switch_all;

    let item = content_parser::find_item_by_id(&list, id)
        .ok_or_else(|| anyhow::anyhow!("条目未找到: {}", id))?;
    let new_on = !item.on;

    let new_list = content_parser::set_on_state_of_item(
        list, id, new_on, choice_mode, multi_switch_all,
    );

    swhdb.set_list(&new_list)?;

    Ok(new_on)
}

/// 测试用：直接 &SwhDb 的同步刷新（单线程安全）
pub fn refresh_remote(swhdb: &SwhDb, id: &str) -> Result<()> {
    let list = swhdb.get_list()?;
    let item = content_parser::find_item_by_id(&list, id)
        .ok_or_else(|| anyhow::anyhow!("条目未找到: {}", id))?;
    let url = item.url.clone()
        .ok_or_else(|| anyhow::anyhow!("非远程条目，无法刷新"))?;
    let content = fetch_remote_content(&url)?;
    swhdb.set_content(id, &content)?;
    let now_ms = chrono::Utc::now().timestamp_millis();
    let updated = HostsListObject { last_refresh_ms: Some(now_ms), ..item.clone() };
    let new_list = content_parser::update_one_item(&list, &updated);
    swhdb.set_list(&new_list)?;
    Ok(())
}

/// 刷新远程 hosts（根据 ID 拉取 URL 内容）
/// 锁内只读 URL→释放锁→网络请求→重新加锁写入，避免慢网络阻塞所有操作
pub fn refresh_remote_async(swhdb: &Arc<Mutex<SwhDb>>, id: &str) -> Result<()> {
    // 第一步：加锁读取 URL，立即释放
    let url = {
        let db = swhdb.lock().map_err(|e| anyhow::anyhow!("锁失败: {}", e))?;
        let list = db.get_list()?;
        let item = content_parser::find_item_by_id(&list, id)
            .ok_or_else(|| anyhow::anyhow!("条目未找到: {}", id))?;
        item.url.clone()
            .ok_or_else(|| anyhow::anyhow!("非远程条目，无法刷新"))?
    };
    // 锁在此处释放

    // 第二步：不加锁请求网络（安全拉取：禁用重定向 + 5MB 上限）
    let content = fetch_remote_content(&url)?;

    // 第三步：重新加锁写入结果
    let db = swhdb.lock().map_err(|e| anyhow::anyhow!("锁失败: {}", e))?;
    let list = db.get_list()?;
    let item = content_parser::find_item_by_id(&list, id)
        .ok_or_else(|| anyhow::anyhow!("条目未找到: {}", id))?;

    db.set_content(id, &content)?;

    let now_ms = chrono::Utc::now().timestamp_millis();
    let updated_item = HostsListObject {
        last_refresh_ms: Some(now_ms),
        ..item.clone()
    };
    let new_list = content_parser::update_one_item(&list, &updated_item);
    db.set_list(&new_list)?;

    Ok(())
}

/// 读取系统 hosts 文件内容
pub fn get_system_hosts_content() -> Result<String> {
    let path = crate::utils::platform::system_hosts_path();
    std::fs::read_to_string(&path)
        .map_err(|e| anyhow::anyhow!("读取系统 hosts 失败: {}", e))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_make_append_content_no_marker() {
        let current = "127.0.0.1 localhost\n";
        let new_content = "10.0.0.1 example.com\n";
        let result = make_append_content(current, new_content);
        assert!(result.contains(HOSTS_CONTENT_MARKER));
        assert!(result.contains("localhost"));
        assert!(result.contains("example.com"));
    }

    #[test]
    fn test_make_append_content_with_marker() {
        let current = "127.0.0.1 localhost\n# --- SWITCHHOSTS_CONTENT_START ---\nold stuff\n";
        let new_content = "10.0.0.1 example.com\n";
        let result = make_append_content(current, new_content);
        assert!(result.contains("localhost"));
        assert!(result.contains("example.com"));
        assert!(!result.contains("old stuff")); // old content replaced
        // marker should appear exactly once
        assert_eq!(result.matches(HOSTS_CONTENT_MARKER).count(), 1);
    }
}

use anyhow::{anyhow, Result};

use crate::db::swhdb::SwhDb;
use crate::models::hosts::{HostsListObject, HostsType};

/// 展开列表为平铺数组（扁平结构，无嵌套）
pub fn flatten(list: &[HostsListObject]) -> Vec<&HostsListObject> {
    list.iter().collect()
}

/// 在列表中按 ID 查找条目（不可变引用）
pub fn find_item_by_id<'a>(list: &'a [HostsListObject], id: &str) -> Option<&'a HostsListObject> {
    list.iter().find(|item| item.id == id)
}

/// 在列表中按 ID 查找条目（可变引用）
pub fn find_item_by_id_mut<'a>(list: &'a mut [HostsListObject], id: &str) -> Option<&'a mut HostsListObject> {
    list.iter_mut().find(|item| item.id == id)
}

/// 递归获取条目的 hosts 内容
///
/// - local/remote: 从 hosts_content 表读取
/// - group: 遍历 include 中的 ID，递归拼接
pub fn get_content_of_hosts(db: &SwhDb, list: &[HostsListObject], id: &str) -> Result<String> {
    let item = find_item_by_id(list, id)
        .ok_or_else(|| anyhow!("条目未找到: {}", id))?;

    match item.type_ {
        HostsType::Local | HostsType::Remote => {
            let content = db.get_content(id)?;
            Ok(content.map(|c| c.content).unwrap_or_default())
        }
        HostsType::Group => {
            let mut content = String::new();
            if let Some(ref include_ids) = item.include {
                for inc_id in include_ids {
                    if let Some(inc_item) = find_item_by_id(list, inc_id) {
                        if !content.is_empty() {
                            content.push_str("\n\n");
                        }
                        content.push_str(&format!("# file: {}\n", inc_item.title));
                        content.push_str(&get_content_of_hosts(db, list, inc_id)?);
                    }
                }
            }
            Ok(content)
        }
    }
}

/// 收集所有 on=true 的条目的合并内容（用于写入系统 hosts）
pub fn get_enabled_content(db: &SwhDb, list: &[HostsListObject]) -> Result<String> {
    let mut content = String::new();

    for item in list {
        if !item.on {
            continue;
        }
        let item_content = get_content_of_hosts(db, list, &item.id)?;
        if !item_content.is_empty() {
            if !content.is_empty() {
                content.push_str("\n\n");
            }
            content.push_str(&item_content);
        }
    }

    Ok(content)
}

/// 不可变更新列表中的单个条目
pub fn update_one_item(list: &[HostsListObject], updated: &HostsListObject) -> Vec<HostsListObject> {
    list.iter().map(|item| {
        if item.id == updated.id { updated.clone() } else { item.clone() }
    }).collect()
}

/// 切换条目的启用/禁用状态，返回修改后的列表
///
/// choice_mode == 1: 顶层单选，关闭其他所有条目
/// choice_mode != 1: 允许多选
pub fn set_on_state_of_item(
    mut list: Vec<HostsListObject>,
    id: &str,
    on: bool,
    choice_mode: i32,
    _multi_chose_folder_switch_all: bool,
) -> Vec<HostsListObject> {
    let item = match find_item_by_id_mut(&mut list, id) {
        Some(item) => item,
        None => return list,
    };

    item.on = on;

    if !on {
        return list;
    }

    // 顶层单选模式 (choice_mode == 1): 关闭其他所有条目
    if choice_mode == 1 {
        for item in list.iter_mut() {
            if item.id != id {
                item.on = false;
            }
        }
    }

    list
}

/// 从列表中删除条目（按 ID），返回修改后的列表
pub fn delete_item_by_id(list: &[HostsListObject], id: &str) -> Vec<HostsListObject> {
    list.iter()
        .filter(|item| item.id != id)
        .cloned()
        .collect()
}

/// 获取当前选中条目之后的下一个可选条目 ID
pub fn get_next_selected_item(list: &[HostsListObject], current_id: &str) -> Option<String> {
    let current_idx = list.iter().position(|item| item.id == current_id)?;
    list.get(current_idx + 1).map(|item| item.id.clone())
}

/// 去除 hosts 内容中的重复记录行
///
/// 以 (IP, hostname) 对为去重键，首次出现保留，后续重复跳过。
/// 注释行和空行不受影响。行内 `#` 后的部分不参与去重键计算。
pub fn remove_duplicate_lines(content: &str) -> String {
    let mut seen = std::collections::HashSet::new();
    let mut result = String::with_capacity(content.len());
    let mut is_first = true;
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            if !is_first {
                result.push('\n');
            }
            result.push_str(line);
            is_first = false;
            continue;
        }
        let no_comment = trimmed.split('#').next().unwrap_or(trimmed);
        let parts: Vec<&str> = no_comment.split_whitespace().collect();
        if parts.len() >= 2 {
            let key = format!("{} {}", parts[0], parts[1]);
            if !seen.insert(key) {
                continue;
            }
        }
        if !is_first {
            result.push('\n');
        }
        result.push_str(line);
        is_first = false;
    }
    if content.ends_with('\n') {
        result.push('\n');
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_item(id: &str, type_: HostsType, on: bool) -> HostsListObject {
        HostsListObject {
            id: id.to_string(),
            title: id.to_string(),
            type_,
            on,
            content: None,
            url: None,
            refresh_interval: None,
            last_refresh_ms: None,
            include: None,
            order: 0,
        }
    }

    #[test]
    fn test_flatten() {
        let list = vec![
            make_item("a", HostsType::Local, true),
            make_item("b", HostsType::Local, false),
        ];
        let flat = flatten(&list);
        assert_eq!(flat.len(), 2);
    }

    #[test]
    fn test_find_item_by_id() {
        let list = vec![
            make_item("a", HostsType::Local, true),
            make_item("b", HostsType::Remote, false),
        ];
        assert!(find_item_by_id(&list, "a").is_some());
        assert!(find_item_by_id(&list, "nonexistent").is_none());
    }

    #[test]
    fn test_delete_item_by_id() {
        let list = vec![
            make_item("a", HostsType::Local, true),
            make_item("b", HostsType::Local, false),
        ];
        let result = delete_item_by_id(&list, "a");
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].id, "b");
    }

    #[test]
    fn test_set_on_state_single_choice() {
        let list = vec![
            make_item("a", HostsType::Local, true),
            make_item("b", HostsType::Local, false),
        ];
        let result = set_on_state_of_item(list, "b", true, 1, false);
        let a = find_item_by_id(&result, "a").unwrap();
        let b = find_item_by_id(&result, "b").unwrap();
        assert!(b.on);
        assert!(!a.on); // a was turned off due to single choice
    }

    #[test]
    fn test_remove_duplicate_lines_basic() {
        let content = "127.0.0.1 localhost\n127.0.0.1 localhost\n10.0.0.1 example.com\n";
        let result = remove_duplicate_lines(content);
        assert_eq!(result, "127.0.0.1 localhost\n10.0.0.1 example.com\n");
    }

    #[test]
    fn test_remove_duplicate_lines_keeps_comments() {
        let content = "# comment line\n127.0.0.1 a.com\n127.0.0.1 a.com\n# another comment\n";
        let result = remove_duplicate_lines(content);
        assert_eq!(result, "# comment line\n127.0.0.1 a.com\n# another comment\n");
    }

    #[test]
    fn test_remove_duplicate_lines_different_ip_same_host() {
        let content = "127.0.0.1 test.com\n10.0.0.1 test.com\n";
        let result = remove_duplicate_lines(content);
        assert_eq!(result, "127.0.0.1 test.com\n10.0.0.1 test.com\n"); // different IP, keep both
    }

    #[test]
    fn test_remove_duplicate_lines_empty() {
        assert_eq!(remove_duplicate_lines(""), "");
    }

    #[test]
    fn test_remove_duplicate_lines_preserves_newlines() {
        let content = "\n127.0.0.1 a.com\n\n127.0.0.1 a.com\n\n";
        let result = remove_duplicate_lines(content);
        assert_eq!(result, "\n127.0.0.1 a.com\n\n\n");
    }
}

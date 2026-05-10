use anyhow::{anyhow, Result};

use crate::core::content_parser;
use crate::db::swhdb::SwhDb;
use crate::models::hosts::{HostsListObject, TrashcanItem};

/// 将条目移入回收站
pub fn move_to_trashcan(swhdb: &SwhDb, id: &str) -> Result<()> {
    let list = swhdb.get_list()?;
    let item = content_parser::find_item_by_id(&list, id)
        .ok_or_else(|| anyhow!("条目未找到: {}", id))?;

    let mut trash_data = item.clone();
    trash_data.on = false;

    let trashcan_item = TrashcanItem {
        data: trash_data,
        parent_id: None,
        add_time_ms: chrono::Utc::now().timestamp_millis(),
    };

    let new_list = content_parser::delete_item_by_id(&list, id);
    swhdb.set_list(&new_list)?;
    swhdb.add_to_trashcan(&trashcan_item)?;

    Ok(())
}

/// 从回收站恢复条目到根层级
pub fn restore_from_trashcan(swhdb: &SwhDb, id: &str) -> Result<()> {
    let trashcan = swhdb.get_trashcan()?;
    let trash_item = trashcan.iter()
        .find(|t| t.data.id == id)
        .ok_or_else(|| anyhow!("回收站中未找到条目: {}", id))?;

    let mut list = swhdb.get_list()?;
    list.push(trash_item.data.clone());

    swhdb.set_list(&list)?;
    swhdb.remove_from_trashcan(id)?;

    Ok(())
}

/// 永久删除回收站中的条目（同时删除关联的 hosts_content）
pub fn permanently_delete(swhdb: &SwhDb, id: &str) -> Result<()> {
    let trashcan = swhdb.get_trashcan()?;
    let trash_item = trashcan.iter()
        .find(|t| t.data.id == id)
        .ok_or_else(|| anyhow!("回收站中未找到条目: {}", id))?;

    let ids = collect_all_ids(&trash_item.data);
    for cid in &ids {
        let _ = swhdb.delete_content(cid);
    }

    swhdb.remove_from_trashcan(id)?;
    Ok(())
}

/// 清空回收站（永久删除所有条目）
pub fn clear_trashcan(swhdb: &SwhDb) -> Result<()> {
    let trashcan = swhdb.get_trashcan()?;
    let mut all_ids = Vec::new();
    for item in &trashcan {
        all_ids.extend(collect_all_ids(&item.data));
    }
    for id in &all_ids {
        let _ = swhdb.delete_content(id);
    }
    swhdb.clear_trashcan()?;
    Ok(())
}

/// 获取回收站列表
pub fn get_trashcan_list(swhdb: &SwhDb) -> Result<Vec<TrashcanItem>> {
    swhdb.get_trashcan()
}

/// 收集条目及其 include 引用的所有 ID
pub fn collect_all_ids(item: &HostsListObject) -> Vec<String> {
    let mut ids = vec![item.id.clone()];
    if let Some(ref include_ids) = item.include {
        ids.extend(include_ids.iter().cloned());
    }
    ids
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::hosts::{HostsListObject, HostsType};

    fn make_item(id: &str, type_: HostsType) -> HostsListObject {
        HostsListObject {
            id: id.to_string(),
            title: id.to_string(),
            type_,
            on: false,
            content: None,
            url: None,
            refresh_interval: None,
            last_refresh_ms: None,
            include: None,
            order: 0,
        }
    }

    #[test]
    fn test_collect_all_ids_group() {
        let group = HostsListObject {
            include: Some(vec!["inc1".to_string(), "inc2".to_string()]),
            ..make_item("group1", HostsType::Group)
        };
        let ids = collect_all_ids(&group);
        assert_eq!(ids.len(), 3);
        assert!(ids.contains(&"inc1".to_string()));
    }
}

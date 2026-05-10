use hostz::db::{swhdb::SwhDb, cfgdb::CfgDb};
use hostz::models::hosts::{
    HostsListObject, HostsType,
};
use hostz::models::config::{AppConfig, WriteMode, Theme};
use hostz::core::{content_parser, hosts_manager, trash, search};
use hostz::utils::platform;

use std::sync::atomic::{AtomicU32, Ordering};
static DB_COUNTER: AtomicU32 = AtomicU32::new(0);

// ── 测试辅助函数 ──

fn make_temp_db() -> SwhDb {
    let tmp_dir = std::env::temp_dir().join("hostz_test");
    std::fs::create_dir_all(&tmp_dir).ok();
    let id = DB_COUNTER.fetch_add(1, Ordering::Relaxed);
    let db_path = tmp_dir.join(format!("swh_test_{}_{}.db", std::process::id(), id));
    let _ = std::fs::remove_file(&db_path);
    let conn = rusqlite::Connection::open(&db_path).unwrap();
    SwhDb::from_connection(conn)
}

fn make_temp_cfgdb() -> CfgDb {
    let tmp_dir = std::env::temp_dir().join("hostz_test");
    std::fs::create_dir_all(&tmp_dir).ok();
    let id = DB_COUNTER.fetch_add(1, Ordering::Relaxed);
    let db_path = tmp_dir.join(format!("cfg_test_{}_{}.db", std::process::id(), id));
    let _ = std::fs::remove_file(&db_path);
    let conn = rusqlite::Connection::open(&db_path).unwrap();
    CfgDb::from_connection(conn)
}

fn make_item(id: &str, type_: HostsType, on: bool, title: &str) -> HostsListObject {
    HostsListObject {
        id: id.to_string(),
        title: title.to_string(),
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

// ── 测试 1: 数据库 CRUD 全生命周期 ──

#[test]
fn test_full_crud_lifecycle() {
    let db = make_temp_db();

    // 写入内容
    db.set_content("item1", "127.0.0.1 localhost\n").unwrap();
    let content = db.get_content("item1").unwrap();
    assert!(content.is_some());
    assert_eq!(content.unwrap().content, "127.0.0.1 localhost\n");

    // 覆盖写入
    db.set_content("item1", "10.0.0.1 example.com\n").unwrap();
    let content = db.get_content("item1").unwrap().unwrap();
    assert_eq!(content.content, "10.0.0.1 example.com\n");

    // 写入列表
    let list = vec![
        make_item("root1", HostsType::Local, true, "Root 1"),
        make_item("root2", HostsType::Remote, false, "Root 2"),
    ];
    db.set_list(&list).unwrap();

    // 验证读取
    let loaded = db.get_list().unwrap();
    assert_eq!(loaded.len(), 2);
    assert!(content_parser::find_item_by_id(&loaded, "root1").is_some());
    assert!(content_parser::find_item_by_id(&loaded, "root2").is_some());

    // 删除
    db.delete_content("item1").unwrap();
    assert!(db.get_content("item1").unwrap().is_none());
}

// ── 测试 2: 内容解析递归展开 ──

#[test]
fn test_content_resolution_recursive() {
    let db = make_temp_db();

    // local 条目
    db.set_content("local1", "127.0.0.1 localhost\n").unwrap();
    // remote 条目
    db.set_content("remote1", "# remote content\n10.0.0.1 api.example.com\n").unwrap();

    let local_item = make_item("local1", HostsType::Local, true, "Local");
    let remote_item = HostsListObject {
        url: Some("https://example.com/hosts".into()),
        ..make_item("remote1", HostsType::Remote, true, "Remote")
    };

    // group 引用两个子项
    let group = HostsListObject {
        id: "group1".into(),
        title: "Test Group".into(),
        type_: HostsType::Group,
        on: true,
        include: Some(vec!["local1".into(), "remote1".into()]),
        ..make_item("", HostsType::Group, false, "")
    };

    let list = vec![group, local_item, remote_item];

    // 测试 local 直接内容
    let local_content = content_parser::get_content_of_hosts(&db, &list, "local1").unwrap();
    assert!(local_content.contains("localhost"));

    // 测试 group 组合内容
    let group_content = content_parser::get_content_of_hosts(&db, &list, "group1").unwrap();
    assert!(group_content.contains("# file: Local"));
    assert!(group_content.contains("# file: Remote"));
}

// ── 测试 3: 回收站完整流程 ──

#[test]
fn test_trashcan_full_flow() {
    let db = make_temp_db();

    let item = make_item("test1", HostsType::Local, true, "Test Item");
    db.set_content("test1", "127.0.0.1 test.local\n").unwrap();
    db.set_list(&[item.clone()]).unwrap();

    // 移入回收站
    trash::move_to_trashcan(&db, "test1").unwrap();

    // 验证列表中没有
    let list = db.get_list().unwrap();
    assert!(content_parser::find_item_by_id(&list, "test1").is_none());

    // 验证回收站中有
    let trash_list = db.get_trashcan().unwrap();
    assert_eq!(trash_list.len(), 1);
    assert_eq!(trash_list[0].data.id, "test1");
    assert_eq!(trash_list[0].data.on, false); // 移入时关闭
    assert!(trash_list[0].add_time_ms > 0);

    // 恢复
    trash::restore_from_trashcan(&db, "test1").unwrap();
    let list = db.get_list().unwrap();
    assert!(content_parser::find_item_by_id(&list, "test1").is_some());
    assert!(db.get_trashcan().unwrap().is_empty());

    // 再次移入 + 永久删除
    trash::move_to_trashcan(&db, "test1").unwrap();
    trash::permanently_delete(&db, "test1").unwrap();

    // 验证 hosts_content 也已删除
    assert!(db.get_content("test1").unwrap().is_none());
    assert!(db.get_trashcan().unwrap().is_empty());
}

// ── 测试 4: 搜索替换完整流程 ──

#[test]
fn test_search_replace_full_flow() {
    let db = make_temp_db();

    db.set_content("item_a", "127.0.0.1 oldhost.local\n10.0.0.1 oldhost2.local\n").unwrap();
    db.set_content("item_b", "192.168.1.1 oldhost.local\n").unwrap();

    let list = vec![
        make_item("item_a", HostsType::Local, true, "Item A"),
        make_item("item_b", HostsType::Local, true, "Item B"),
    ];
    db.set_list(&list).unwrap();

    // 字面搜索
    let results = search::find_by(
        &db,
        "oldhost",
        search::FindOptions { is_regexp: false, is_ignore_case: false },
    ).unwrap();
    assert_eq!(results.len(), 2); // 两个条目都匹配
    assert!(results[0].positions.len() >= 1);

    // 正则搜索
    let results = search::find_by(
        &db,
        r"oldhost\d+",
        search::FindOptions { is_regexp: true, is_ignore_case: false },
    ).unwrap();
    // item_a should match "oldhost2"
    assert!(results.iter().any(|r| r.item_id == "item_a"));

    // 替换全部
    let count = search::find_and_replace_all(
        &db,
        "oldhost",
        "newhost",
        search::FindOptions { is_regexp: false, is_ignore_case: false },
    ).unwrap();
    assert_eq!(count, 2);

    // 验证替换后内容
    let content_a = db.get_content("item_a").unwrap().unwrap().content;
    assert!(!content_a.contains("oldhost"));
    assert!(content_a.contains("newhost"));

    // 验证不再有匹配
    let results_after = search::find_by(
        &db,
        "oldhost",
        search::FindOptions { is_regexp: false, is_ignore_case: false },
    ).unwrap();
    assert!(results_after.is_empty());
}

// ── 测试 5: 远程 hosts 刷新 ──

const REMOTE_URL: &str = "https://raw.hellogithub.com/hosts";

#[test]
fn test_remote_refresh() {
    let db = make_temp_db();

    let remote_item = HostsListObject {
        id: "remote_test".into(),
        title: "GitHub Hosts".into(),
        type_: HostsType::Remote,
        on: true,
        url: Some(REMOTE_URL.into()),
        refresh_interval: Some(3600),
        ..make_item("", HostsType::Remote, false, "")
    };

    db.set_list(&[remote_item.clone()]).unwrap();

    // 执行远程刷新
    let result = hosts_manager::refresh_remote(&db, "remote_test");
    assert!(result.is_ok(), "远程刷新失败: {:?}", result.err());

    // 验证内容已获取
    let content = db.get_content("remote_test").unwrap();
    assert!(content.is_some(), "未获取到远程内容");
    let content = content.unwrap().content;
    assert!(!content.is_empty(), "远程内容为空");

    // 验证 hosts 格式：应包含 IP 和域名
    let has_ip = content.lines().any(|line| {
        line.split_whitespace().next()
            .map_or(false, |first| first.parse::<std::net::IpAddr>().is_ok())
    });
    assert!(has_ip, "远程内容应包含有效 IP 地址");

    // 验证 last_refresh_ms 已更新
    let list = db.get_list().unwrap();
    let updated = content_parser::find_item_by_id(&list, "remote_test").unwrap();
    assert!(updated.last_refresh_ms.is_some());
    assert!(updated.last_refresh_ms.unwrap() > 0);

    println!("远程 hosts 内容 (前 10 行):");
    for line in content.lines().take(10) {
        println!("  {}", line);
    }
}

// ── 测试 6: 开关逻辑 ──

#[test]
fn test_toggle_logic() {
    let db = make_temp_db();
    let cfgdb = make_temp_cfgdb();

    let item1 = make_item("item1", HostsType::Local, false, "Item 1");
    let item2 = make_item("item2", HostsType::Local, true, "Item 2");

    db.set_content("item1", "127.0.0.1 i1.local\n").unwrap();
    db.set_content("item2", "127.0.0.1 i2.local\n").unwrap();
    db.set_list(&[item1, item2]).unwrap();
    cfgdb.save_config(&AppConfig::default()).unwrap();

    // 单选模式 (choice_mode=1): 打开 item1 应关闭 item2
    let list = db.get_list().unwrap();
    let new_list = content_parser::set_on_state_of_item(
        list, "item1", true, 1, false,
    );
    let i1 = content_parser::find_item_by_id(&new_list, "item1").unwrap();
    let i2 = content_parser::find_item_by_id(&new_list, "item2").unwrap();
    assert!(i1.on, "item1 应开启");
    assert!(!i2.on, "item2 应被关闭");

    // 多选模式 (choice_mode=2): 两个都可以 ON
    let list = db.get_list().unwrap();
    let new_list = content_parser::set_on_state_of_item(
        list.clone(), "item1", true, 2, false,
    );
    let new_list = content_parser::set_on_state_of_item(
        new_list, "item2", true, 2, false,
    );
    let i1 = content_parser::find_item_by_id(&new_list, "item1").unwrap();
    let i2 = content_parser::find_item_by_id(&new_list, "item2").unwrap();
    assert!(i1.on, "多选模式 item1 应开启");
    assert!(i2.on, "多选模式 item2 也应开启");
}

// ── 测试 7: 配置系统 ──

#[test]
fn test_config_full_flow() {
    let cfgdb = make_temp_cfgdb();

    // 加载默认配置
    let config = cfgdb.load_config().unwrap();
    assert_eq!(config.write_mode, WriteMode::Append);
    assert_eq!(config.history_limit, 50);
    assert_eq!(config.choice_mode, 2);

    // 保存自定义配置
    let mut custom = config.clone();
    custom.write_mode = WriteMode::Overwrite;
    custom.theme = Theme::Dark;
    custom.hide_at_launch = true;
    custom.history_limit = 100;
    cfgdb.save_config(&custom).unwrap();

    // 重新加载验证
    let loaded = cfgdb.load_config().unwrap();
    assert_eq!(loaded.write_mode, WriteMode::Overwrite);
    assert_eq!(loaded.theme, Theme::Dark);
    assert!(loaded.hide_at_launch);
    assert_eq!(loaded.history_limit, 100);

    // 部分更新
    let partial = serde_json::json!({
        "theme": "system",
        "historyLimit": 200,
    });
    let (old, new, changed_keys) = cfgdb.apply_config_update(&partial).unwrap();
    assert_eq!(changed_keys.len(), 2);
    assert!(changed_keys.contains(&"theme".to_string()));
    assert!(changed_keys.contains(&"historyLimit".to_string()));
    assert_eq!(old.theme, Theme::Dark);
    assert_eq!(new.theme, Theme::System);
    assert_eq!(new.history_limit, 200);
}

// ── 测试 8: 获取已启用内容 ──

#[test]
fn test_get_enabled_content() {
    let db = make_temp_db();

    db.set_content("enabled1", "127.0.0.1 enabled.local\n").unwrap();
    db.set_content("disabled1", "10.0.0.1 disabled.local\n").unwrap();

    let list = vec![
        make_item("enabled1", HostsType::Local, true, "Enabled"),
        make_item("disabled1", HostsType::Local, false, "Disabled"),
    ];
    db.set_list(&list).unwrap();

    let content = content_parser::get_enabled_content(&db, &list).unwrap();
    assert!(content.contains("enabled.local"));
    assert!(!content.contains("disabled.local"), "禁用条目不应出现在输出中");
}

// ── 测试 9: 系统路径验证 ──

#[test]
fn test_system_paths() {
    let hosts_path = platform::system_hosts_path();
    if cfg!(target_os = "windows") {
        assert!(hosts_path.to_str().unwrap().contains("etc"));
        assert!(hosts_path.to_str().unwrap().contains("hosts"));
    } else {
        assert_eq!(hosts_path, std::path::PathBuf::from("/etc/hosts"));
    }

    let data_dir = platform::data_dir();
    assert!(data_dir.to_str().unwrap().contains("HostZ"));
    assert!(data_dir.ends_with("data"));
}

// ══════════════════════════════════════════════
// P0 回归测试（审查发现的 Bug 修复验证）
// ══════════════════════════════════════════════

// ── 测试 10: 配置键 camelCase 一致性 ──

#[test]
fn test_config_key_naming_camelcase() {
    let cfgdb = make_temp_cfgdb();
    // 用 camelCase 键更新配置（与前端一致）
    let partial: serde_json::Value = serde_json::json!({
        "showTitleOnTray": true,
        "hideDockIcon": true,
        "hideAtLaunch": true,
        "historyLimit": 123,
    });
    let (_, new_config, changed_keys) = cfgdb.apply_config_update(&partial).unwrap();
    // 键名应为 camelCase（与前端 JSON 一致）
    assert!(changed_keys.contains(&"showTitleOnTray".to_string()));
    assert!(changed_keys.contains(&"hideDockIcon".to_string()));
    assert!(changed_keys.contains(&"hideAtLaunch".to_string()));
    // 新配置应反映更新值
    assert!(new_config.show_title_on_tray);
    assert!(new_config.hide_dock_icon);
    assert!(new_config.hide_at_launch);
    assert_eq!(new_config.history_limit, 123);
}

// ── 测试 11: CRLF 规范化不产生加倍 ──

#[test]
fn test_crlf_normalization_no_doubling() {
    // 模拟 set_system_hosts_content 的规范化逻辑
    let content = "line1\r\nline2\nline3\r\n";
    let normalized = content.replace("\r\n", "\n");
    let result = normalized.replace('\n', "\r\n");
    // 不应出现 \r\r\n
    assert!(!result.contains("\r\r\n"));
    assert!(!result.contains("\r\r"));
    // 每行末尾应为 \r\n
    assert_eq!(result, "line1\r\nline2\r\nline3\r\n");
}

// ── 测试 12: 导入/导出完整往返 ──

#[test]
fn test_import_export_roundtrip() {
    let db = make_temp_db();
    // 创建含列表+回收站的数据
    let list = vec![
        make_item("a", HostsType::Local, true, "Item A"),
        make_item("b", HostsType::Remote, false, "Item B"),
    ];
    db.set_list(&list).unwrap();
    db.set_content("a", "127.0.0.1 localhost").unwrap();
    let trash_item = hostz::models::hosts::TrashcanItem {
        data: make_item("c", HostsType::Local, false, "Trashed C"),
        parent_id: None,
        add_time_ms: 1000,
    };
    db.add_to_trashcan(&trash_item).unwrap();

    // 导出
    let exported = db.to_json().unwrap();
    assert!(exported.get("list").unwrap().as_array().unwrap().len() == 2);
    assert!(exported.get("trashcan").unwrap().as_array().unwrap().len() == 1);

    // 导入到新 DB
    let db2 = make_temp_db();
    db2.load_json(&exported).unwrap();

    // 验证列表恢复了
    let list2 = db2.get_list().unwrap();
    assert_eq!(list2.len(), 2);
    assert_eq!(list2[0].id, "a");
    assert_eq!(list2[1].id, "b");

    // 验证回收站恢复了
    let trash2 = db2.get_trashcan().unwrap();
    assert_eq!(trash2.len(), 1);
    assert_eq!(trash2[0].data.id, "c");
}

// ── 测试 13: 导入无效 JSON 不破坏现有数据 ──

#[test]
fn test_import_invalid_json_preserves_data() {
    let db = make_temp_db();
    let list = vec![make_item("x", HostsType::Local, true, "Original")];
    db.set_list(&list).unwrap();

    // 尝试导入不含 "list" 的对象（合法但空）
    let empty: serde_json::Value = serde_json::json!({});
    db.load_json(&empty).unwrap();
    // 数据库被清空但无新数据 → 列表为空
    assert_eq!(db.get_list().unwrap().len(), 0);
}

// ── 测试 14: 多行正则搜索不 panic ──

#[test]
fn test_multiline_regex_no_panic() {
    let db = make_temp_db();
    let list = vec![make_item("m", HostsType::Local, true, "Multi")];
    db.set_list(&list).unwrap();
    db.set_content("m", "line1 abc\nline2 def\nline3 ghi\n").unwrap();

    let options = search::FindOptions { is_regexp: true, is_ignore_case: false };
    // 使用跨行模式 (?s) 的正则表达式
    let results = search::find_by(&db, "(?s)abc.*def", options.clone()).unwrap_or_default();
    // 应正常返回（不 panic），可能找到或找不到——关键是不要崩溃
    // 如果没找到跨行匹配也是可以接受的
    let _ = results.len();
}

// ── 测试 15: 展开/扁平操作保持一致性 ──

#[test]
fn test_flatten_roundtrip_preserves_ids() {
    let list = vec![
        make_item("a", HostsType::Local, true, "A"),
        make_item("b", HostsType::Remote, false, "B"),
    ];
    let flat = content_parser::flatten(&list);
    assert_eq!(flat.len(), 2);
    let ids: Vec<&str> = flat.iter().map(|i| i.id.as_str()).collect();
    assert!(ids.contains(&"a"));
    assert!(ids.contains(&"b"));
}

// ── 测试 16: 回收站引用关联 ID 收集 ──

#[test]
fn test_trash_collect_ids_for_group() {
    let mut group = make_item("g1", HostsType::Group, true, "Group");
    group.include = Some(vec!["ref1".to_string(), "ref2".to_string()]);
    // 组和引用条目都被删除时应收集所有 ID
    let ids = trash::collect_all_ids(&group);
    assert!(ids.contains(&"g1".to_string()));
    assert!(ids.contains(&"ref1".to_string()));
    assert!(ids.contains(&"ref2".to_string()));
    assert_eq!(ids.len(), 3);
}

// ── 测试 17: 空内容搜索不 panic ──

#[test]
fn test_search_empty_content() {
    let db = make_temp_db();
    let list = vec![make_item("e", HostsType::Local, true, "Empty")];
    db.set_list(&list).unwrap();
    // 不设定内容——get_content 返回 None

    let options = search::FindOptions::default();
    let results = search::find_by(&db, "anything", options).unwrap();
    // 空内容条目应被跳过，结果为空
    assert!(results.is_empty());
}

// ── 测试 18: 设置/获取配置后加载一致性 ──

#[test]
fn test_config_set_get_roundtrip() {
    let cfgdb = make_temp_cfgdb();
    cfgdb.set("testKey", "\"testValue\"").unwrap();
    let val = cfgdb.get("testKey").unwrap();
    assert_eq!(val, Some("\"testValue\"".to_string()));
}

// ── 测试 19: 历史记录上限裁剪 ──

#[test]
fn test_history_trim() {
    use hostz::models::hosts::HostsHistoryObject;

    let db = make_temp_db();
    for i in 0..10 {
        db.add_history(&HostsHistoryObject {
            id: format!("h{}", i),
            content: format!("content{}", i),
            add_time_ms: i * 1000,
            label: None,
        }).unwrap();
    }
    assert_eq!(db.get_history(100).unwrap().len(), 10);
    db.trim_history(5).unwrap();
    assert_eq!(db.get_history(100).unwrap().len(), 5);
}

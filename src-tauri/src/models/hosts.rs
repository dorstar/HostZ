use serde::{Deserialize, Serialize};

/// hosts 条目类型，对应原版 HostsType
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "lowercase")]
pub enum HostsType {
    #[default]
    Local,
    Remote,
    Group,
}

/// hosts 列表条目（树节点），对应原版 IHostsListObject
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct HostsListObject {
    pub id: String,
    pub title: String,
    #[serde(rename = "type")]
    pub type_: HostsType,
    #[serde(default)]
    pub on: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub refresh_interval: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_refresh_ms: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub include: Option<Vec<String>>,
    pub order: i32,
}

/// 回收站条目，对应原版 ITrashcanItem / ITrashcanObject
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TrashcanItem {
    pub data: HostsListObject,
    pub parent_id: Option<String>,
    pub add_time_ms: i64,
}

/// hosts 内容存储，对应原版 IHostsContentObject
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HostsContentObject {
    pub id: String,
    pub content: String,
}

/// 系统 hosts 写入历史，对应原版 IHostsHistoryObject
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HostsHistoryObject {
    pub id: String,
    pub content: String,
    pub add_time_ms: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
}

/// 应用启动时加载的完整基础数据
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HostsBasicData {
    pub list: Vec<HostsListObject>,
    pub trashcan: Vec<TrashcanItem>,
    pub version: [u16; 4],
}

/// 查找结果中的单个匹配位置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FindPosition {
    pub start: usize,
    pub end: usize,
    pub line: usize,
    pub line_pos: usize,
    pub end_line: usize,
    pub end_line_pos: usize,
    pub before: String,
    pub r#match: String,
    pub after: String,
}

/// 查找结果中的一个条目
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FindItem {
    pub item_id: String,
    pub item_title: String,
    pub item_type: HostsType,
    pub positions: Vec<FindPosition>,
}

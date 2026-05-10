use serde::{Deserialize, Serialize};

/// hosts 写入模式
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum WriteMode {
    Append,
    Overwrite,
}

impl Default for WriteMode {
    fn default() -> Self {
        WriteMode::Append
    }
}

/// UI 主题
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Theme {
    Light,
    Dark,
    System,
}

impl Default for Theme {
    fn default() -> Self {
        Theme::Light
    }
}

/// 应用配置，对应原版 ConfigsType
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppConfig {
    // ---- UI ----
    #[serde(default = "default_true")]
    pub left_panel_show: bool,
    #[serde(default = "default_left_panel_width")]
    pub left_panel_width: i32,
    #[serde(default)]
    pub use_system_window_frame: bool,

    // ---- 偏好设置 ----
    #[serde(default)]
    pub write_mode: WriteMode,
    #[serde(default = "default_history_limit")]
    pub history_limit: i32,
    #[serde(default)]
    pub locale: Option<String>,
    #[serde(default)]
    pub theme: Theme,
    #[serde(default = "default_choice_mode")]
    pub choice_mode: i32,
    #[serde(default)]
    pub show_title_on_tray: bool,
    #[serde(default)]
    pub hide_at_launch: bool,
    #[serde(default)]
    pub send_usage_data: bool,
    #[serde(default)]
    pub cmd_after_hosts_apply: Option<String>,
    #[serde(default)]
    pub remove_duplicate_records: bool,
    #[serde(default)]
    pub hide_dock_icon: bool,

    // ---- 托盘 ----
    #[serde(default = "default_true")]
    pub tray_mini_window: bool,
    #[serde(default)]
    pub multi_chose_folder_switch_all: bool,

    // ---- 更新 ----
    #[serde(default = "default_true")]
    pub auto_download_update: bool,
}

fn default_true() -> bool { true }
fn default_left_panel_width() -> i32 { 270 }
fn default_history_limit() -> i32 { 50 }
fn default_choice_mode() -> i32 { 2 }

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            left_panel_show: true,
            left_panel_width: 270,
            use_system_window_frame: false,
            write_mode: WriteMode::Append,
            history_limit: 50,
            locale: None,
            theme: Theme::Light,
            choice_mode: 2,
            show_title_on_tray: false,
            hide_at_launch: false,
            send_usage_data: false,
            cmd_after_hosts_apply: None,
            remove_duplicate_records: false,
            hide_dock_icon: false,
            tray_mini_window: true,
            multi_chose_folder_switch_all: false,
            auto_download_update: true,
        }
    }
}

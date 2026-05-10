use serde::Serialize;
use std::collections::HashMap;
use std::sync::Mutex;

/// 快捷键定义
#[derive(Debug, Clone, Serialize)]
pub struct HotkeyDef {
    pub id: String,
    pub modifiers: Modifiers,
    pub key: String,
}

/// 修饰键
#[derive(Debug, Clone, Default, Serialize)]
pub struct Modifiers {
    pub ctrl: bool,
    pub alt: bool,
    pub shift: bool,
    pub meta: bool,
}

/// 快捷键管理器（占位实现）
///
/// Tauri v2 将全局快捷键移至插件 `tauri-plugin-global-shortcut`。
/// 当前版本提供占位结构，后续可通过添加插件实现完整的全局快捷键功能。
///
/// 预定义的快捷键：
/// - Ctrl+Shift+H → 切换当前选中条目的启用状态
/// - Ctrl+Shift+S → 显示/隐藏主窗口
pub struct HotkeyManager {
    #[allow(dead_code)]
    shortcuts: Mutex<HashMap<String, HotkeyDef>>,
}

impl HotkeyManager {
    pub fn new() -> Self {
        let mut shortcuts = HashMap::new();

        shortcuts.insert(
            "toggle_hosts".into(),
            HotkeyDef {
                id: "toggle_hosts".into(),
                modifiers: Modifiers {
                    ctrl: true,
                    shift: true,
                    alt: false,
                    meta: false,
                },
                key: "H".into(),
            },
        );

        shortcuts.insert(
            "toggle_window".into(),
            HotkeyDef {
                id: "toggle_window".into(),
                modifiers: Modifiers {
                    ctrl: true,
                    shift: true,
                    alt: false,
                    meta: false,
                },
                key: "S".into(),
            },
        );

        Self {
            shortcuts: Mutex::new(shortcuts),
        }
    }

    /// 注册所有快捷键（TODO: 集成 tauri-plugin-global-shortcut 后实现）
    #[allow(dead_code)]
    pub fn register_all(&self) {
        // 后续 Phase: 使用 tauri-plugin-global-shortcut 注册
        // app_handle.plugin(
        //     tauri_plugin_global_shortcut::Builder::new()
        //         .with_shortcut("Ctrl+Shift+H", |app, _shortcut, event| {
        //             ...
        //         })
        //         .build()
        // )?;
    }

    /// 注销所有快捷键
    #[allow(dead_code)]
    pub fn unregister_all(&self) {
        self.shortcuts.lock().unwrap().clear();
    }

    /// 获取已定义的快捷键列表（供前端展示）
    pub fn get_definitions(&self) -> Vec<HotkeyDef> {
        self.shortcuts.lock().unwrap().values().cloned().collect()
    }
}

impl Default for HotkeyManager {
    fn default() -> Self {
        Self::new()
    }
}

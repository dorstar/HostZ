use tauri::{
    Emitter, Manager,
    menu::{MenuBuilder, MenuItemBuilder},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
};

/// 创建系统托盘（根据语言设置显示文本）
pub fn create_tray(app: &tauri::App, is_en: bool) -> anyhow::Result<()> {
    let (show_label, prefs_label, quit_label) = if is_en {
        ("Show Window", "Preferences", "Quit")
    } else {
        ("显示主窗口", "偏好设置", "退出")
    };
    let show = MenuItemBuilder::with_id("show", show_label).build(app)?;
    let prefs = MenuItemBuilder::with_id("prefs", prefs_label).build(app)?;
    let quit = MenuItemBuilder::with_id("quit", quit_label).build(app)?;

    let menu = MenuBuilder::new(app)
        .item(&show)
        .item(&prefs)
        .separator()
        .item(&quit)
        .build()?;

    let icon = app.default_window_icon().cloned().unwrap();

    let _tray = TrayIconBuilder::new()
        .icon(icon)
        .menu(&menu)
        .tooltip("HostZ")
        .on_menu_event(|app, event| {
            match event.id().0.as_str() {
                "show" => {
                    if let Some(window) = app.get_webview_window("main") {
                        let _ = window.unminimize();
                        let _ = window.show();
                        let _ = window.set_focus();
                    }
                }
                "prefs" => {
                    if let Some(window) = app.get_webview_window("main") {
                        let _ = window.unminimize();
                        let _ = window.show();
                        let _ = window.set_focus();
                    }
                    let _ = app.emit("show_preferences", ());
                }
                "quit" => {
                    app.exit(0);
                }
                _ => {}
            }
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                if let Some(window) = tray.app_handle().get_webview_window("main") {
                    if window.is_visible().unwrap_or(false) {
                        let _ = window.hide();
                    } else {
                        let _ = window.show();
                        let _ = window.set_focus();
                    }
                }
            }
        })
        .build(app)?;

    Ok(())
}

use notify_rust::Notification;

/// 发送桌面通知
pub fn send(title: &str, body: &str) {
    let result = Notification::new()
        .summary(title)
        .body(body)
        .appname("HostZ")
        .show();

    if let Err(e) = &result {
        eprintln!("通知发送失败: {}", e);
    }
}

/// 远程刷新成功通知
pub fn remote_refreshed(title: &str) {
    send("远程刷新成功", &format!("「{}」已更新", title));
}

/// 远程刷新失败通知
pub fn remote_refresh_failed(title: &str, error: &str) {
    send(
        "远程刷新失败",
        &format!("「{}」: {}", title, error),
    );
}

/// 更新可用通知
pub fn update_available(version: &str) {
    send("发现新版本", &format!("HostZ {} 可供下载", version));
}

use std::path::PathBuf;

/// 返回系统 hosts 文件路径
pub fn system_hosts_path() -> PathBuf {
    if cfg!(target_os = "windows") {
        let windir = std::env::var("windir").unwrap_or_else(|_| "C:\\Windows".to_string());
        PathBuf::from(windir).join("System32").join("drivers").join("etc").join("hosts")
    } else {
        PathBuf::from("/etc/hosts")
    }
}

/// 返回应用数据根目录
/// Windows: %APPDATA%/SwitchHosts
/// macOS:   ~/Library/Application Support/SwitchHosts
/// Linux:   ~/.config/SwitchHosts
pub fn app_data_dir() -> PathBuf {
    if cfg!(target_os = "windows") {
        let appdata = std::env::var("APPDATA").unwrap_or_else(|_| ".".to_string());
        PathBuf::from(appdata).join("HostZ")
    } else if cfg!(target_os = "macos") {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        PathBuf::from(home).join("Library").join("Application Support").join("HostZ")
    } else {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        PathBuf::from(home).join(".config").join("HostZ")
    }
}

/// 返回 hosts 数据目录 (data/)
pub fn data_dir() -> PathBuf {
    app_data_dir().join("data")
}

/// 返回配置目录 (config/)
pub fn config_dir() -> PathBuf {
    app_data_dir().join("config")
}

/// 确保数据目录和配置目录存在
pub fn ensure_data_dirs() -> std::io::Result<()> {
    std::fs::create_dir_all(data_dir())?;
    std::fs::create_dir_all(config_dir())?;
    Ok(())
}

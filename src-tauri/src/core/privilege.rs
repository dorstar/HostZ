use anyhow::Result;
use std::path::Path;
use std::process::Command;

/// 将内容写入系统 hosts 文件，如无权限则尝试提权
pub fn write_system_hosts(system_path: &Path, content: &str) -> Result<()> {
    match std::fs::write(system_path, content) {
        Ok(_) => return Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::PermissionDenied => {
            // 权限不足，尝试提权
        }
        Err(e) => return Err(e.into()),
    }

    // 写入临时文件
    let tmp_path = std::env::temp_dir().join("hostz_tmp_hosts");
    std::fs::write(&tmp_path, content)?;

    let result = elevate_copy(&tmp_path, system_path);
    let _ = std::fs::remove_file(&tmp_path);
    result
}

/// 使用系统命令提权复制文件
fn elevate_copy(src: &Path, dst: &Path) -> Result<()> {
    let src_str = src.to_str().unwrap_or("");
    let dst_str = dst.to_str().unwrap_or("");

    if cfg!(target_os = "windows") {
        // Windows: 使用 PowerShell Start-Process -Verb RunAs（隐藏窗口）
        let ps_script = format!(
            "Start-Process -FilePath 'cmd.exe' -ArgumentList '/c copy /Y \"{}\" \"{}\"' -Verb RunAs -Wait -WindowStyle Hidden",
            src_str, dst_str
        );
        let mut cmd = std::process::Command::new("powershell");
        cmd.arg("-NoProfile").arg("-Command").arg(&ps_script);
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW
        }
        let output = cmd.output()
            .map_err(|e| anyhow::anyhow!("PowerShell 启动失败: {}", e))?;

        if !output.status.success() {
            return Err(anyhow::anyhow!(
                "写入系统 hosts 需要管理员权限。请以管理员身份运行 HostZ。\n{}",
                String::from_utf8_lossy(&output.stderr)
            ));
        }
    } else if cfg!(target_os = "macos") {
        let script = format!(
            "do shell script \"cp -f '{}' '{}'\" with administrator privileges",
            src_str.replace('\'', "'\\''"),
            dst_str.replace('\'', "'\\''"),
        );
        let output = Command::new("osascript")
            .arg("-e")
            .arg(&script)
            .output()
            .map_err(|e| anyhow::anyhow!("执行 osascript 失败: {}", e))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow::anyhow!("macOS 提权失败: {}", stderr));
        }
    } else {
        // Linux: 先尝试 pkexec，失败用 sudo
        let output = Command::new("pkexec")
            .arg("cp")
            .arg(src_str)
            .arg(dst_str)
            .output()
            .map_err(|e| anyhow::anyhow!("执行 pkexec 失败: {}", e))?;

        if !output.status.success() {
            let output = Command::new("sudo")
                .arg("cp")
                .arg(src_str)
                .arg(dst_str)
                .output()
                .map_err(|e| anyhow::anyhow!("执行 sudo 失败: {}", e))?;

            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                return Err(anyhow::anyhow!("Linux 提权失败: {}", stderr));
            }
        }
    }

    Ok(())
}


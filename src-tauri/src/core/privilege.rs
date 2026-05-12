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

    // 写入临时文件（随机文件名 + 最小权限防止 TOCTOU 竞态）
    let tmp_name = format!("hostz_{}.tmp", uuid::Uuid::new_v4());
    let tmp_path = std::env::temp_dir().join(tmp_name);
    {
        let mut f = std::fs::File::create_new(&tmp_path)?;
        std::io::Write::write_all(&mut f, content.as_bytes())?;
    }

    let result = elevate_copy(&tmp_path, system_path);
    let _ = std::fs::remove_file(&tmp_path);

    // 提权后验证是否真正写入成功
    if result.is_ok() {
        match std::fs::read_to_string(system_path) {
            Ok(actual) if actual == content => return Ok(()),
            Ok(_) => return Err(anyhow::anyhow!(
                "提权操作未生效，请以管理员身份运行 HostZ"
            )),
            Err(_) => return Err(anyhow::anyhow!(
                "写入后无法读取验证，请以管理员身份运行 HostZ"
            )),
        }
    }
    result
}

/// 使用系统命令提权复制文件
fn elevate_copy(src: &Path, dst: &Path) -> Result<()> {
    let src_str = src.to_str()
        .ok_or_else(|| anyhow::anyhow!("临时文件路径包含非 UTF-8 字符"))?;
    let dst_str = dst.to_str()
        .ok_or_else(|| anyhow::anyhow!("目标路径包含非 UTF-8 字符"))?;

    if cfg!(target_os = "windows") {
        // 写 .bat 文件传递路径，避免环境变量在提权边界丢失
        let bat_name = format!("hostz_{}.bat", uuid::Uuid::new_v4());
        let bat_path = std::env::temp_dir().join(&bat_name);
        let bat_content = format!("@echo off\r\ncopy /Y \"{}\" \"{}\"\r\nexit /b %errorlevel%\r\n", src_str, dst_str);
        std::fs::write(&bat_path, &bat_content)?;

        let bat_str = bat_path.to_str()
            .ok_or_else(|| anyhow::anyhow!("批处理文件路径包含非 UTF-8 字符"))?;
        let ps_script = format!(
            "Start-Process -FilePath 'cmd.exe' -ArgumentList '/c \"{}\"' -Verb RunAs -Wait -WindowStyle Hidden",
            bat_str
        );
        let mut cmd = std::process::Command::new("powershell");
        cmd.arg("-NoProfile").arg("-Command").arg(&ps_script);
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW
        }
        let output = cmd.output();
        // 确保 .bat 文件一定被清理
        let _ = std::fs::remove_file(&bat_path);
        let output = output
            .map_err(|e| anyhow::anyhow!("PowerShell 启动失败: {}", e))?;

        if !output.status.success() {
            return Err(anyhow::anyhow!(
                "写入系统 hosts 需要管理员权限。请以管理员身份运行 HostZ。\n{}",
                String::from_utf8_lossy(&output.stderr)
            ));
        }
    } else if cfg!(target_os = "macos") {
        // 先转义反斜杠再转义单引号，防止 \ 绕过
        fn escape_applescript_path(s: &str) -> String {
            s.replace('\\', "\\\\").replace('\'', "'\\''")
        }
        let script = format!(
            "do shell script \"cp -f '{}' '{}'\" with administrator privileges",
            escape_applescript_path(src_str),
            escape_applescript_path(dst_str),
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


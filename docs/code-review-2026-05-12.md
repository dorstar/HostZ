# 代码审查报告 — src-tauri

> 审查日期：2026-05-12

---

## 目录

1. [安全问题](#1-安全问题)
2. [逻辑问题](#2-逻辑问题)
3. [代码质量问题](#3-代码质量问题)
4. [修复优先级总结](#4-修复优先级总结)

---

## 1. 安全问题

---

### 1.1 临时文件符号链接攻击 — `privilege.rs:18`

**严重程度:** 中

**问题描述:**
临时文件使用 `std::fs::write()` 创建，未使用 `create_new(true)` 标志。如果攻击者在临时文件路径提前创建符号链接，写入的内容会重定向到任意文件，造成 TOCTOU 竞态。

**当前代码:**

```rust
let tmp_path = std::env::temp_dir().join(tmp_name);
std::fs::write(&tmp_path, content)?;
```

**修复方案:**
```rust
use std::io::Write;

let file = OpenOptions::new()
    .write(true)
    .create_new(true)
    .open(&tmp_path)
    .map_err(|e| anyhow::anyhow!("创建临时文件失败: {}", e))?;
file.write_all(content.as_bytes())
    .map_err(|e| anyhow::anyhow!("写入临时文件失败: {}", e))?;
```

---

### 1.2 PowerShell 命令注入 — `privilege.rs:44-50`

**严重程度:** 中-高

**问题描述:**
PowerShell 脚本中直接拼接文件路径。特殊字符（`"`、`` ` ``、`$` 等）可突破上下文限制，执行任意命令。

**当前代码:**

```rust
let ps_script = format!(
    "Start-Process -FilePath 'cmd.exe' -ArgumentList '/c copy /Y \"{}\" \"{}\"' -Verb RunAs -Wait -WindowStyle Hidden",
    src_str, dst_str
);
```

**修复方案:**

添加转义函数并对路径特殊字符做处理：
```rust
fn escape_powershell_arg(s: &str) -> String {
    // PowerShell 特殊字符转义顺序：反引号 > 双引号 > $
    s.replace('`', "``")
     .replace('"', "`\"")
     .replace('$', "`$")
}

// 使用
let src_escaped = escape_powershell_arg(src_str);
let dst_escaped = escape_powershell_arg(dst_str);

let ps_script = format!(
    "Start-Process -FilePath 'cmd.exe' -ArgumentList '/c copy /Y \"{src_escaped}\" \"{dst_escaped}\"' -Verb RunAs -Wait -WindowStyle Hidden"
);
```

---

### 1.3 macOS AppleScript 转义 — `privilege.rs:66-70`

**严重程度:** 中

**问题描述:**
AppleScript 单引号转义不完整。路径中含 `\` 时可能绕过转义逻辑。

**当前代码:**

```rust
let script = format!(
    "do shell script \"cp -f '{}' '{}'\" with administrator privileges",
    src_str.replace('\'', "'\\''"),
    dst_str.replace('\'', "'\\''"),
);
```

**修复方案:**

先转义反斜杠，再转义单引号，确保不会被反向截断：
```rust
fn escape_applescript_path(s: &str) -> String {
    s.replace('\\', "\\\\").replace('\'', "'\\''")
}

let script = format!(
    "do shell script \"cp -f '{}' '{}'\" with administrator privileges",
    escape_applescript_path(src_str),
    escape_applescript_path(dst_str),
);
```

---

### 1.4 路径遍历 — `main.rs:263-274`

**严重程度:** 中

**问题描述:**
`export_to_file` 只检查父目录是否存在，未验证最终路径是否在允许范围内。攻击者可利用相对路径或符号链接写入任意位置。

**当前代码:**

```rust
fn export_to_file(state: tauri::State<AppState>, path: String) -> Result<(), String> {
    let p = std::path::Path::new(&path);
    if let Some(parent) = p.parent() {
        if !parent.exists() {
            return Err("目标目录不存在".to_string());
        }
    }
    // 无路径校验直接写入
    std::fs::write(&path, &content).map_err(|e| e.to_string())
}
```

**修复方案:**

```rust
fn export_to_file(state: tauri::State<AppState>, path: String) -> Result<(), String> {
    let allowed_base = hostz::utils::platform::app_data_dir();
    let p = std::path::Path::new(&path);

    // 规范化路径，解析符号链接
    let canonical = p.canonicalize()
        .map_err(|e| format!("路径无效: {}", e))?;

    // 验证路径在允许范围内
    if !canonical.starts_with(&allowed_base) {
        return Err("不允许写入此路径".to_string());
    }

    // 只允许导出 JSON
    if canonical.extension().map_or(true, |e| e != "json") {
        return Err("仅允许导出 JSON 文件".to_string());
    }

    let swhdb = state.swhdb.lock().map_err(|e| e.to_string())?;
    let json = swhdb.to_json().map_err(|e| e.to_string())?;
    let content = serde_json::to_string_pretty(&json).map_err(|e| e.to_string())?;
    std::fs::write(&canonical, &content).map_err(|e| e.to_string())
}
```

---

### 1.5 HTTP 请求无证书验证 — `hosts_manager.rs:101-109`

**严重程度:** 低

**问题描述:**
`ureq` 请求远程 hosts 内容时未配置 TLS 证书验证，中间人可篡改返回内容。

**当前代码:**

```rust
fn fetch_remote_content(url: &str) -> Result<String> {
    let response = ureq::get(url)
        .set("User-Agent", &format!("HostZ/{}", env!("CARGO_PKG_VERSION")))
        .timeout(std::time::Duration::from_secs(30))
        .call()
        .map_err(|e| anyhow::anyhow!("HTTP 请求失败: {}", e))?;
    response.into_string()
        .map_err(|e| anyhow::anyhow!("读取响应失败: {}", e))
}
```

**修复方案:**

强制 HTTPS 并限制内容大小（5MB），防止 ZIP 炸弹：
```rust
fn fetch_remote_content(url: &str) -> Result<String> {
    // 仅允许 HTTPS
    if !url.starts_with("https://") {
        return Err(anyhow::anyhow!("仅支持 HTTPS 协议"));
    }

    let max_size: u64 = 5 * 1024 * 1024; // 5MB

    let response = ureq::get(url)
        .set("User-Agent", &format!("HostZ/{}", env!("CARGO_PKG_VERSION")))
        .timeout(std::time::Duration::from_secs(30))
        .call()
        .map_err(|e| anyhow::anyhow!("HTTP 请求失败: {}", e))?;

    // 检查 Content-Length
    if let Some(len) = response.header("Content-Length")
        .and_then(|v| v.parse::<u64>().ok())
    {
        if len > max_size {
            return Err(anyhow::anyhow!("文件过大（{} bytes > {} 限制）", len, max_size));
        }
    }

    // 限制读取大小
    let mut buf: Vec<u8> = Vec::new();
    response.into_reader()
        .take(max_size)
        .read_to_end(&mut buf)
        .map_err(|e| anyhow::anyhow!("读取响应失败: {}", e))?;

    String::from_utf8(buf).map_err(|e| anyhow::anyhow!("响应不是合法 UTF-8: {}", e))
}
```

---

## 2. 逻辑问题

---

### 2.1 提权验证逻辑错误 — `hosts_manager.rs:24-36`

**严重程度:** 高

**问题描述:**
当 `result.is_ok()` 且验证通过时正确返回 `Ok(())`，但最后的 `result` 在分支外，控制流不够清晰，容易引入错误。如果添加了新的分支路径，可能意外走到错误的返回值。

**当前代码:**

```rust
if result.is_ok() {
    match std::fs::read_to_string(system_path) {
        Ok(actual) if actual == content => return Ok(()),
        Ok(_) => return Err(anyhow::anyhow!("提权操作未生效，请以管理员身份运行 HostZ")),
        Err(_) => return Err(anyhow::anyhow!("写入后无法读取验证")),
    }
}
result
```

**修复方案:**

```rust
if result.is_err() {
    return result;
}

let actual = std::fs::read_to_string(system_path)
    .map_err(|e| anyhow::anyhow!("写入后无法读取验证: {}", e))?;

if actual == content {
    Ok(())
} else {
    Err(anyhow::anyhow!("提权操作未生效，请以管理员身份运行 HostZ"))
}
```

---

### 2.2 竞态条件 — `cron.rs:39-85` & `hosts_manager.rs:149-180`

**严重程度:** 中

**问题描述:**
- cron 的 `check_refresh` 先收集 ID 列表（锁内），然后逐个刷新（锁释放），最后加锁应用到系统。期间用户可能修改了列表状态，导致操作基于过时数据。
- `refresh_remote_async` 也有类似的先读后写问题。

**当前代码（cron.rs）:**

```rust
// 第一步：锁内收集 ID
let ids_to_refresh = {
    let db = swhdb.lock().map_err(|e| e.to_string())?;
    let list = db.get_list()?;
    // 遍历收集符合条件的 ID
    ids
};

// 第二步：逐个刷新（无锁，存在窗口）
for id in &ids_to_refresh {
    hosts_manager::refresh_remote_async(swhdb, id)?;
}

// 第三步：重新锁住应用到系统
if any_refreshed {
    let db = swhdb.lock().map_err(|e| e.to_string())?;
    let cfg = cfgdb.lock().map_err(|e| e.to_string())?;
    hosts_manager::apply_to_system(&db, &cfg)?;
}
```

**修复方案 A — 列表版本号（推荐）:**

在 `SwhDb` 中添加版本计数器：
```rust
// swhdb.rs 中添加
pub fn get_list_version(&self) -> u64 {
    self.list_version
}
```

cron 中验证版本未变更：
```rust
let (ids, version) = {
    let db = swhdb.lock().map_err(|e| e.to_string())?;
    let list = db.get_list().map_err(|e| e.to_string())?;
    let version = db.get_list_version();
    let ids: Vec<String> = list.iter()
        .filter(|item| /* 刷新条件 */)
        .map(|item| item.id.clone())
        .collect();
    (ids, version)
};

if ids.is_empty() { return Ok(()); }

// 刷新时检查版本
let mut any_refreshed = false;
for id in &ids {
    {
        let db = swhdb.lock().map_err(|e| e.to_string())?;
        if db.get_list_version() != version {
            return Ok(()); // 列表已变更，放弃本次自动应用
        }
    }
    if hosts_manager::refresh_remote_async(swhdb, id).is_ok() {
        any_refreshed = true;
    }
}
```

**修复方案 B — 一次性锁定:**
```rust
fn check_refresh(
    swhdb: &Arc<Mutex<SwhDb>>,
    cfgdb: &Arc<Mutex<CfgDb>>,
    app_handle: &AppHandle,
) -> Result<(), String> {
    // 尽量缩短锁持有时间：锁内只筛选和刷新
    let any_refreshed = {
        let db = swhdb.lock().map_err(|e| e.to_string())?;

        // 一次性把所有满足条件的远程条目就地刷新
        let ids = collect_refresh_ids(&db)?;
        let mut any = false;
        for id in &ids {
            // 获取 URL 后释放 swhdb 锁去网络请求
            let url = db.get_list().ok()
                .and_then(|list| find_item_by_id(&list, id))
                .and_then(|item| item.url.clone());

            if let Some(url) = url {
                drop(db); // 释放锁再去请求
                if let Ok(content) = fetch_remote_content(&url) {
                    let db = swhdb.lock().map_err(|e| e.to_string())?;
                    db.set_content(id, &content).ok();
                    // 更新 last_refresh_ms...
                    any = true;
                }
                db = swhdb.lock().map_err(|e| e.to_string())?;
            }
        }
        any
    };

    // 应用系统 hosts
    if any_refreshed {
        let db = swhdb.lock().map_err(|e| e.to_string())?;
        let cfg = cfgdb.lock().map_err(|e| e.to_string())?;
        if let Err(e) = hosts_manager::apply_to_system(&db, &cfg) {
            eprintln!("Cron 应用到系统失败: {}", e);
        }
    }
    Ok(())
}
```

---

### 2.3 临时文件清理静默失败 — `privilege.rs:21`

**严重程度:** 低

**问题描述:**
临时文件删除失败被 `let _ =` 静默忽略。多次崩溃后可能积累大量 `hostz_*.tmp` 残留文件。

**当前代码:**
```rust
let _ = std::fs::remove_file(&tmp_path);
```

**修复方案:**

```rust
if let Err(e) = std::fs::remove_file(&tmp_path) {
    eprintln!("清理临时文件失败: {}. 路径: {}", e, tmp_path.display());
}
```

同时启动时清理过期临时文件：
```rust
// 应用启动时（如 main.rs 的 platform::ensure_data_dirs() 之后）
fn cleanup_stale_temp_files() {
    if let Ok(entries) = std::fs::read_dir(std::env::temp_dir()) {
        for entry in entries.flatten() {
            let name = entry.file_name();
            let name_str = name.to_string_lossy();
            if name_str.starts_with("hostz_") && name_str.ends_with(".tmp") {
                let _ = std::fs::remove_file(entry.path());
            }
        }
    }
}
```

---

### 2.4 `history_limit` 无边界检查 — `swhdb.rs:191`

**严重程度:** 低

**问题描述:**
`get_history(limit)` 接受 `i32`，SQLite 的 LIMIT 是 64 位整数。传入负数或极大值可能导致不期望的行为。

**当前代码:**
```rust
pub fn get_history(&self, limit: i32) -> Result<Vec<HostsHistoryObject>> {
    let mut stmt = self.conn.prepare("SELECT ... LIMIT ?1")?;
    let rows = stmt.query_map([limit], |row| { ... })?;
}
```

**修复方案:**

```rust
pub fn get_history(&self, limit: i32) -> Result<Vec<HostsHistoryObject>> {
    const MAX_HISTORY_LIMIT: i32 = 10000;
    let safe_limit = if limit <= 0 { 50 } else { limit.min(MAX_HISTORY_LIMIT) };

    let mut stmt = self.conn.prepare("SELECT ... LIMIT ?1")?;
    let rows = stmt.query_map([safe_limit], |row| { ... })?;
    // ...
}
```

---

### 2.5 搜索边界条件可能 panic — `search.rs:121`

**严重程度:** 中

**问题描述:**
`content[start..end]` 切片操作在索引越界时会导致 panic。

**当前代码片段:**
```rust
let match_text = content[start..end].to_string();
```

**修复方案:**

```rust
if start > content.len() || end > content.len() || start > end {
    return None;
}
let match_text = content[start..end].to_string();
```

---

### 2.6 配置 JSON 解析失败静默忽略 — `cfgdb.rs:117-123`

**严重程度:** 低

**问题描述:**
JSON 解析失败时静默回退为字符串，配置损坏不会被发现。

**当前代码:**
```rust
let json_val = serde_json::from_str(value)
    .unwrap_or_else(|_| serde_json::Value::String(value.clone()));
```

**修复方案:**
```rust
let json_val = serde_json::from_str(value)
    .unwrap_or_else(|e| {
        eprintln!("配置项 `{}` JSON 解析失败: {}。值: {}", key, e, value);
        serde_json::Value::String(value.clone())
    });
```

---

## 3. 代码质量问题

---

### 3.1 `unwrap()` 导致 panic — `main.rs:370`

**严重程度:** 中

**问题描述:**
锁被 Poisoned 时 `unwrap()` 会直接 panic，使整个进程崩溃。

**当前代码:**
```rust
let cfgdb = state.cfgdb.lock().unwrap();
```

**修复方案:**
```rust
let cfgdb = state.cfgdb.lock()
    .map_err(|e| anyhow::anyhow!("配置数据库锁失败: {}", e))?;
```

（代码中其他地方也有类似问题，例如 cron 中收集 ID 时用了 `map_err(|e| e.to_string())?` 是正确的用法，应保持一致。）

---

### 3.2 `AppState` 不应实现 `Default` — `main.rs:11-15`

**严重程度:** 低

**问题描述:**
`AppState` 如果通过 derive 实现了 `Default`，`HotkeyManager` 可能在未初始化状态下被调用。建议显式禁止 `Default`。

**修复方案:**
```rust
pub struct AppState {
    pub swhdb: Arc<Mutex<SwhDb>>,
    pub cfgdb: Arc<Mutex<CfgDb>>,
    pub hotkey_manager: hostz::services::hotkey::HotkeyManager,
}
```
检查确保它没有 `#[derive(Default)]`。如果 Tauri 的 `manage()` 要求 `Default`，则在 `new()` 方法中显式创建。

---

### 3.3 `get_history` 参数类型安全 — `swhdb.rs:191`

**严重程度:** 低

**问题描述:**
`i32` 作为 `LIMIT` 参数，但 SQLite `LIMIT` 需要 64 位整型。

**修复方案:**
```rust
pub fn get_history(&self, limit: i32) -> Result<Vec<HostsHistoryObject>> {
    let limit_i64: i64 = if limit <= 0 { 50 } else { limit.min(10000) as i64 };
    let mut stmt = self.conn.prepare("SELECT ... LIMIT ?1")?;
    let rows = stmt.query_map([limit_i64], |row| { ... })?;
    // ...
}
```

---

### 3.4 硬编码中文字符串 — `hosts_manager.rs:51-54`

**严重程度:** 低

**问题描述:**
错误消息和标签硬编码为中文，项目已有 `i18n.rs` 但未使用。

**当前代码:**
```rust
let mode_label = match write_mode {
    WriteMode::Append => "追加模式",
    WriteMode::Overwrite => "覆盖模式",
};
```

**修复方案:**

使用国际化系统（如果已存在）或提取为常量供未来 i18n 使用：
```rust
// 临时：使用英文作为通用语言，等 i18n 机制完善
let mode_label = match write_mode {
    WriteMode::Append => "append",
    WriteMode::Overwrite => "overwrite",
};
```
或者在所有用户可见的错误消息处统一使用 i18n 工具函数。

---

## 4. 修复优先级总结

| 优先级 | 问题 | 影响 | 文件 |
|--------|------|------|------|
| **P0** | 提权验证逻辑错误 | 导致假成功或假失败 | `hosts_manager.rs:24-36` |
| **P0** | `unwrap()` panic | 应用直接崩溃 | `main.rs:370` 等 |
| **P1** | PowerShell 命令注入 | 任意代码执行（受限条件下） | `privilege.rs:44-50` |
| **P1** | 临时文件符号链接攻击 | 文件写入劫持 | `privilege.rs:18` |
| **P1** | 搜索边界 panic | 极端输入导致崩溃 | `search.rs:121` |
| **P2** | 竞态条件（cron） | 数据不一致 | `cron.rs:39-85` |
| **P2** | 路径遍历 | 文件写入绕过 | `main.rs:263-274` |
| **P2** | macOS 转义不完整 | 注入风险 | `privilege.rs:66-70` |
| **P3** | HTTP 无证书验证 | MITM 攻击 | `hosts_manager.rs:101-109` |
| **P3** | 临时文件清理静默失败 | 磁盘残留 | `privilege.rs:21` |
| **P3** | 边界检查缺失（history_limit） | 潜在异常 | `swhdb.rs:191` |
| **P4** | JSON 解析失败静默 | 调试困难 | `cfgdb.rs:117-123` |
| **P4** | 硬编码字符串 | 国际化困难 | `hosts_manager.rs:51-54` |
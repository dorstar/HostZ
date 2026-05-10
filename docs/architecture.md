# HostZ — 架构设计文档

> 基于 SwitchHosts 4.3.0 的 Rust + Tauri v2 轻量化重写

## 0. 项目目标

| 指标 | 原版 (Electron) | 目标 (Tauri v2) |
|------|----------------|-----------------|
| 打包体积 | >100 MB | **<15 MB** |
| 内存占用 | ~200 MB | **<50 MB** |
| 启动时间 | ~3s | **<1s** |
| 核心功能 | 完整 | **完整保留** |

## 1. 技术栈

| 层 | 原版 | HostZ |
|---|------|-------|
| 应用壳 | Electron 30 | Tauri v2 |
| 前端 | React + Mantine + Jotai | **纯 HTML/CSS/JS**（无框架，无 WASM） |
| 编辑器 | CodeMirror 6 | textarea（编辑）+ pre 高亮（只读） |
| 图标 | 内联 SVG | **Octicons 16px SVG（CSS mask-image）** |
| 数据库 | PotDB (JSON 文件) | SQLite (rusqlite, bundled) |
| HTTP 客户端 | axios | ureq |
| HTTP 服务 | Express / Hono | tiny_http |
| 调度器 | setInterval | std::thread + AtomicBool |
| 系统托盘 | Electron Tray | Tauri tray-icon |
| 桌面通知 | Electron Notification | notify-rust |
| IPC | ipcMain/ipcRenderer | #[tauri::command] + emit |

**禁止引入**: tokio, axum, actix-web, warp, reqwest, diesel, sqlx, egui, dioxus

## 2. 项目结构

```
HostZ/
├── dist/                    # 前端静态资源（纯 HTML/CSS/JS）
│   ├── index.html           # 应用外壳 (~150 行)
│   ├── style.css            # 全局样式 + 深色主题 (~530 行)
│   ├── app.js               # 全部前端逻辑 (~780 行)
│   └── icons/               # Octicons SVG 图标 (16 个)
├── src-tauri/               # 后端源码 (Rust)
│   ├── Cargo.toml           # 依赖: tauri, rusqlite, ureq, tiny_http 等
│   ├── tauri.conf.json      # Tauri 窗口/打包配置 (withGlobalTauri: true)
│   ├── build.rs             # tauri_build
│   ├── capabilities/
│   │   └── default.json     # IPC 权限声明 (9 项)
│   ├── icons/               # 应用图标 (32x32, 128x128, .ico, .icns)
│   ├── src/
│   │   ├── main.rs          # Tauri 入口 + 27 条命令
│   │   ├── lib.rs           # 模块声明
│   │   ├── db/              # SQLite 数据层
│   │   │   ├── swhdb.rs     # hosts 数据 (5 表, 含导入/导出)
│   │   │   └── cfgdb.rs     # 配置管理 (1 表, 支持部分更新)
│   │   ├── models/
│   │   │   ├── hosts.rs     # 数据模型 (8 个 struct/enum)
│   │   │   └── config.rs    # 配置模型 (~25 项, 含默认值)
│   │   ├── core/
│   │   │   ├── hosts_manager.rs  # hosts 读写 + 系统应用 + 远程刷新
│   │   │   ├── content_parser.rs # 递归内容解析 + 树操作 (9 个函数)
│   │   │   ├── trash.rs          # 回收站 CRUD (+ 关联 ID 收集)
│   │   │   ├── search.rs         # 正则查找替换 (防跨行 panic)
│   │   │   └── privilege.rs      # 提权写入 (Win: PS RunAs, Mac: osascript, Linux: pkexec/sudo)
│   │   ├── services/
│   │   │   ├── cron.rs           # 后台调度器 (60s 检查远程刷新)
│   │   │   ├── http_api.rs       # REST API (tiny_http, 5 个路由)
│   │   │   ├── tray.rs           # 系统托盘 (右键菜单 6 项)
│   │   │   ├── hotkey.rs         # 快捷键管理 (定义 4 组)
│   │   │   ├── update.rs         # 更新检查 (每小时, 60s 轮询)
│   │   │   └── notification.rs   # 桌面通知 (notify-rust)
│   │   └── utils/
│   │       ├── platform.rs       # 平台路径 + 数据目录
│   │       └── i18n.rs           # 国际化 (占位)
│   └── tests/
│       └── integration_test.rs   # 集成测试 (19 个)
├── docs/
│   └── architecture.md    # 本文档
└── reference/             # 原版源码参考
    ├── SwitchHosts-master/
    └── screenshots/
```

## 3. 数据模型

### 3.1 HostsListObject（核心）

```rust
pub struct HostsListObject {
    pub id: String,                    // UUID
    pub title: String,                 // 显示名
    #[serde(rename = "type")]
    pub type_: HostsType,             // local | remote | group | folder
    pub on: bool,                      // 启用状态
    pub content: Option<String>,       // 内容 (local 直接存储)
    pub url: Option<String>,           // 远程 URL
    pub refresh_interval: Option<i64>, // 刷新间隔 (秒)
    pub last_refresh_ms: Option<i64>,  // 上次刷新时间戳
    pub include: Option<Vec<String>>,  // 组引用的 ID 列表
    pub children: Option<Vec<HostsListObject>>, // 文件夹子项
    pub folder_mode: Option<FolderMode>, // 0=多选 1=单选 2=明确多选
    pub order: i32,                    // 排序
}
```

### 3.2 四种类型行为

| 类型 | 内容来源 | 可编辑 | 特有字段 |
|------|---------|--------|---------|
| `local` | 用户编辑 | ✅ | content |
| `remote` | HTTP 拉取 | ❌ | url, refresh_interval, last_refresh_ms |
| `group` | 聚合 include[] 中的条目 | ❌ | include |
| `folder` | 聚合 children 中的条目 | ❌ | children, folder_mode |

### 3.3 数据库表设计 (SQLite)

```sql
-- hosts 内容存储
CREATE TABLE hosts_content (id TEXT PRIMARY KEY, content TEXT NOT NULL);

-- 树形列表 (JSON 序列化存储，含 children)
CREATE TABLE list_tree (id TEXT PRIMARY KEY, parent_id TEXT,
    order_idx INTEGER DEFAULT 0, data TEXT NOT NULL);

-- 回收站
CREATE TABLE trashcan (id TEXT PRIMARY KEY, data TEXT NOT NULL,
    add_time_ms INTEGER NOT NULL);

-- 系统 hosts 写入历史
CREATE TABLE history (id TEXT PRIMARY KEY, content TEXT NOT NULL,
    add_time_ms INTEGER NOT NULL, label TEXT);

-- 元数据
CREATE TABLE meta (key TEXT PRIMARY KEY, value TEXT NOT NULL);
```

**数据目录**: `%APPDATA%/HostZ/data/hostz.db`

### 3.4 配置模型 (~25 项)

```rust
pub struct AppConfig {
    // UI
    pub left_panel_show: bool,          // 默认 true
    pub left_panel_width: i32,          // 默认 270
    pub use_system_window_frame: bool,
    // 偏好
    pub write_mode: WriteMode,         // Append (默认) | Overwrite
    pub history_limit: i32,            // 默认 50
    pub locale: Option<String>,
    pub theme: Theme,                   // Light | Dark | System
    pub choice_mode: i32,              // 默认 2
    pub show_title_on_tray: bool,
    pub hide_at_launch: bool,
    pub remove_duplicate_records: bool,
    pub cmd_after_hosts_apply: Option<String>,
    // HTTP API
    pub http_api_on: bool,             // 默认 false
    pub http_api_only_local: bool,     // 默认 true
    pub http_api_port: u16,            // 默认 50761
    // 代理
    pub use_proxy: bool,
    pub proxy_protocol: Option<ProxyProtocol>,
    pub proxy_host: Option<String>,
    pub proxy_port: Option<u16>,
    // 托盘
    pub tray_mini_window: bool,        // 默认 true
    pub multi_chose_folder_switch_all: bool,
    // 更新
    pub auto_download_update: bool,    // 默认 true
}
```

**配置存储**: `%APPDATA%/HostZ/config/cfg.db`

## 4. 核心算法

### 4.1 内容递归解析

```
get_content_of_hosts(db, list, id):
  item = find_item_by_id(list, id)
  match item.type:
    local/remote → db.get_content(id)
    group → for each inc_id in item.include:
               "# file: {title}\n" + get_content_of_hosts(inc_id)
             join all with "\n\n"
    folder → for each child in item.children:
               "# file: {title}\n" + get_content_of_hosts(child.id)
             join all with "\n\n"
```

### 4.2 系统 hosts 写入

**Append 模式** (默认):
1. 读取当前系统 hosts 文件
2. 查找标记 `# --- SWITCHHOSTS_CONTENT_START ---`
3. 保留标记之前的内容，替换标记之后的内容
4. 若未找到标记，在文件末尾追加标记+新内容

**Overwrite 模式**: 直接覆盖整个文件。

**提权写入**: 先尝试直接写入 → 失败则写入临时文件 → `runas`(Win) / `pkexec`→`sudo`(Linux) / `osascript`(macOS) → 复制到系统路径 → 清理临时文件

### 4.3 开关逻辑 (set_on_state_of_item)

```
toggle(id, on):
  item = find_mut(list, id)
  item.on = on

  if multi_chose_folder_switch_all:
    if folder: cascade to all children
    propagate up to parent (parent.on = all children are on)

  if !on: return  // 关闭时不处理互斥

  if choice_mode == 1 (single):  // 顶层单选
    turn off all other top-level items

  if parent.folder_mode == Single:   // 文件夹内单选
    turn off all siblings
```

### 4.4 远程同步

```
cron (每 60 秒):
  for each remote item where on == true:
    if refresh_interval > 0 AND url starts with http:
      if (now_ms - last_refresh_ms) / 1000 >= refresh_interval:
        fetch HTTP GET → update hosts_content
        emit "hosts_refreshed", "reload_list"
        apply_to_system()
```

## 5. IPC 通信 (Tauri v2 映射)

| 原版 | HostZ Tauri v2 |
|------|---------------|
| `_agent.call('action', params)` | `window.__TAURI__.core.invoke('command', args)` |
| `_agent.broadcast('event', data)` | `app_handle.emit("event", payload)` |
| `_agent.on('event', handler)` | `window.__TAURI__.event.listen("event", cb)` |

**关键配置**: `withGlobalTauri: true` (使 `window.__TAURI__` 全局可用)

**Capabilities**: `src-tauri/capabilities/default.json` — 必须声明 `core:default` 等权限

## 6. 前端 UI 设计

### 6.1 页面路由

```
App
├── Home (主页)          ← 默认，左侧树 + 右侧编辑器
├── Find (查找替换)       ← 托盘触发 / 快捷键 Ctrl+F
├── Settings (设置)       ← 托盘"偏好设置"
└── QuickToggle (快速切换) ← 托盘"快速切换"
```

### 6.2 主页面布局 (Grid)

```
┌──────────────────────────────────────────────────────┐
│ TopBar (40px)    [◀] [+]  HostName [只读]    [⚙] [✕]│
├──────────┬───────────────────────────────────────────┤
│          │ Toolbar (28px) [应用到系统] [保存] [查找]   │
│ Left     ├───────────────────────────────────────────┤
│ Panel    │ Editor Area (flex: 1)                     │
│ (260px)  │ monospace, 14px/1.8em, pre-wrap            │
│          │                                           │
│ 📁Folder │                                           │
│   📄Item │                                           │
│ 🌐Remote │                                           │
│ 📋Group  │                                           │
│          ├───────────────────────────────────────────┤
│ 🗑️Trash  │ StatusBar (22px)  12 行 | 256 B | 只读      │
└──────────┴───────────────────────────────────────────┘
```

### 6.3 树节点

```
[▶/▼] [icon] Title                    [●○ switch]
 ├── 20px 缩进
 └── grid: 20px(arrow) + 1fr(label) + auto(switch)
```

- Hover: 背景 `#f2f1f5`
- Selected: 背景 `#cbdef6`
- Switch: 23×13px pill, 绿色 ON / 灰色 OFF, 0.3s 过渡

### 6.4 配色方案 (Light)

| 元素 | 色值 |
|------|------|
| 主色调 | `#007aff` |
| 左面板 | `#edebf1` |
| 主背景 | `#fff` |
| 状态栏 | `#f0f1f1` (背景) `#999` (文字) |
| 编辑器 IP | `#096dd9` |
| 编辑器注释 | `#090` |
| 开关 ON | `#91d982` |
| 树选中 | `#cbdef6` |

### 6.5 查找页

```
┌──────────────────────────────┐
│ [←] 查找替换                   │
│ [________________] [查找]     │ ← 无边框输入 (border-bottom)
│ [________________] [替换全部]  │
│ ☐正则 ☐忽略大小写    3 个匹配   │
│ ──────────────────────────── │
│ match content  │ title │ Ln │ ← 三列 grid
│ match content  │ 📄 item│ 5  │
│ ...                          │
└──────────────────────────────┘
```

### 6.6 设置页

分区: 外观 → Hosts 写入 → HTTP API → 托盘 → 更新
每行: `label(120px) + input(160px)`, 13px 字体

## 7. HTTP API (极轻量)

| 方法 | 路径 | 响应 |
|------|------|------|
| GET | `/` | `"Hello HostZ!"` |
| GET | `/remote-test` | `{"time":"...","content":"#..."}` |
| GET | `/api/hosts` | `{"success":true,"data":[...]}` |
| POST | `/api/hosts/{id}/toggle` | `{"id":"...","on":true}` |
| POST | `/api/hosts/{id}/refresh` | `{"id":"...","refreshed":true}` |

- 默认端口: 50761
- 默认绑定: 127.0.0.1
- 无认证，CORS `*`

## 8. 系统托盘

```
右键菜单:
├── 显示主窗口
├── 快速切换      → 迷你 hosts 开关列表
├── 偏好设置      → 设置页面
├── 检查更新
└── 退出
```

左键: 点击图标 → 显示/隐藏主窗口

## 9. 配置变更副作用

| 配置项 | 副作用 |
|--------|--------|
| `theme` | 广播 `config_theme_changed` |
| `locale` | 广播 `config_locale_changed` |
| `http_api_on/port/only_local` | 重启 HTTP API 服务器 |
| `show_title_on_tray` | 广播 `config_tray_title_changed` |
| `hide_dock_icon` | macOS: `set_visible_on_all_workspaces` |
| 任意变更 | 广播 `config_updated` |

## 10. 数据流

```
启动 → ensure_data_dirs() → SwhDb::open() + CfgDb::open()
     → tray创建 → 按配置启动HTTP API → cron启动
前端加载 → mount_to(#app) → HomePage
         → invoke("get_basic_data") → 列表+回收站
         → invoke("config_all") → 配置
用户操作 → invoke("toggle_hosts") → set_on_state_of_item
         → apply_to_system → 写入系统hosts
Cron → check_refresh() → refresh_remote() → apply_to_system
     → emit("reload_list") → 前端重新加载
```

## 11. 构建与部署

```bash
# 后端编译检查
cd src-tauri && cargo check

# 运行全部测试
cd src-tauri && cargo test

# 最终打包（编译 + 签名 + NSIS 安装包）
cd src-tauri && cargo tauri build
# 输出: src-tauri/target/release/bundle/nsis/HostZ_0.1.0_x64-setup.exe
# 二进制: src-tauri/target/release/hostz.exe (~10 MB)
```

### 11.1 27 条 Tauri 命令

| 类别 | 命令 |
|------|------|
| 基础 | `ping`, `get_basic_data` |
| hosts 内容 | `get_hosts_content`, `set_hosts_content` |
| 列表管理 | `get_list`, `set_list`, `add_item` |
| 开关与应用 | `toggle_hosts`, `apply_to_system`, `get_system_hosts_content`, `set_system_hosts_content` |
| 远程同步 | `refresh_remote` |
| 配置 | `config_get`, `config_all`, `config_update` |
| 回收站 | `move_to_trashcan`, `restore_from_trashcan`, `permanently_delete`, `clear_trashcan`, `get_trashcan_list` |
| 历史 | `get_history`, `clear_history` |
| 导入导出 | `export_data`, `import_data` |
| 搜索 | `find_by`, `find_and_replace_all` |
| 快捷键 | `get_hotkeys` |

## 12. 测试

```bash
cargo test  # 32 tests: 13 单元 + 19 集成
```

| 类别 | 数量 | 覆盖 |
|------|------|------|
| content_parser | 4 | flatten, find, delete, toggle_single_choice |
| hosts_manager | 2 | append_marker, append_no_marker |
| trash | 2 | collect_ids_nested, collect_ids_group |
| search | 5 | literal, case_insensitive, multiline, replace_one, replace_all |
| 集成 (原有) | 9 | CRUD, 递归解析, 回收站流程, 搜索替换, 远程刷新, 开关, 配置, 已启用过滤, 路径 |
| 集成 (新增) | 10 | camelCase 配置键, CRLF 规范化, 导入/导出往返, 无效 JSON 安全, 多行正则, 扁平一致性, 回收站关联 ID, 空内容搜索, 配置读写, 历史裁剪 |

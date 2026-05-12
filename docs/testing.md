# HostZ 全流程测试文档

> 最后更新：2026-05-12

## 1. 测试环境

| 项目 | 说明 |
|------|------|
| 操作系统 | Windows 11 Pro 10.0.26200 |
| Rust 版本 | rustc 1.95.0 |
| Tauri 版本 | 2.11.1 |
| 前端技术 | 纯 HTML/CSS/JS（无框架） |
| 图标方案 | Octicons 16px SVG（CSS mask-image） |
| 远程测试 URL | `https://raw.hellogithub.com/hosts` |

## 2. 架构总览

```
HostZ
├── 前端 (dist/)
│   ├── index.html   — 应用外壳 (150 行)
│   ├── style.css    — 全局样式 + 深色主题 (550 行)
│   ├── app.js       — 全部前端逻辑 (~1060 行)
│   └── icons/       — Octicons SVG (16 个)
│
├── 后端 (src-tauri/src/)
│   ├── models/      — hosts.rs, config.rs
│   ├── db/          — swhdb.rs (hostz.db), cfgdb.rs (cfg.db)
│   ├── core/        — hosts_manager, content_parser, trash, search, privilege
│   ├── services/    — tray, hotkey, cron, update, notification
│   └── utils/       — platform, i18n
│
└── 测试
    ├── 单元测试 (src/core/*_test) — 17 个
    └── 集成测试 (tests/)          — 21 个
```

## 3. 支持的类型

| 类型 | 内容来源 | 可编辑 | 特有字段 |
|------|---------|--------|---------|
| `local` | 用户编辑 | ✅ | — |
| `remote` | HTTP 拉取 | ❌ | url, refresh_interval, last_refresh_ms |
| `group` | 聚合 include[] 条目 | ❌ | include（多选列表） |

> 注：`folder` 类型已移除。原文件夹功能由"组合"(group) 替代。

## 4. 单元测试

**17 个，全部通过**

### 4.1 content_parser（9 个）

| 测试 | 验证点 |
|------|--------|
| `test_flatten` | 扁平列表展开 |
| `test_find_item_by_id` | 按 ID 查找条目 |
| `test_delete_item_by_id` | 按 ID 删除后列表长度 |
| `test_set_on_state_single_choice` | choice_mode=1 互斥逻辑（单选关其他） |
| `test_remove_duplicate_lines_basic` | 去除重复 (IP, hostname) 行 |
| `test_remove_duplicate_lines_keeps_comments` | 注释行不参与去重 |
| `test_remove_duplicate_lines_different_ip_same_host` | 不同 IP 同主机名均保留 |
| `test_remove_duplicate_lines_empty` | 空字符串处理 |
| `test_remove_duplicate_lines_preserves_newlines` | 空行与尾部换行保留 | |

### 4.2 hosts_manager（2 个）

| 测试 | 验证点 |
|------|--------|
| `test_make_append_content_no_marker` | 无标记时追加 `MARKER + 内容` |
| `test_make_append_content_with_marker` | 有标记时替换后续内容，标记唯一 |

### 4.3 trash（1 个）

| 测试 | 验证点 |
|------|--------|
| `test_collect_all_ids_group` | 收集条目 + include 引用 ID |

### 4.4 search（5 个）

| 测试 | 验证点 |
|------|--------|
| `test_find_positions_literal` | 字面匹配返回位置/行号 |
| `test_find_positions_case_insensitive` | 忽略大小写匹配 |
| `test_find_multiline_positions` | 多行匹配行号正确 |
| `test_replace_one` | 单个替换 |
| `test_replace_all` | 全局替换全部匹配 |

## 5. 集成测试

**21 个，全部通过**

### 5.1 原有测试（9 个）

| # | 测试 | 覆盖流程 |
|---|------|---------|
| 1 | `test_full_crud_lifecycle` | 内容写入→读取→覆盖→列表写入→验证→删除 |
| 2 | `test_content_resolution_recursive` | local 直接内容、group 组合聚合（`# file:` 头） |
| 3 | `test_trashcan_full_flow` | 移入回收站→恢复→永久删除→验证 hosts_content 清理 |
| 4 | `test_search_replace_full_flow` | 查找→替换→验证不再有匹配 |
| 5 | `test_remote_refresh` | HTTP 拉取→存储→验证 last_refresh_ms |
| 6 | `test_toggle_logic` | 单选模式互斥 + 多选模式允许多开 |
| 7 | `test_config_full_flow` | 默认值→保存→加载→部分更新→camelCase 键验证 |
| 8 | `test_get_enabled_content` | 仅 on=true 条目的内容出现在输出中 |
| 9 | `test_system_paths` | Windows/macOS/Linux 系统 hosts 路径正确性 |

### 5.2 审查后新增（12 个）

| # | 测试 | 覆盖 Bug 修复 |
|---|------|-------------|
| 10 | `test_config_key_naming_camelcase` | 配置键 camelCase 一致性（前端→后端匹配） |
| 11 | `test_crlf_normalization_no_doubling` | CRLF 规范化不产生 `\r\r\n` |
| 12 | `test_import_export_roundtrip` | 导出 JSON → 导入到新 DB → 验证列表+回收站 |
| 13 | `test_import_invalid_json_preserves_data` | 无效 JSON 不会破坏现有数据 |
| 14 | `test_multiline_regex_no_panic` | 多行正则不 panic（跨行匹配安全处理） |
| 15 | `test_flatten_roundtrip_preserves_ids` | 扁平操作保持 ID 一致 |
| 16 | `test_trash_collect_ids_for_group` | 组合条目删除时收集 include 引用 ID |
| 17 | `test_search_empty_content` | 空内容条目被搜索跳过 |
| 18 | `test_config_set_get_roundtrip` | 配置写入→读取一致性 |
| 19 | `test_history_trim` | 历史记录按 limit 裁剪 |
| 20 | `test_cron_refresh_logic` | Cron 时间间隔计算、过期判断、on/off/interval/url 过滤 |
| 21 | `test_add_item_parameter_mapping` | Tauri camelCase 参数转换（前端→后端 refreshInterval→refresh_interval） |

## 6. 前端手动测试清单

### 6.1 基础功能

- [ ] 应用启动，左侧树形列表渲染
- [ ] 顶栏 ≡ 折叠/展开左面板（动画 0.3s）
- [ ] 顶栏 + 新建条目对话框（本地/远程/组合）
- [ ] 右键菜单：编辑 / 复制 / 刷新远程 / 移入回收站
- [ ] 左面板空白处右键"添加 Hosts"
- [ ] 回收站展开/折叠、恢复/永久删除/清空

### 6.2 开关与系统应用

- [ ] 点击开关，后端 `toggle_hosts` 响应，树刷新
- [ ] 单选模式：开一个自动关其他
- [ ] 多选模式：允许多个同时 ON
- [ ] "应用到系统"按钮 → 保存编辑器内容 → 写入系统 hosts
- [ ] 无 ON 条目时提示"没有已启用的条目"
- [ ] `C:\Windows\System32\drivers\etc\hosts` 正确修改（追加模式保留原有内容）

### 6.3 编辑器

- [ ] 本地类型：可编辑，语法高亮（注释绿/IP 蓝/hostname 黑）
- [ ] 远程/组合/SystemHosts：只读，背景灰，不可编辑
- [ ] 透明 textarea + pre 叠加：输入同步高亮
- [ ] 鼠标滚轮：可编辑和只读模式均可滚动
- [ ] Pre 滚动同步：textarea 滚时 pre 跟随

### 6.4 查找替换

- [ ] Ctrl+F / 工具栏按钮打开查找面板
- [ ] 字面搜索返回匹配列表（显示匹配行 + 条目名）
- [ ] 忽略大小写
- [ ] 替换全部（仅本地条目生效）
- [ ] 点击结果行跳转到对应条目
- [ ] 非本地类型：替换输入框禁用

### 6.5 设置面板

- [ ] 主题切换（浅色 / 深色 / 跟随系统）
- [ ] 左面板宽度调整
- [ ] 写入模式（追加 / 覆盖）
- [ ] 选择模式（单选 / 多选）
- [ ] 历史记录上限
- [ ] 删除重复记录复选框（开启/关闭→保存→重新打开验证状态）
- [ ] 导入/导出数据
- [ ] 写入历史查看

### 6.6 深色主题

- [ ] `[data-theme="dark"]` 切换 35+ CSS 变量
- [ ] 编辑器背景/文字色跟随主题
- [ ] 语法高亮颜色自适应
- [ ] 图标通过 `currentColor` 反白
- [ ] 滚动条、输入框等原生控件跟随 `color-scheme: dark`

### 6.7 Toast 通知

- [ ] 保存/应用/导入/导出/刷新 操作有成功提示
- [ ] 失败操作有红色错误提示
- [ ] Toast 2.5s 自动消失
- [ ] 深色模式下 Toast 颜色正常

### 6.8 远程刷新

- [ ] 创建远程条目（URL + 刷新间隔）
- [ ] 右键/工具栏按钮刷新 → 拉取内容 → 自动应用到系统 → toast "刷新成功"
- [ ] 失败时 toast "刷新失败: xxx"
- [ ] 后台 cron 自动定时刷新（`cron_tick` 事件可监听诊断）
- [ ] Cron 刷新失败时前端收到 `cron_error` 事件

### 6.9 导入导出

- [ ] 导出 → 原生保存对话框 → 选择路径 → JSON 文件写入
- [ ] 导入 → 原生打开对话框 → 选择 JSON → 恢复数据
- [ ] 导入无效文件 → 错误提示

### 6.10 面板拖拽

- [ ] 左面板右侧边缘 4px 拖拽手柄
- [ ] 拖拽范围 180-500px
- [ ] 折叠/展开按钮自适应面板宽度

### 6.11 系统集成

- [ ] 托盘图标显示 + 右键菜单
- [ ] 托盘左键显隐窗口
- [ ] 配置变更事件广播
- [ ] 快捷键定义可获取

## 7. 权限提升测试（Windows）

| 场景 | 预期 |
|------|------|
| 以管理员运行 → 点"应用到系统" | 直接写入成功，无 UAC |
| 普通用户运行 → 点"应用到系统" | 弹出 UAC → 点"是" → 写入成功 |
| 普通用户运行 → UAC 点"否" | 返回错误"请以管理员身份运行" |
| PowerShell 窗口 | 不显示（`CREATE_NO_WINDOW`） |

## 8. 卸载数据清理测试（Windows NSIS）

| 场景 | 预期 |
|------|------|
| 卸载 → 不勾选"Delete user data" | `%APPDATA%\HostZ\` 保留 |
| 卸载 → 勾选"Delete user data" | `%APPDATA%\HostZ\` 被删除 |
| 卸载后重新安装 | 应用正常初始化，创建新的 `hostz.db` + `cfg.db` |

## 9. 测试命令

```bash
# 后端编译检查
cd src-tauri && cargo check

# 全部测试
cd src-tauri && cargo test

# 仅单元测试
cargo test --lib

# 仅集成测试
cargo test --test integration_test

# 单个测试
cargo test test_toggle_logic

# Release 构建
cargo tauri build
```

## 10. 测试结果汇总

| 类别 | 数量 | 通过 |
|------|------|------|
| content_parser | 9 | ✅ 9 |
| hosts_manager | 2 | ✅ 2 |
| trash | 1 | ✅ 1 |
| search | 5 | ✅ 5 |
| 集成（原有） | 9 | ✅ 9 |
| 集成（新增） | 12 | ✅ 12 |
| **合计** | **38** | **38** |

## 11. 已知限制

1. **GUI 测试**: Tauri 窗口需要桌面环境，无自动化 GUI 测试
2. **托盘测试**: 系统托盘依赖平台 API，需手动验证
3. **快捷键**: 全局快捷键注册尚未接入 Tauri plugin
4. **网络测试**: `test_remote_refresh` 依赖外部 URL，网络不可用时失败
5. **测试数据清理**: `%TEMP%\hostz_test\` 下的测试数据库文件不会自动清理
6. **i18n**: 翻译模块是占位，`t()` 返回原始 key
7. **更新检查**: Tauri updater 未接入，`check_update()` 始终返回 false
8. **卸载数据清理**: NSIS 卸载时"Delete user data"通过自定义 hook 删除 `%APPDATA%\HostZ`，需手动验证

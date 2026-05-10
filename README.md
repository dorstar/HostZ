# HostZ

> 轻量级系统 hosts 管理工具，Rust + Tauri v2 重写版 SwitchHosts

[![Rust](https://img.shields.io/badge/rust-1.95+-orange.svg)](https://www.rust-lang.org)
[![Tauri](https://img.shields.io/badge/tauri-2.11-blue.svg)](https://tauri.app)
[![License](https://img.shields.io/badge/license-MIT-green.svg)](LICENSE)

## 特性

- **极轻量**：【核心理念：小工具一定要小】。二进制 10 MB，内存 < 10 MB，启动 < 1s（SwitchHosts日常内存200M+，HostZ内存仅为其1/20）
- **三种条目类型**：本地（可编辑）、远程（HTTP 拉取）、组合（聚合多个条目）
- **追加/覆盖模式**：追加模式保留系统 hosts 原有内容
- **语法高亮**：只读模式下注释（绿）、IP（蓝）着色
- **查找替换**：支持正则，跨条目搜索
- **导入导出**：原生文件对话框，一键迁移配置
- **深色主题**：35+ CSS 变量全覆盖
- **中英双语**：设置中切换，托盘/工具栏/对话框全部跟随
- **回收站**：软删除 + 恢复 + 永久删除
- **系统托盘**：关闭窗口最小化到托盘，右键菜单快速操作
- **写入历史**：每次应用到系统自动记录，支持回看

## 安装

从 [Releases](../../releases) 下载 `HostZ_1.0.0_x64-setup.exe`（NSIS 安装包，~3 MB）。

或直接运行 `hostz.exe`（便携版，无需安装）。

**依赖**：Windows 10+（自带 WebView2），macOS / Linux 需自行安装 WebView2。

## 构建

```bash
# 要求：Rust 1.95+
cd src-tauri
cargo tauri build

# 输出：src-tauri/target/release/bundle/nsis/HostZ_1.0.0_x64-setup.exe
```

## 技术栈

| 层 | 技术 |
|---|------|
| 应用壳 | Tauri v2 |
| 前端 | 纯 HTML/CSS/JS（无框架，零依赖） |
| 图标 | Octicons 16px SVG（CSS mask-image） |
| 数据库 | SQLite（rusqlite bundled） |
| HTTP | ureq |
| 调度 | std::thread |

## 项目结构

```
HostZ/
├── dist/                  # 前端静态资源
│   ├── index.html
│   ├── style.css
│   ├── app.js
│   └── icons/             # Octicons SVG 图标
├── src-tauri/             # Rust 后端
│   ├── src/
│   │   ├── main.rs        # Tauri 入口 + 27 条命令
│   │   ├── models/        # 数据模型
│   │   ├── db/            # SQLite 数据层
│   │   ├── core/          # 核心业务逻辑
│   │   ├── services/      # 托盘/调度/通知/热键
│   │   └── utils/         # 平台/i18n
│   └── tests/             # 集成测试 (19 个)
└── docs/                  # 文档
```

## 数据存储

| 平台 | 路径 |
|------|------|
| Windows | `%APPDATA%\HostZ\data\hostz.db` |
| macOS | `~/Library/Application Support/HostZ/data/hostz.db` |
| Linux | `~/.config/HostZ/data/hostz.db` |

## License

MIT

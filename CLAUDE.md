# CLAUDE.md — 为 SwitchHosts 轻量化重写定制的项目指令

## 0. 项目目标与核心约束
- **目标**：结合 `@docs/architecture.md` 中的架构设计，用 Rust + Tauri v2 重写 SwitchHosts。
- **核心约束**：
  - 最终打包体积 **<15MB** (原 Electron 版 >100MB)。
  - 内存占用 **<50MB**。
  - 启动时间 **<1s**。
  - **功能完整性**：必须完全覆盖原 SwitchHosts 所有核心功能（hosts管理、远程同步、回收站等），不可因轻量化而删减。

## 1. 技术栈与硬性约束
- **可以用的库**：`tauri v2`, `leptos`, `rusqlite`, `ureq`, `tiny_http`, `serde`等，详见架构文档。
- **禁止引入的依赖**：`tokio` 全套, `axum`, `actix-web`, `warp`, `reqwest`, `diesel`, `sqlx`, `egui`, `dioxus`。

## 编码注意事项 (AI 必须遵循)
- 任何新增的第三方库必须事先评估其对 binary 大小和启动时间的影响，优先使用标准库替代。
- 所有字符串使用 `&str` 或 `String`，避免不必要的 `clone()`。
- 数据库操作尽量批量进行。
- 系统 hosts 写入操作需加文件锁。
- 错误处理使用 `anyhow` 或自定义错误类型，不要滥用 `unwrap()`。
- 保持代码模块化，每个文件不超过 500 行（除非必要）。
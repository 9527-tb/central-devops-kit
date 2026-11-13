# Central DevOps Kit

桌面工作台壳。技术栈和模块划分与 DevKit 对齐：Tauri 2、Vue 3、Vite、antdv-next。主区域是空的，用来接新功能。

没有带过来的部分：左侧图标菜单，以及它打开的工作台、运行计划、工具（项目扫描、构建运行、端口管理等）。

支持 **macOS · Windows · Linux**。界面有简体中文、繁體中文、English、日本語。

## 开始

```bash
pnpm install
pnpm tauri dev
```

只看前端（没有托盘和设置文件）：

```bash
pnpm dev
```

个人设置保存在：

- macOS / Linux：`~/.central-devops-kit/settings.json`
- Windows：`%APPDATA%\central-devops-kit\settings.json`

左侧图标轨底部是设置：主题、语言、开机自启、关闭到托盘。新功能的图标加在轨的上方。标题栏右侧云下载图标用来检查更新。

## 加功能

- 页面放 `src/views/`
- 特性放 `src/features/<name>/`
- 状态放 `src/stores/`
- 左侧入口加在 `src/features/app/AppRail.vue` 的 `items`（不要设 `bottom`），并在 `src/stores/appNav.js` 补跳转
- Rust 业务放 `src-tauri/src/` 的独立模块，`commands.rs` 只做 IPC 转发

## 发版

版本从 **0.0.1** 开始。以 `src-tauri/tauri.conf.json` 的 `version` 为准，并与 `package.json`、`src-tauri/Cargo.toml` 保持一致。

推送标签 `v0.0.1`（之后用 `v*`），或在 GitHub Actions 里手动运行 Release。工作流会构建 macOS（Apple 芯片和 Intel）、Windows、Linux 安装包，并发布到同名 GitHub Release。

## 应用内更新

更新插件已接上，清单和签名公钥还没配。准备开启应用内更新时，在 `src-tauri/tauri.conf.json` 增加 `plugins.updater`（`pubkey` 与 `endpoints`），并把 `bundle.createUpdaterArtifacts` 设为 `true`。

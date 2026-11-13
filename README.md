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

左侧图标轨底部是设置：主题、语言、开机自启、关闭到托盘。检查更新在左侧栏最下方。新功能的图标加在轨的上方。端口检测在 GitLab 变量旁边：输入端口号查询占用进程，确认后可以结束该进程。窗口使用系统标题栏。

## 加功能

- 页面放 `src/views/`
- 特性放 `src/features/<name>/`
- 状态放 `src/stores/`
- 左侧入口加在 `src/features/app/AppRail.vue` 的 `items`（不要设 `bottom`），并在 `src/stores/appNav.js` 补跳转
- Rust 业务放 `src-tauri/src/` 的独立模块，`commands.rs` 只做 IPC 转发

## 发版

版本从 **0.0.1** 开始。`package.json`、`src-tauri/Cargo.toml`、`src-tauri/tauri.conf.json` 保持同一版本号。

代码合并到 `master` 后，GitHub Actions 会发布一个新版本。没有历史标签时发布配置里的版本；提交里已经把版本调得更高时，按那个版本发布；否则在最新标签上把补丁号加一。然后构建 macOS（Apple 芯片和 Intel）、Windows、Linux 安装包，并发布 GitHub Release。附件文件名带有系统，例如 `Windows_x64_setup.exe`、`Linux_x64.deb`、`macOS_AppleSilicon.dmg`。

## 应用内更新

启动约 3 秒后自动检查；也可以在左侧栏或设置里手动检查。发现新版本后确认下载安装，完成后重启。

客户端按顺序读取这些清单：

- `https://ghfast.top/https://github.com/9527-tb/central-devops-kit/releases/latest/download/latest.json`
- `https://cdn.jsdelivr.net/gh/9527-tb/central-devops-kit@master/updater/latest.json`
- `https://github.com/9527-tb/central-devops-kit/releases/latest/download/latest.json`

安装包用 minisign 验签。仓库 Secret `TAURI_SIGNING_PRIVATE_KEY` 是 `~/.tauri/central-devops-kit.key` 的全文；私钥没有密码。发布成功后，工作流会按 Release 附件重建 `latest.json`，并写回 `updater/latest.json`。

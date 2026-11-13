# Central DevOps Kit

桌面工具。技术栈与 DevKit 对齐：Tauri 2、Vue 3、Vite、antdv-next。支持 **macOS · Windows · Linux**。界面有简体中文、繁體中文、English、日本語。

当前包含 GitLab 变量对比、端口检测、设置和托盘、应用内更新。DevKit 里的工作台、运行计划和项目扫描没有放进来。

## 功能

- **GitLab 变量**：填写 group 地址和 Access Token 后读取变量，并排对比 `connect-staging` 与 `connect-production`。可以看哪一侧缺变量、哪些值不同，并复制变量名或值。
- **端口检测**：按端口号查询占用进程。精确匹配 1–65535；勾选模糊匹配后，`80` 会命中 `80`、`8080`、`18080`。确认后可以结束对应进程。macOS / Linux 用 `lsof`，Windows 用 `netstat`。
- **设置**：主题、语言、开机自启、关闭到托盘、启动时检查更新。关闭窗口可进托盘。窗口使用系统标题栏。
- **应用内更新**：启动约 3 秒后自动检查，左侧栏和设置里也可以手动检查。有新版本时弹窗，确认后下载安装并重启。

左侧图标轨从上到下是 GitLab 变量、端口检测，底部是设置，最下方是检查更新。悬停显示名称。当前页保持选中，再点一次不会回到空白页。

## 开始

需要 Node.js、pnpm 和 Rust。桌面模式还要本机的 Tauri 系统依赖。

```bash
pnpm install
pnpm tauri dev
```

只看前端（没有托盘、设置文件和端口查询）：

```bash
pnpm dev
```

个人设置保存在：

- macOS / Linux：`~/.central-devops-kit/settings.json`
- Windows：`%APPDATA%\central-devops-kit\settings.json`

## 加功能

- 页面放 `src/views/`
- 特性放 `src/features/<name>/`
- 状态放 `src/stores/`
- 左侧入口加在 `src/features/app/AppRail.vue` 的 `items`（不要设 `bottom`），并在 `src/stores/appNav.js` 补 `goXxx`，再在 `src/App.vue` 挂上对应视图
- 文案加到 `src/i18n/locales/` 的四种语言
- Rust 业务放 `src-tauri/src/` 的独立模块，`commands.rs` 只做 IPC 转发

## 发版

版本从 **0.0.1** 开始。`package.json`、`src-tauri/Cargo.toml`、`src-tauri/tauri.conf.json` 保持同一版本号。

代码推到 `master` 后，GitHub Actions 用 Node.js 24 打开发布。没有历史标签时发布配置里的版本；配置版本已经高于最新标签时，按配置版本发布；否则在最新标签上把补丁号加一。提交说明里带 `[skip ci]` 或 `[ci skip]` 时不发布。

构建范围：

- macOS Apple Silicon、macOS Intel
- Windows
- Linux（deb、rpm、AppImage）

发布完成后，工作流把 Tauri 默认文件名改成带系统的名字，并写上说明。同名 `.sig` 是签名。`macOS_*.app.tar.gz` 是应用内更新包，不是日常安装包。

| 文件名 | 说明 |
| --- | --- |
| `Central-DevOps-Kit_<version>_macOS_AppleSilicon.dmg` | macOS Apple Silicon 安装包 |
| `Central-DevOps-Kit_<version>_macOS_Intel.dmg` | macOS Intel 安装包 |
| `Central-DevOps-Kit_<version>_Windows_x64_setup.exe` | Windows 安装程序 |
| `Central-DevOps-Kit_<version>_Windows_x64.msi` | Windows MSI 安装包 |
| `Central-DevOps-Kit_<version>_Linux_x64.deb` | Linux Debian/Ubuntu 安装包 |
| `Central-DevOps-Kit_<version>_Linux_x64.rpm` | Linux RPM 安装包 |
| `Central-DevOps-Kit_<version>_Linux_x64.AppImage` | Linux AppImage |

## 应用内更新

客户端按顺序读取这些清单：

- `https://ghfast.top/https://github.com/9527-tb/central-devops-kit/releases/latest/download/latest.json`
- `https://cdn.jsdelivr.net/gh/9527-tb/central-devops-kit@master/updater/latest.json`
- `https://github.com/9527-tb/central-devops-kit/releases/latest/download/latest.json`

安装包用 minisign 验签。仓库 Secret `TAURI_SIGNING_PRIVATE_KEY` 是 `~/.tauri/central-devops-kit.key` 的全文，私钥没有密码。发布成功后，工作流按 Release 附件重建 `latest.json`，并写回 `updater/latest.json`。

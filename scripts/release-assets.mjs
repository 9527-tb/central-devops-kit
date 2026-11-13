/**
 * GitHub Release 附件的可读文件名。
 * Tauri 默认名只有架构（x64 / amd64 / aarch64），这里改成带系统的名字。
 */

const TAURI_PREFIX = "Central.DevOps.Kit";
const CLEAR_PREFIX = "Central-DevOps-Kit";

/**
 * @param {string} version 不带 v 的版本号
 */
export function releaseAssetPlan(version) {
  return [
    {
      from: `${TAURI_PREFIX}_${version}_aarch64.dmg`,
      to: `${CLEAR_PREFIX}_${version}_macOS_AppleSilicon.dmg`,
      label: "macOS Apple Silicon 安装包",
    },
    {
      from: `${TAURI_PREFIX}_${version}_x64.dmg`,
      to: `${CLEAR_PREFIX}_${version}_macOS_Intel.dmg`,
      label: "macOS Intel 安装包",
    },
    {
      from: `${TAURI_PREFIX}_${version}_aarch64.app.tar.gz`,
      to: `${CLEAR_PREFIX}_${version}_macOS_AppleSilicon.app.tar.gz`,
      label: "macOS Apple Silicon 更新包",
      platforms: ["darwin-aarch64", "darwin-aarch64-app"],
    },
    {
      from: `${TAURI_PREFIX}_${version}_x64.app.tar.gz`,
      to: `${CLEAR_PREFIX}_${version}_macOS_Intel.app.tar.gz`,
      label: "macOS Intel 更新包",
      platforms: ["darwin-x86_64", "darwin-x86_64-app"],
    },
    {
      from: `${TAURI_PREFIX}_${version}_x64-setup.exe`,
      to: `${CLEAR_PREFIX}_${version}_Windows_x64_setup.exe`,
      label: "Windows 安装程序",
      platforms: ["windows-x86_64", "windows-x86_64-nsis"],
    },
    {
      from: `${TAURI_PREFIX}_${version}_x64_en-US.msi`,
      to: `${CLEAR_PREFIX}_${version}_Windows_x64.msi`,
      label: "Windows MSI 安装包",
      platforms: ["windows-x86_64-msi"],
    },
    {
      from: `${TAURI_PREFIX}_${version}_amd64.deb`,
      to: `${CLEAR_PREFIX}_${version}_Linux_x64.deb`,
      label: "Linux Debian/Ubuntu 安装包",
      platforms: ["linux-x86_64-deb"],
    },
    {
      from: `${TAURI_PREFIX}_${version}_amd64.AppImage`,
      to: `${CLEAR_PREFIX}_${version}_Linux_x64.AppImage`,
      label: "Linux AppImage",
      platforms: ["linux-x86_64", "linux-x86_64-appimage"],
    },
    {
      from: `${TAURI_PREFIX}-${version}-1.x86_64.rpm`,
      to: `${CLEAR_PREFIX}_${version}_Linux_x64.rpm`,
      label: "Linux RPM 安装包",
      platforms: ["linux-x86_64-rpm"],
    },
  ];
}

export function releaseNotes(version) {
  return `## Central DevOps Kit v${version}

安装包文件名带有系统，下载后可以直接分辨。

- macOS Apple Silicon 安装包：\`macOS_AppleSilicon.dmg\`
- macOS Intel 安装包：\`macOS_Intel.dmg\`
- macOS 更新包：\`macOS_*.app.tar.gz\`（应用内更新使用）
- Windows 安装程序：\`Windows_x64_setup.exe\`
- Windows 安装包：\`Windows_x64.msi\`
- Linux Debian/Ubuntu 安装包：\`Linux_x64.deb\`
- Linux RPM 安装包：\`Linux_x64.rpm\`
- Linux AppImage：\`Linux_x64.AppImage\`

应用内更新读取本 Release 的 \`latest.json\`。
`;
}

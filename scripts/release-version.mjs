/**
 * 计算合并到 master 后要发布的版本，并写回三个版本文件。
 * 没有历史标签时使用配置里的版本（从 0.0.1 开始）。
 * 配置版本已经高于最新标签时，按配置版本发布。
 * 否则在最新标签上把补丁号加一。
 */

import { execFileSync } from "node:child_process";
import { readFileSync, writeFileSync } from "node:fs";
import { pathToFileURL } from "node:url";

const VERSION_FILES = ["package.json", "src-tauri/tauri.conf.json", "src-tauri/Cargo.toml"];

export function parseVersion(value) {
  const match = String(value).trim().match(/^v?(\d+)\.(\d+)\.(\d+)$/);
  if (!match) return null;
  return [Number(match[1]), Number(match[2]), Number(match[3])];
}

export function compareVersions(left, right) {
  const a = parseVersion(left);
  const b = parseVersion(right);
  if (!a || !b) return 0;
  for (let i = 0; i < 3; i += 1) {
    if (a[i] !== b[i]) return a[i] > b[i] ? 1 : -1;
  }
  return 0;
}

export function bumpPatch(version) {
  const parts = parseVersion(version);
  if (!parts) throw new Error(`Invalid version: ${version}`);
  parts[2] += 1;
  return parts.join(".");
}

export function nextReleaseVersion(configVersion, tags) {
  const current = parseVersion(configVersion);
  if (!current) throw new Error(`Invalid configured version: ${configVersion}`);
  const released = tags
    .map((tag) => parseVersion(tag))
    .filter(Boolean)
    .sort((a, b) => compareVersions(a.join("."), b.join(".")));
  if (!released.length) return current.join(".");
  const latest = released[released.length - 1].join(".");
  if (compareVersions(current.join("."), latest) > 0) return current.join(".");
  return bumpPatch(latest);
}

export function readConfiguredVersion() {
  const tauri = JSON.parse(readFileSync("src-tauri/tauri.conf.json", "utf8"));
  return tauri.version;
}

export function applyVersion(version) {
  if (!parseVersion(version)) throw new Error(`Invalid version: ${version}`);
  for (const file of VERSION_FILES) {
    const original = readFileSync(file, "utf8");
    const updated = file.endsWith(".toml")
      ? original.replace(/^version = "[^"]+"/m, `version = "${version}"`)
      : original.replace(/("version"\s*:\s*")[^"]+(")/, `$1${version}$2`);
    if (updated === original) throw new Error(`Version field not found in ${file}`);
    writeFileSync(file, updated);
  }
}

function gitTags() {
  const output = execFileSync("git", ["tag", "--list", "v*"], { encoding: "utf8" });
  return output.split("\n").map((line) => line.trim()).filter(Boolean);
}

const isDirectRun = process.argv[1]
  && import.meta.url === pathToFileURL(process.argv[1]).href;

if (isDirectRun) {
  const version = nextReleaseVersion(readConfiguredVersion(), gitTags());
  applyVersion(version);
  process.stdout.write(`${version}\n`);
}

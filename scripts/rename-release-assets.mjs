#!/usr/bin/env node
/**
 * 把 Tauri 默认附件名改成带系统的文件名，并写上 Release 说明。
 *
 * 用法: node scripts/rename-release-assets.mjs v0.0.6
 */
import { execFileSync } from "node:child_process";
import { releaseAssetPlan, releaseNotes } from "./release-assets.mjs";

const tag = (process.argv[2] || "").replace(/^refs\/tags\//, "");
if (!tag) {
  console.error("Usage: node scripts/rename-release-assets.mjs <tag>");
  process.exit(1);
}

const repo =
  process.env.GITHUB_REPOSITORY ||
  execFileSync("gh", ["repo", "view", "--json", "nameWithOwner", "-q", ".nameWithOwner"], {
    encoding: "utf8",
  }).trim();

function ghJson(args, input) {
  const output = execFileSync("gh", args, {
    encoding: "utf8",
    input,
    stdio: ["pipe", "pipe", "inherit"],
  });
  return output ? JSON.parse(output) : null;
}

const release = ghJson(["api", `repos/${repo}/releases/tags/${tag}`]);
const byName = new Map((release.assets || []).map((asset) => [asset.name, asset]));
const version = tag.replace(/^v/, "");

function renameAsset(from, to, label) {
  const source = byName.get(from);
  const target = byName.get(to);
  const asset = source || target;
  if (!asset) {
    console.warn(`[rename-release-assets] skip missing ${from}`);
    return;
  }
  if (source && target && source.id !== target.id) {
    console.warn(`[rename-release-assets] both ${from} and ${to} exist, keeping ${to}`);
    return;
  }
  if (asset.name === to && asset.label === label) {
    console.log(`[rename-release-assets] ok ${to}`);
    return;
  }
  const updated = ghJson(
    ["api", "--method", "PATCH", `repos/${repo}/releases/assets/${asset.id}`, "--input", "-"],
    JSON.stringify({ name: to, label }),
  );
  byName.delete(asset.name);
  byName.set(updated.name, updated);
  console.log(`[rename-release-assets] ${from} → ${to}`);
}

for (const item of releaseAssetPlan(version)) {
  renameAsset(item.from, item.to, item.to);
  renameAsset(`${item.from}.sig`, `${item.to}.sig`, `${item.to}.sig`);
}

ghJson(
  ["api", "--method", "PATCH", `repos/${repo}/releases/${release.id}`, "--input", "-"],
  JSON.stringify({ body: releaseNotes(version) }),
);
console.log(`[rename-release-assets] updated release notes for ${tag}`);

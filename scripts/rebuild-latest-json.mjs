#!/usr/bin/env node
/**
 * 根据 Release 资产完整重建 latest.json，URL 一律用 browser_download_url。
 * 同时写入仓库 updater/latest.json，供 jsDelivr 镜像端点使用。
 *
 * 用法: node scripts/rebuild-latest-json.mjs v0.0.5
 *
 * 环境变量:
 *   CDK_UPDATE_DOWNLOAD_MIRROR=0  — 不改写下载 URL（默认经 ghfast 加速）
 */
import { writeFileSync, readFileSync, mkdirSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { execFileSync } from "node:child_process";
import { releaseAssetPlan } from "./release-assets.mjs";

const __dirname = dirname(fileURLToPath(import.meta.url));
const root = join(__dirname, "..");

const tag = (process.argv[2] || "").replace(/^refs\/tags\//, "");
if (!tag) {
  console.error("Usage: node scripts/rebuild-latest-json.mjs <tag>");
  process.exit(1);
}

const useDownloadMirror = process.env.CDK_UPDATE_DOWNLOAD_MIRROR !== "0";
const DOWNLOAD_MIRROR_PREFIX = "https://ghfast.top/";

const repo =
  process.env.GITHUB_REPOSITORY ||
  execFileSync("gh", ["repo", "view", "--json", "nameWithOwner", "-q", ".nameWithOwner"], {
    encoding: "utf8",
  }).trim();

function ghJson(args) {
  return JSON.parse(execFileSync("gh", args, { encoding: "utf8" }));
}

function mirrorUrl(url) {
  if (!useDownloadMirror) return url;
  if (url.startsWith(DOWNLOAD_MIRROR_PREFIX)) return url;
  return `${DOWNLOAD_MIRROR_PREFIX}${url}`;
}

/** @param {string} name @param {Map<string, any>} byName */
function platformEntries(name, byName) {
  const asset = byName.get(name);
  const sigAsset = byName.get(`${name}.sig`);
  if (!asset?.browser_download_url || !sigAsset) return null;

  const sigPath = join(tmpdir(), `cdk-sig-${sigAsset.id}.sig`);
  execFileSync(
    "gh",
    [
      "release",
      "download",
      tag,
      "--repo",
      repo,
      "--pattern",
      sigAsset.name,
      "--output",
      sigPath,
      "--clobber",
    ],
    { stdio: "pipe" },
  );
  const signature = readFileSync(sigPath, "utf8").trim();
  return { url: mirrorUrl(asset.browser_download_url), signature };
}

const release = ghJson(["api", `repos/${repo}/releases/tags/${tag}`]);
const byName = new Map((release.assets || []).map((asset) => [asset.name, asset]));

const version = tag.replace(/^v/, "");
/** @type {Record<string, {url:string, signature:string}>} */
const platforms = {};

const mapping = releaseAssetPlan(version).filter((item) => item.platforms);

for (const item of mapping) {
  const file = byName.has(item.to) ? item.to : item.from;
  const keys = item.platforms;
  const entry = platformEntries(file, byName);
  if (!entry) {
    console.warn(`[rebuild-latest-json] skip missing ${file}`);
    continue;
  }
  for (const key of keys) {
    platforms[key] = entry;
  }
  console.log(`[rebuild-latest-json] + ${keys.join(", ")} ← ${file}`);
}

if (!platforms["darwin-aarch64"] && !platforms["darwin-x86_64"]) {
  console.error("[rebuild-latest-json] no darwin platforms — macOS updater will fail");
  process.exit(1);
}
if (!Object.keys(platforms).length) {
  console.error("[rebuild-latest-json] no platforms found");
  process.exit(1);
}

const latest = {
  version,
  notes:
    (release.body || `## Central DevOps Kit ${tag}`).trim() +
    `\n\n<!-- bust ${Date.now()} -->\n`,
  pub_date: new Date().toISOString(),
  platforms,
};

const jsonText = `${JSON.stringify(latest, null, 2)}\n`;

const updaterDir = join(root, "updater");
mkdirSync(updaterDir, { recursive: true });
const repoPath = join(updaterDir, "latest.json");
writeFileSync(repoPath, jsonText);
console.log(`[rebuild-latest-json] wrote ${repoPath}`);

const workDir = join(tmpdir(), `cdk-rebuild-latest-${tag}-${Date.now()}`);
mkdirSync(workDir, { recursive: true });
const uploadPath = join(workDir, "latest.json");
writeFileSync(uploadPath, jsonText);

try {
  try {
    execFileSync(
      "gh",
      ["release", "delete-asset", tag, "latest.json", "--repo", repo, "--yes"],
      { stdio: "inherit" },
    );
  } catch {
    /* 第一次发布还没有这份清单 */
  }
  execFileSync("gh", ["release", "upload", tag, uploadPath, "--repo", repo], {
    stdio: "inherit",
  });
  console.log(
    `[rebuild-latest-json] ${tag}: uploaded latest.json with ${Object.keys(platforms).length} platform keys` +
      (useDownloadMirror ? " (download URLs via ghfast)" : ""),
  );
} finally {
  try {
    rmSync(workDir, { recursive: true, force: true });
  } catch {
    /* ignore */
  }
}

#!/usr/bin/env node
"use strict";

const fs = require("fs");
const path = require("path");
const os = require("os");
const https = require("https");
const crypto = require("crypto");
const { spawnSync } = require("child_process");

const REPO = "bmmaral/gittriage";
const pkg = require(path.join(__dirname, "..", "package.json"));
const VERSION = pkg.version;

function assetSuffix() {
  const p = process.platform;
  const a = process.arch;
  if (p === "darwin" && a === "arm64") return "aarch64-apple-darwin";
  if (p === "darwin" && (a === "x64" || a === "amd64")) return "x86_64-apple-darwin";
  if (p === "linux" && (a === "x64" || a === "amd64")) return "x86_64-unknown-linux-musl";
  if (p === "win32" && (a === "x64" || a === "amd64")) return "x86_64-pc-windows-msvc.exe";
  throw new Error(
    `gittriage: unsupported platform ${p}-${a}. Use cargo install or see https://github.com/${REPO}#install`,
  );
}

function download(url, notFoundHint) {
  return new Promise((resolve, reject) => {
    const req = https.get(url, (res) => {
      if (
        res.statusCode === 301 ||
        res.statusCode === 302 ||
        res.statusCode === 307 ||
        res.statusCode === 308
      ) {
        const loc = res.headers.location;
        if (!loc) {
          reject(new Error(`Redirect without Location from ${url}`));
          return;
        }
        download(new URL(loc, url).href, notFoundHint).then(resolve).catch(reject);
        return;
      }
      if (res.statusCode === 404) {
        reject(
          new Error(
            `Release asset missing: ${url}\n` +
              (notFoundHint ||
                `Publish a GitHub Release for v${VERSION} (see .github/workflows/release.yml), or install from source.`),
          ),
        );
        return;
      }
      if (res.statusCode !== 200) {
        reject(new Error(`GET ${url} -> HTTP ${res.statusCode}`));
        return;
      }
      const chunks = [];
      res.on("data", (c) => chunks.push(c));
      res.on("end", () => resolve(Buffer.concat(chunks)));
      res.on("error", reject);
    });
    req.on("error", reject);
  });
}

async function ensureBinary() {
  const suf = assetSuffix();
  const base = `https://github.com/${REPO}/releases/download/v${VERSION}`;
  const remoteName = `gittriage-v${VERSION}-${suf}`;
  const cacheRoot = path.join(os.homedir(), ".cache", "gittriage", VERSION);
  const localName = suf.endsWith(".exe") ? "gittriage.exe" : "gittriage";
  const target = path.join(cacheRoot, localName);

  if (fs.existsSync(target)) {
    return target;
  }

  fs.mkdirSync(cacheRoot, { recursive: true });
  const url = `${base}/${remoteName}`;
  const body = await download(
    url,
    `Expected file ${remoteName} on release v${VERSION}.`,
  );
  fs.writeFileSync(target, body);
  if (process.platform !== "win32") {
    fs.chmodSync(target, 0o755);
  }

  try {
    const sumBuf = await download(`${url}.sha256`, "");
    const expected = sumBuf.toString("utf8").trim().split(/\s+/)[0];
    const got = crypto.createHash("sha256").update(body).digest("hex");
    if (expected && /^[a-fA-F0-9]{64}$/.test(expected) && got !== expected) {
      fs.unlinkSync(target);
      throw new Error(`SHA256 mismatch for ${remoteName}`);
    }
  } catch (e) {
    if (e.message && e.message.includes("SHA256 mismatch")) {
      throw e;
    }
    /* Older releases may lack .sha256 sidecars; binary still usable. */
  }

  return target;
}

async function main() {
  const bin = await ensureBinary();
  const r = spawnSync(bin, process.argv.slice(2), { stdio: "inherit" });
  if (r.error) {
    throw r.error;
  }
  process.exit(r.status === null ? 1 : r.status);
}

main().catch((e) => {
  console.error(e.message || String(e));
  process.exit(1);
});

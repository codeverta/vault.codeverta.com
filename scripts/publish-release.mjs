#!/usr/bin/env node

import { existsSync, readFileSync, readdirSync } from "node:fs";
import { join, relative } from "node:path";
import { spawnSync } from "node:child_process";

const root = process.cwd();
const pkg = JSON.parse(readFileSync(join(root, "package.json"), "utf8"));
const tag = process.argv.find((arg) => arg.startsWith("--tag="))?.slice(6) ?? `v${pkg.version}`;
const repo = "codeverta/vault.codeverta.com";
const notes = join(root, `RELEASE_NOTES_${tag}.md`);

function run(command, args, options = {}) {
  const result = spawnSync(command, args, { cwd: root, stdio: "inherit", ...options });
  if (result.status !== 0) process.exit(result.status ?? 1);
}

function capture(command, args) {
  return spawnSync(command, args, { cwd: root, encoding: "utf8" });
}

if (!/^v\d+\.\d+\.\d+$/.test(tag)) {
  console.error(`Invalid release tag: ${tag}. Use --tag=v1.2.3.`);
  process.exit(1);
}

if (`v${pkg.version}` !== tag) {
  console.error(`package.json is ${pkg.version}, but release tag is ${tag}. Update version files first.`);
  process.exit(1);
}

const status = capture("git", ["status", "--porcelain"]).stdout.trim();
if (status) {
  console.error("Working tree is not clean. Commit changes before publishing:");
  console.error(status);
  process.exit(1);
}

const auth = capture("gh", ["auth", "status"]);
if (auth.status !== 0) {
  console.error("GitHub CLI is not authenticated. Run: gh auth login");
  process.exit(1);
}

console.log(`Building ${tag} from the current workspace…`);
run("npm", ["run", "tauri", "build"]);

const bundleRoot = join(root, "src-tauri", "target", "release", "bundle");
const allowed = /\.(dmg|app\.tar\.gz|AppImage|deb|msi|exe|sig|json)$/i;
const artifacts = [];
function collect(dir) {
  if (!existsSync(dir)) return;
  for (const entry of readdirSync(dir, { withFileTypes: true })) {
    const path = join(dir, entry.name);
    if (entry.isDirectory()) collect(path);
    else if (allowed.test(entry.name) && !entry.name.endsWith(".json")) artifacts.push(path);
  }
}
collect(bundleRoot);
if (!artifacts.length) {
  console.error("No release artifacts found in src-tauri/target/release/bundle.");
  process.exit(1);
}

const releaseExists = capture("gh", ["release", "view", tag, "--repo", repo]).status === 0;
const files = artifacts.map((path) => relative(root, path));
if (releaseExists) {
  run("gh", ["release", "upload", tag, ...files, "--repo", repo, "--clobber"]);
} else {
  if (!existsSync(notes)) {
    console.error(`Missing release notes: ${notes}`);
    process.exit(1);
  }
  run("gh", ["release", "create", tag, ...files, "--repo", repo, "--title", `Vault ${tag}`, "--notes-file", notes]);
}
console.log(`Published ${tag} with ${artifacts.length} local artifact(s).`);

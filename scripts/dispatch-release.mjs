#!/usr/bin/env node

import { readFileSync } from "node:fs";
import { spawnSync } from "node:child_process";

const pkg = JSON.parse(readFileSync("package.json", "utf8"));
const tag = process.argv.find((arg) => arg.startsWith("--tag="))?.slice(6) ?? `v${pkg.version}`;
const repo = "codeverta/vault.codeverta.com";
const result = spawnSync("gh", ["workflow", "run", "release.yml", "--repo", repo, "--ref", "main", "-f", `release_tag=${tag}`], { stdio: "inherit" });
process.exit(result.status ?? 1);

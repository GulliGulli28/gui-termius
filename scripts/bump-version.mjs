#!/usr/bin/env node
// Bumps the app version everywhere it needs to be in sync for a release:
// package.json + package-lock.json (via `npm version`) and the workspace
// Cargo.toml. src-tauri/tauri.conf.json intentionally has no "version" field
// of its own — Tauri reads it from src-tauri/Cargo.toml, which inherits the
// workspace version, so there's nothing to update there.
import { execFileSync } from "node:child_process";
import { readFileSync, writeFileSync } from "node:fs";

const version = process.argv[2];
if (!version || !/^\d+\.\d+\.\d+$/.test(version)) {
  console.error("Usage: npm run bump-version -- <x.y.z>");
  process.exit(1);
}

execFileSync("npm", ["version", version, "--no-git-tag-version", "--allow-same-version"], { stdio: "inherit" });

const cargoTomlPath = "Cargo.toml";
const cargoToml = readFileSync(cargoTomlPath, "utf8");
const re = /(\[workspace\.package\][^[]*?version\s*=\s*)"[^"]+"/;
if (!re.test(cargoToml)) {
  console.error(`Could not find a [workspace.package] version field in ${cargoTomlPath}`);
  process.exit(1);
}
writeFileSync(cargoTomlPath, cargoToml.replace(re, `$1"${version}"`));

execFileSync("cargo", ["check", "--quiet", "--manifest-path", "src-tauri/Cargo.toml"], { stdio: "inherit" });

console.log(`\nVersion bumped to ${version}. Next steps:`);
console.log(`  git add -A && git commit -m "Bump version to ${version}"`);
console.log(`  git push`);
console.log(`  git tag v${version} && git push origin v${version}`);

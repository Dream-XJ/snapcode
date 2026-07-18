#!/usr/bin/env node
// 版本号同步脚本：一次性更新 package.json / tauri.conf.json / Cargo.toml / Cargo.lock 四处版本号。
// 用法：node scripts/bump-version.mjs <x.y.z>   例：node scripts/bump-version.mjs 0.2.0
// 之后按提示 commit + 打 v 前缀 tag 推送，即可触发 .github/workflows/release.yml 自动构建。
import { readFileSync, writeFileSync } from 'node:fs';
import { execFileSync } from 'node:child_process';

const version = process.argv[2];
if (!version || !/^\d+\.\d+\.\d+$/.test(version)) {
  console.error('用法: node scripts/bump-version.mjs <x.y.z>   例: node scripts/bump-version.mjs 0.2.0');
  process.exit(1);
}

// package.json 与 tauri.conf.json：JSON 往返保持原有键序，统一 2 空格缩进 + 末尾换行
for (const file of ['package.json', 'src-tauri/tauri.conf.json']) {
  const json = JSON.parse(readFileSync(file, 'utf8'));
  json.version = version;
  writeFileSync(file, JSON.stringify(json, null, 2) + '\n');
}

// Cargo.toml：只替换行首的 version（即 [package] 段那一行），不碰依赖版本；不锚定行尾以兼容 CRLF
const tomlPath = 'src-tauri/Cargo.toml';
const toml = readFileSync(tomlPath, 'utf8');
const versionLine = /^version = "[^"]*"/m;
if (!versionLine.test(toml)) {
  console.error(`错误: 未能在 ${tomlPath} 中找到 [package] 的 version 行`);
  process.exit(1);
}
writeFileSync(tomlPath, toml.replace(versionLine, `version = "${version}"`));

// Cargo.lock：cargo metadata 会重新解析并同步根包版本，不编译代码
try {
  execFileSync('cargo', ['metadata', '--format-version', '1', '--manifest-path', 'src-tauri/Cargo.toml'], { stdio: 'ignore' });
} catch {
  console.warn('警告: 同步 Cargo.lock 失败（cargo 不可用？），请手动执行任意 cargo 命令刷新');
}

console.log(`版本已更新为 ${version}: package.json / tauri.conf.json / Cargo.toml / Cargo.lock`);
console.log('\n接下来:');
console.log(`  git add -A && git commit -m "chore: bump version to ${version}"`);
console.log(`  git tag v${version}`);
console.log(`  git push && git push origin v${version}   # 推送 tag 触发 Release 构建`);

import { readFileSync } from "node:fs";
const pkg = JSON.parse(readFileSync("package.json","utf8")).version;
const conf = JSON.parse(readFileSync("src-tauri/tauri.conf.json","utf8")).version;
const cargo = readFileSync("src-tauri/Cargo.toml","utf8").match(/^version\s*=\s*"([^"]+)"/m)?.[1];
let props = null;
try { props = readFileSync("src-tauri/tauri.properties","utf8").match(/tauri\.version=(.+)/)?.[1]?.trim(); } catch {}
const all = { package: pkg, tauriConf: conf, cargo, ...(props ? { properties: props } : {}) };
const uniq = [...new Set(Object.values(all).filter(Boolean))];
console.log(all);
if (uniq.length !== 1) { console.error(`::error::version mismatch: ${JSON.stringify(all)}`); process.exit(1); }
console.log(`version consistent: ${uniq[0]}`);

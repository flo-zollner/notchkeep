import { readFileSync, readdirSync } from "node:fs";
const fail = (m) => { console.error(`::error::${m}`); process.exitCode = 1; };
const conf = JSON.parse(readFileSync("src-tauri/tauri.conf.json","utf8"));
const csp = conf.app?.security?.csp ?? conf.app?.windows?.[0]?.csp ?? conf.tauri?.security?.csp ?? "";
if (!csp || /default-src\s+\*/.test(csp)) fail("CSP missing or wildcard default-src");
if (!/script-src\s+'self'/.test(csp)) fail("CSP script-src must be self");
if (!/object-src\s+'none'/.test(csp)) fail("CSP object-src must be none");
for (const f of readdirSync("src-tauri/capabilities")) {
  const cap = JSON.parse(readFileSync(`src-tauri/capabilities/${f}`,"utf8"));
  const perms = JSON.stringify(cap.permissions ?? []);
  if (/:allow-(execute|all)\b|shell:.*spawn|fs:allow-(write|all)-/.test(perms)) fail(`capability ${f} grants broad/dangerous permission — review`);
}
if (process.exitCode) console.error("capability audit FAILED"); else console.log("capability audit clean");

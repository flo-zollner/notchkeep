#!/usr/bin/env node
// Local verification pipeline. Runs every gate that the CI pipeline runs (the
// ones executable without a CI runner) and prints one consolidated report, so
// failures surface here — before a push — instead of one-by-one in Actions.
//
// Usage:
//   pnpm verify             # all local gates
//   pnpm verify --quick     # skip slow gates (e2e)
//   pnpm verify --no-network# skip gates that need network (update-endpoint cert)
//   pnpm verify --only=rust,security   # run only the named groups
//
// Exit code is non-zero if any non-optional gate fails.

import { spawnSync } from 'node:child_process';

const argv = process.argv.slice(2);
const has = (f) => argv.includes(f);
const quick = has('--quick');
const noNet = has('--no-network');
const onlyArg = argv.find((a) => a.startsWith('--only='));
const onlyGroups = onlyArg ? onlyArg.slice('--only='.length).split(',').map((s) => s.trim()) : null;

// Each stage mirrors a CI step. group = report section; slow/network = skip flags.
const STAGES = [
  { name: 'versions',        group: 'meta',     cmd: ['node', 'scripts/check-versions.mjs'] },
  { name: 'lint',            group: 'frontend', cmd: ['pnpm', 'lint'] },
  { name: 'typecheck',       group: 'frontend', cmd: ['pnpm', 'check'] },
  { name: 'unit',            group: 'frontend', cmd: ['pnpm', 'test'] },
  { name: 'e2e',             group: 'frontend', cmd: ['pnpm', 'test:e2e'], slow: true },
  { name: 'rust-test',       group: 'rust',     cmd: ['cargo', 'test', '--manifest-path', 'src-tauri/Cargo.toml'] },
  { name: 'clippy',          group: 'rust',     cmd: ['cargo', 'clippy', '--manifest-path', 'src-tauri/Cargo.toml', '--all-targets', '--', '-D', 'warnings'] },
  { name: 'licenses',        group: 'rust',     cmd: ['pnpm', 'gen:licenses:check'] },
  { name: 'deny',            group: 'security', cmd: ['cargo', 'deny', '--manifest-path', 'src-tauri/Cargo.toml', 'check'] },
  { name: 'pii',             group: 'security', cmd: ['bash', 'scripts/scan-pii.sh'] },
  { name: 'logs-pii',        group: 'security', cmd: ['node', 'scripts/scan-logs-pii.mjs'] },
  { name: 'capabilities',    group: 'security', cmd: ['node', 'scripts/audit-capabilities.mjs'] },
  { name: 'secrets',         group: 'security', cmd: ['bash', 'scripts/check-secrets-committed.sh'] },
  { name: 'update-endpoint', group: 'security', cmd: ['node', 'scripts/check-update-endpoint.mjs'], network: true },
];

const skipReason = (s) => {
  if (onlyGroups && !onlyGroups.includes(s.group)) return `not in --only=${onlyGroups.join(',')}`;
  if (quick && s.slow) return '--quick';
  if (noNet && s.network) return '--no-network';
  return null;
};

const planned = STAGES.map((s) => ({ stage: s, skip: skipReason(s) }));
const toRun = planned.filter((p) => !p.skip).map((p) => p.stage);
const skipped = planned.filter((p) => p.skip);

const fmtDur = (ms) => (ms >= 1000 ? `${(ms / 1000).toFixed(1)}s` : `${Math.round(ms)}ms`);
const pad = (s, n) => String(s).padEnd(n);

console.log(`\nVerify pipeline — ${toRun.length} stage(s)${skipped.length ? `, ${skipped.length} skipped` : ''}\n`);

const results = [];
for (const stage of toRun) {
  console.log(`\x1b[1m▶ ${stage.name}\x1b[0m  (${stage.cmd.join(' ')})`);
  const t0 = process.hrtime.bigint();
  const r = spawnSync(stage.cmd[0], stage.cmd.slice(1), { stdio: 'inherit' });
  const ms = Number(process.hrtime.bigint() - t0) / 1e6;
  // spawnSync sets .error (e.g. ENOENT) and null status if the binary is missing.
  const code = r.status ?? (r.error ? 127 : 1);
  results.push({ stage, ok: code === 0, ms, code });
  console.log(code === 0 ? `\x1b[32m✓ ${stage.name} (${fmtDur(ms)})\x1b[0m\n` : `\x1b[31m✗ ${stage.name} (exit ${code})\x1b[0m\n`);
}

// ---- Consolidated report ----
const line = '─'.repeat(50);
console.log(`\n${line}\nVerify Report\n${line}`);
console.log(`${pad('Stage', 18)}${pad('Group', 11)}${pad('Result', 9)}Time`);
console.log(line);
let lastGroup = null;
for (const r of results) {
  const g = r.stage.group;
  console.log(`${pad(r.stage.name, 18)}${pad(g === lastGroup ? '' : g, 11)}${pad(r.ok ? '✓ pass' : '✗ FAIL', 9)}${fmtDur(r.ms)}`);
  lastGroup = g;
}
for (const p of skipped) {
  console.log(`${pad(p.stage.name, 18)}${pad(p.stage.group, 11)}${pad('– skip', 9)}${p.skip}`);
}
console.log(line);

const failed = results.filter((r) => !r.ok);
const passed = results.length - failed.length;
console.log(`${results.length} ran · ${passed} passed · ${failed.length} failed · ${skipped.length} skipped`);

// GitHub Step Summary, when running inside Actions.
if (process.env.GITHUB_STEP_SUMMARY) {
  const rows = results
    .map((r) => `| ${r.stage.name} | ${r.stage.group} | ${r.ok ? '✅ pass' : '❌ fail'} | ${fmtDur(r.ms)} |`)
    .concat(skipped.map((p) => `| ${p.stage.name} | ${p.stage.group} | ⏭️ skip | ${p.skip} |`))
    .join('\n');
  const md = `## Verify Report\n\n| Stage | Group | Result | Time |\n|---|---|---|---|\n${rows}\n\n**${passed}/${results.length} passed**${failed.length ? ` · ❌ ${failed.map((f) => f.stage.name).join(', ')}` : ''}\n`;
  try {
    const { appendFileSync } = await import('node:fs');
    appendFileSync(process.env.GITHUB_STEP_SUMMARY, md);
  } catch { /* summary is best-effort */ }
}

if (failed.length) {
  console.log(`\n\x1b[31mFAILED:\x1b[0m ${failed.map((f) => `${f.stage.name} (exit ${f.code})`).join(', ')}`);
  process.exit(1);
}
console.log('\n\x1b[32mAll gates green ✓\x1b[0m');

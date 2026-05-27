#!/usr/bin/env node
// Appends the licenses of all JS production dependencies to static/licenses.html.
// Run via `pnpm licenses` after `cargo about generate`.

import { execSync } from 'node:child_process';
import { readFileSync, writeFileSync, existsSync } from 'node:fs';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const target = resolve(root, 'static/licenses.html');

if (!existsSync(target)) {
  console.error(`error: ${target} not found — run "cargo about generate" first.`);
  process.exit(1);
}

const raw = execSync(
  'npx --yes license-checker --production --json --excludePackages "budget-app@0.1.0"',
  { cwd: root, encoding: 'utf8', maxBuffer: 16 * 1024 * 1024 }
);
const data = JSON.parse(raw);

const escapeHtml = (s) =>
  String(s).replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;').replace(/"/g, '&quot;');

const entries = Object.entries(data).map(([key, info]) => {
  const match = key.match(/^(.+)@([^@]+)$/);
  const name = match ? match[1] : key;
  const version = match ? match[2] : '';
  const licenses = Array.isArray(info.licenses) ? info.licenses.join(' OR ') : info.licenses || 'UNKNOWN';
  let text = '';
  if (info.licenseFile && existsSync(info.licenseFile)) {
    try { text = readFileSync(info.licenseFile, 'utf8'); } catch {}
  }
  return { name, version, licenses, repo: info.repository || info.url || '', text };
});

const groups = new Map();
for (const e of entries) {
  if (!groups.has(e.licenses)) groups.set(e.licenses, []);
  groups.get(e.licenses).push(e);
}

const sortedGroups = [...groups.entries()].sort((a, b) => b[1].length - a[1].length);

const sectionHtml = `
  <h2 style="margin-top:48px;border-top:2px solid currentColor;padding-top:18px;border-bottom:none">JavaScript / TypeScript Production-Dependencies</h2>
  <div class="summary-card">
    <ul>
      ${sortedGroups.map(([lic, list]) =>
        `<li><strong>${escapeHtml(lic)}</strong> <span class="pill">${list.length}</span></li>`
      ).join('\n      ')}
    </ul>
  </div>
  ${sortedGroups.map(([lic, list]) => `
    <h2>${escapeHtml(lic)} <span class="pill">${list.length}</span></h2>
    ${list.map((e) => `
      <details>
        <summary>
          ${escapeHtml(e.name)}
          <span class="meta">
            v${escapeHtml(e.version)}${e.repo ? ` · <a href="${escapeHtml(e.repo)}">${escapeHtml(e.repo)}</a>` : ''}
          </span>
        </summary>
        ${e.text
          ? `<pre>${escapeHtml(e.text)}</pre>`
          : `<p class="meta">Lizenztext nicht in Paket eingebettet — Bezug über das verlinkte Repository.</p>`}
      </details>
    `).join('')}
  `).join('')}
`;

// Assumes cargo-about has freshly written static/licenses.html — we just inject before </body>.
const html = readFileSync(target, 'utf8');
writeFileSync(target, html.replace('</body>', sectionHtml + '\n</body>'));

console.log(`Appended ${entries.length} JS license entries (${sortedGroups.length} unique) to ${target}`);

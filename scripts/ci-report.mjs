import { readFileSync, existsSync, appendFileSync } from 'node:fs';
const out = process.env.GITHUB_STEP_SUMMARY ?? '/dev/stdout';
const files = [['OSV', 'osv.sarif'], ['Semgrep', 'semgrep.sarif']];
let md = '## Security scan summary\n\n| Tool | Findings | Errors |\n|---|---|---|\n';
let errorTotal = 0;
for (const [name, f] of files) {
  if (!existsSync(f)) { md += `| ${name} | – (skipped) | – |\n`; continue; }
  const runs = JSON.parse(readFileSync(f, 'utf8')).runs ?? [];
  const res = runs.flatMap((r) => r.results ?? []);
  const errs = res.filter((r) => r.level === 'error').length;
  errorTotal += errs;
  md += `| ${name} | ${res.length} | ${errs} |\n`;
}
md += '\nDetail: **Security → Code scanning**.\n';
appendFileSync(out, md);
if (errorTotal > 0) { console.error(`${errorTotal} error-level findings`); process.exit(1); }

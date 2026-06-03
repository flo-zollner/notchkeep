import { readFileSync, readdirSync, statSync } from "node:fs";
import { join } from "node:path";
const dir = process.argv[2] ?? ".";
const IBAN = /\bDE[0-9]{2} ?([0-9]{4} ?){4}[0-9]{2}\b/;
const DUMMY = /DE00[ 0]+00|DE12 ?3456 ?7890/;
let hit = false;
const walk = (d) => { let ents; try { ents = readdirSync(d); } catch { return; } ents.forEach((n)=>{
  const p = join(d,n);
  let st; try { st = statSync(p); } catch { return; }
  if (st.isDirectory()) return walk(p);
  if (!/\.(log|txt|out)$/.test(n)) return;
  const t = readFileSync(p,"utf8");
  if (IBAN.test(t) && !DUMMY.test(t)) { console.error(`::error::PII (IBAN) in log ${p}`); hit = true; }
}); };
walk(dir);
if (hit) process.exit(1); else console.log("logs clean of PII");

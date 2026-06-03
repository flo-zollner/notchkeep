import tls from "node:tls";
import { readFileSync } from "node:fs";
const conf = JSON.parse(readFileSync("src-tauri/tauri.conf.json","utf8"));
const eps = conf.plugins?.updater?.endpoints ?? conf.tauri?.updater?.endpoints ?? [];
const hosts = [...new Set(eps.map((e)=>{try{return new URL(e.replace(/\{\{.*?\}\}/g,"x")).host;}catch{return null;}}).filter(Boolean))];
if (!hosts.length) { console.log("no updater endpoints to check"); process.exit(0); }
let bad = 0;
await Promise.all(hosts.map((host)=>new Promise((res)=>{
  const h = host.split(":")[0];
  const s = tls.connect({ host:h, port:443, servername:h }, ()=>{
    const c = s.getPeerCertificate();
    const days = Math.round((new Date(c.valid_to)-Date.now())/864e5);
    if (!s.authorized) { console.error(`::error::${host}: TLS not authorized (${s.authorizationError})`); bad=1; }
    else if (days<0) { console.error(`::error::${host}: certificate EXPIRED`); bad=1; }
    else if (days<30) console.warn(`::warning::${host}: certificate expires in ${days} days`);
    else console.log(`${host}: cert valid, ${days} days left`);
    s.end(); res();
  });
  s.on("error",(e)=>{ console.error(`::error::${host}: ${e.message}`); bad=1; res(); });
})));
process.exit(bad);

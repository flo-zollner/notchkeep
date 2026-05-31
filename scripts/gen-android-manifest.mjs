#!/usr/bin/env node
// Args: <apkPath> <version> <versionCode> <downloadUrl> <signatureFile> <outPath>
import { createHash } from 'node:crypto';
import { readFileSync, writeFileSync } from 'node:fs';

const [apk, version, versionCode, url, signatureFile, out] = process.argv.slice(2);
if (!apk || !version || !url || !signatureFile || !out) {
  console.error('usage: gen-android-manifest <apk> <version> <versionCode> <url> <signatureFile> <out>');
  process.exit(1);
}
const sha256 = createHash('sha256').update(readFileSync(apk)).digest('hex');
const manifest = {
  version,
  versionCode: Number(versionCode) || 0,
  notes: '',
  pub_date: process.env.PUB_DATE || '',
  url,
  sha256,
  signature: readFileSync(signatureFile, 'utf8').trim(),
};
writeFileSync(out, JSON.stringify(manifest, null, 2));
console.log('wrote', out, 'sha256', sha256);

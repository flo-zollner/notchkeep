#!/usr/bin/env bash
set -euo pipefail
bad=0
if git ls-files | grep -nE "\.(jks|keystore|p12|pfx|pem|key)$|tauri.*\.key$|minisign.*\.key$"; then
  echo "::error::A private key/keystore is tracked in git"; bad=1; fi
# Match the actual PEM armor / minisign secret-key header, not the loose phrase.
# Exclude this scanner and the gitleaks config, which contain the patterns as text.
if git ls-files | grep -vE 'scripts/check-secrets-committed\.sh|\.gitleaks\.toml' \
     | xargs grep -lE -- '-----BEGIN [A-Z0-9 ]*PRIVATE KEY-----|untrusted comment: minisign encrypted secret key' 2>/dev/null; then
  echo "::error::A private key blob is tracked in git"; bad=1; fi
grep -qE "\"pubkey\"\s*:\s*\"[A-Za-z0-9+/=]{40,}\"" src-tauri/tauri.conf.json || { echo "::error::updater pubkey missing/short"; bad=1; }
[ "$bad" = 0 ] && echo "secret-commit check clean" || exit 1

#!/usr/bin/env bash
set -euo pipefail
mode="${1:-staged}"
get() {
  if [ "$mode" = "staged" ]; then git diff --cached;
  elif [ "$mode" = "tree" ]; then git diff origin/main...HEAD 2>/dev/null || git diff HEAD;
  else grep -rhI "" "${@:-.}" 2>/dev/null; fi
}
hits=0
if get | grep -nE "\bDE[0-9]{2}[ ]?([0-9]{4}[ ]?){4}[0-9]{2}\b" | grep -vE "DE00[ 0]+00|DE12[ ]?3456[ ]?7890"; then
  echo "::error::Possible real IBAN found"; hits=1; fi
if git diff --cached -- "src-tauri/tests/fixtures/**" 2>/dev/null | grep -nE "\b[0-9]{8,12}\b" | grep -vE "99999999|00000000"; then
  echo "::error::Possible account number in fixtures"; hits=1; fi
[ "$hits" = 0 ] && echo "PII scan clean" || { echo "PII scan FAILED"; exit 1; }

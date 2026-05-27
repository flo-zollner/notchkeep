# Drittanbieter-Lizenzen

Diese Datei listet die direkten Abhängigkeiten von `Notchkeep` und ihre
Lizenzen auf. Alle Lizenzen sind kompatibel mit GPL-3.0-or-later.

Eine vollständige, automatisch generierte Liste inklusive transitiver
Abhängigkeiten kannst du erzeugen mit:

```bash
# Rust
cargo install cargo-license
cargo license --manifest-path src-tauri/Cargo.toml

# JavaScript
npx license-checker --production --summary
```

---

## Rust (`src-tauri/Cargo.toml`)

| Crate | Lizenz |
| --- | --- |
| `tauri`, `tauri-build`, `tauri-plugin-opener`, `tauri-plugin-dialog` | Apache-2.0 OR MIT |
| `serde`, `serde_json` | MIT OR Apache-2.0 |
| `tokio` | MIT |
| `sqlx` | MIT OR Apache-2.0 |
| `chrono` | MIT OR Apache-2.0 |
| `thiserror` | MIT OR Apache-2.0 |
| `uuid` | Apache-2.0 OR MIT |
| `gethostname` | Apache-2.0 |
| `csv` | Unlicense OR MIT |
| `sha2` | MIT OR Apache-2.0 |
| `regex` | MIT OR Apache-2.0 |
| `strsim` | MIT |
| `reqwest` | MIT OR Apache-2.0 |
| `tempfile` (dev) | MIT OR Apache-2.0 |

SQLite (über `sqlx`): **Public Domain**.

## JavaScript / TypeScript (`package.json`)

| Paket | Lizenz |
| --- | --- |
| `@tauri-apps/api`, `@tauri-apps/plugin-*`, `@tauri-apps/cli` | Apache-2.0 OR MIT |
| `svelte`, `@sveltejs/kit`, `@sveltejs/adapter-static`, `@sveltejs/vite-plugin-svelte` | MIT |
| `vite` | MIT |
| `typescript` | Apache-2.0 |
| `svelte-check` | MIT |
| `@types/node` | MIT |

## Hinweise zur Apache-2.0-Klausel

Apache-2.0-lizenzierte Komponenten verlangen, dass eine vorhandene `NOTICE`-Datei
mit weitergegeben wird, sofern sie vom Originalprojekt mitgeliefert wird.
Beim Bauen von Distributions-Artefakten (z. B. via `pnpm tauri build`) sollten
die Lizenz-Texte aller verlinkten Abhängigkeiten gebündelt werden — etwa über
`cargo-about` (`cargo install cargo-about`) für die Rust-Seite und
`license-checker` für die JS-Seite.

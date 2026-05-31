# Contributing to Notchkeep

[English](#english) · [Deutsch](#deutsch)

---

## English

Thanks for taking the time to contribute! Notchkeep is a local-first personal
finance and net-worth tracker built with **Tauri 2 (Rust)** and **SvelteKit
(Svelte 5 + TypeScript)**. Contributions of all kinds are welcome — bug reports,
features, bank/broker importers, translations and documentation.

By participating you agree to our [Code of Conduct](CODE_OF_CONDUCT.md).

### Ways to contribute

- 🐞 **Report a bug** or 💡 **request a feature** via the
  [issue tracker](https://github.com/flo-zollner/notchkeep/issues).
- 🏦 **Add a bank/broker importer** (see below).
- 🌍 **Improve translations** in `src/lib/i18n` (German is the default locale).
- 📖 **Improve the docs.**
- 🔒 **Report a security issue** — please follow the [Security Policy](SECURITY.md),
  do **not** open a public issue.

### Getting started

Prerequisites (versions in the [README](README.md#prerequisites)): Node.js ≥ 22,
pnpm ≥ 11, the Rust stable toolchain, and the Tauri 2 platform dependencies.

```bash
git clone https://github.com/flo-zollner/notchkeep.git
cd notchkeep
pnpm install
pnpm tauri dev          # desktop dev build + WebView
pnpm dev:mock           # browser-only dev against the mock IPC layer
```

> Never run `npm install` — this is a pnpm workspace.

### Project conventions

- **Language:** code, identifiers, comments and commit *content* are in English.
  User-facing strings are localised through `src/lib/i18n` (German default).
- **Money:** always integer **cents** (`i64` in Rust, `number` in TS); convert to
  decimals only at the UI boundary.
- **IDs:** UUID v4 stored as TEXT. **Migrations** are additive and live in
  `src-tauri/migrations/`.
- See [`CLAUDE.md`](CLAUDE.md) for the full architecture and workflow notes.

### 🔐 Privacy — never commit real financial data

This is a finance app, so this rule is strict and non-negotiable:

- **No real IBANs, BICs, account numbers, real names, tax IDs, or real
  amounts+dates** anywhere in the repo — not in code, tests, fixtures, commit
  messages, issues or PRs.
- `/test-material/` is gitignored and holds real statements. **Never** commit it
  or copy it into `src-tauri/tests/fixtures/`.
- Test fixtures must use the documented dummy substitutions (e.g.
  `DE00 0000 0000 0000 0000 00`, `Max Mustermann`, `BANKDEFFXXX`) — see the
  fixture rules in [`CLAUDE.md`](CLAUDE.md).
- Redact PII (counterparty, IBAN suffix, purpose) before logging.

When in doubt, use synthetic data and ask in the issue/PR.

### Tests &amp; quality gates

We practice **test-driven development**: write a failing test first, then the
minimal implementation. Run these before opening a PR — all must be green:

```bash
cargo test --manifest-path src-tauri/Cargo.toml   # Rust unit + integration
pnpm check                                         # svelte-check + tsc (0 errors)
pnpm test                                          # frontend unit tests (vitest)
pnpm gen:licenses:check                            # only if you changed dependencies
```

Integration tests run against a real SQLite database (via `tempfile`), not mocks.

### Dependencies &amp; licensing

Notchkeep is **GPL-3.0-or-later**. Every new dependency (direct or transitive)
must be license-compatible. The accepted licenses are whitelisted in
`src-tauri/about.toml`. **Incompatible** (do not add): AGPL-3.0, BSL-1.1, SSPL,
Elastic-License-2.0, CC-BY-NC*, or any proprietary "all rights reserved" license.

After `cargo add` / `pnpm add`, run `pnpm gen:licenses:check` — it must stay green.

### Adding a bank/broker importer

Implement the `Importer` trait in `src-tauri/src/importers/<bank>.rs`. The caller
auto-routes cash vs. trade rows independently of the UI account picker. Each
importer needs the five mandatory tests (happy path, idempotent re-import, tax
classification, trade routing, warnings). The full playbook is in
[`CLAUDE.md`](CLAUDE.md).

### Pull requests

1. Branch from `main` (or fork), keep changes focused.
2. Make sure all quality gates above are green and there is no PII in the diff.
3. Open a PR against `main` and fill in the
   [PR template](.github/PULL_REQUEST_TEMPLATE.md).
4. Conventional-commit style (`feat:`, `fix:`, `docs:`, …) is appreciated.
   Commit messages and PR descriptions may be in **English or German**.

New to open source? The [Open Source Guides](https://opensource.guide/) are a
great place to start.

---

## Deutsch

Danke, dass du beitragen möchtest! Notchkeep ist ein local-first
Haushalts- und Vermögens-Tracker auf Basis von **Tauri 2 (Rust)** und
**SvelteKit (Svelte 5 + TypeScript)**. Beiträge aller Art sind willkommen —
Fehlerberichte, Features, Bank-/Broker-Importer, Übersetzungen und Doku.

Mit deiner Teilnahme akzeptierst du unseren
[Verhaltenskodex](CODE_OF_CONDUCT.md).

### Möglichkeiten beizutragen

- 🐞 **Fehler melden** oder 💡 **Feature vorschlagen** über den
  [Issue-Tracker](https://github.com/flo-zollner/notchkeep/issues).
- 🏦 **Bank-/Broker-Importer ergänzen** (siehe unten).
- 🌍 **Übersetzungen verbessern** in `src/lib/i18n` (Deutsch ist Default).
- 📖 **Doku verbessern.**
- 🔒 **Sicherheitslücke melden** — bitte über die
  [Sicherheitsrichtlinie](SECURITY.md), **kein** öffentliches Issue.

### Erste Schritte

Voraussetzungen (Versionen im [README](README.md#prerequisites)): Node.js ≥ 22,
pnpm ≥ 11, das stabile Rust-Toolchain sowie die Tauri-2-Plattform-Abhängigkeiten.

```bash
git clone https://github.com/flo-zollner/notchkeep.git
cd notchkeep
pnpm install
pnpm tauri dev          # Desktop-Dev-Build + WebView
pnpm dev:mock           # Browser-only-Dev gegen die Mock-IPC-Schicht
```

> Niemals `npm install` ausführen — das ist ein pnpm-Workspace.

### Projekt-Konventionen

- **Sprache:** Code, Identifier, Kommentare und Commit-*Inhalt* auf Englisch.
  UI-Texte werden über `src/lib/i18n` lokalisiert (Deutsch als Default).
- **Geldbeträge:** immer Integer-**Cent** (`i64` in Rust, `number` in TS);
  Umrechnung erst an der UI-Grenze.
- **IDs:** UUID v4 als TEXT. **Migrations** sind additiv und liegen in
  `src-tauri/migrations/`.
- Architektur und Workflow im Detail: [`CLAUDE.md`](CLAUDE.md).

### 🔐 Privatsphäre — niemals echte Finanzdaten committen

Dies ist eine Finanz-App, daher gilt diese Regel strikt und ohne Ausnahme:

- **Keine echten IBANs, BICs, Kontonummern, Klarnamen, Steuer-IDs oder realen
  Beträge+Daten** irgendwo im Repo — nicht in Code, Tests, Fixtures,
  Commit-Messages, Issues oder PRs.
- `/test-material/` ist gitignored und enthält echte Auszüge. **Niemals**
  committen oder nach `src-tauri/tests/fixtures/` kopieren.
- Test-Fixtures müssen die dokumentierten Dummy-Ersetzungen verwenden (z. B.
  `DE00 0000 0000 0000 0000 00`, `Max Mustermann`, `BANKDEFFXXX`) — siehe die
  Fixture-Regeln in [`CLAUDE.md`](CLAUDE.md).
- PII (Gegenpartei, IBAN-Suffix, Verwendungszweck) vor dem Logging schwärzen.

Im Zweifel synthetische Daten nutzen und im Issue/PR nachfragen.

### Tests &amp; Qualitäts-Gates

Wir arbeiten **testgetrieben**: erst einen roten Test, dann die minimale
Implementierung. Vor einem PR ausführen — alles muss grün sein:

```bash
cargo test --manifest-path src-tauri/Cargo.toml   # Rust Unit + Integration
pnpm check                                         # svelte-check + tsc (0 Fehler)
pnpm test                                          # Frontend-Unit-Tests (vitest)
pnpm gen:licenses:check                            # nur bei geänderten Dependencies
```

Integrationstests laufen gegen echte SQLite (via `tempfile`), nicht gegen Mocks.

### Abhängigkeiten &amp; Lizenzen

Notchkeep steht unter **GPL-3.0-or-later**. Jede neue Abhängigkeit (direkt oder
transitiv) muss lizenzkompatibel sein. Die zulässigen Lizenzen stehen in
`src-tauri/about.toml`. **Inkompatibel** (nicht hinzufügen): AGPL-3.0, BSL-1.1,
SSPL, Elastic-License-2.0, CC-BY-NC*, proprietär/„all rights reserved".

Nach `cargo add` / `pnpm add`: `pnpm gen:licenses:check` muss grün bleiben.

### Bank-/Broker-Importer hinzufügen

Den `Importer`-Trait in `src-tauri/src/importers/<bank>.rs` implementieren. Der
Caller routet Cash- vs. Trade-Zeilen automatisch, unabhängig vom UI-Konto-Picker.
Jeder Importer braucht die fünf Pflicht-Tests (Happy-Path, idempotenter
Re-Import, Tax-Klassifikation, Trade-Routing, Warnings). Das vollständige
Playbook steht in [`CLAUDE.md`](CLAUDE.md).

### Pull Requests

1. Branch von `main` (oder Fork), Änderungen fokussiert halten.
2. Alle Qualitäts-Gates grün, keine PII im Diff.
3. PR gegen `main` öffnen und das
   [PR-Template](.github/PULL_REQUEST_TEMPLATE.md) ausfüllen.
4. Conventional-Commit-Stil (`feat:`, `fix:`, `docs:`, …) ist erwünscht.
   Commit-Messages und PR-Beschreibungen dürfen **deutsch oder englisch** sein.

Neu bei Open Source? Die [Open Source Guides](https://opensource.guide/) sind ein
guter Einstieg.

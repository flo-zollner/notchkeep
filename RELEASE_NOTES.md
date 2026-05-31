# Notchkeep v0.2.3 — Wartung & CI

**Simple Personal Networth.**

Wartungs-Release ohne funktionale Änderungen. Keine Breaking Changes, keine
Datenmigration — der bestehende lokale Datenbestand bleibt unverändert.

Wer auf **v0.2.2** ist und Auto-Updates aktiviert hat, bekommt dieses Update
angeboten — es ist zugleich der erste Praxistest der Auto-Update-Kette.

## Änderungen (intern)

- **CI-Testgate:** cargo-Tests (App + Plugins), Lizenz-Drift-Prüfung,
  svelte-check, vitest und Playwright-e2e laufen jetzt bei jedem Push/PR — und
  als Gate vor jedem Release (ein roter Test verhindert ein Release).
- **Clippy als Gate:** alle Lints bereinigt, `clippy -D warnings` ist Teil der CI.
- **Parallele Release-Pipeline:** Tests → Draft anlegen → die vier Builds
  (Linux, Windows, macOS, Android) laufen parallel statt nacheinander.

Die in **v0.2.2** eingeführten Auto-Updates (Desktop via Tauri-Updater,
Android via signiertem In-App-APK-Updater) sind unverändert enthalten.

## Privatsphäre

Daten bleiben lokal in SQLite. Optional via Syncthing zwischen deinen Geräten
synchronisiert — keine Cloud, keine Telemetrie. Der Update-Check ist ein
opt-in Versions-Abruf, keine Telemetrie.

## Plattformen

Binary-Bundles für:
- Linux (`.AppImage`, `.deb`, `.rpm`)
- macOS (`.dmg`, Apple Silicon)
- Windows (`.msi`, `.exe`)
- Android (`.apk`)

## Build aus Source

```bash
pnpm install --frozen-lockfile
pnpm gen:licenses
pnpm tauri build
```

Details siehe [README.md](README.md).

## Lizenz

Notchkeep ist freie Software unter **GPL-3.0-or-later** (siehe `COPYING`).
Vollständige Drittanbieter-Lizenzen in `static/licenses.html` (im App-Bundle
unter *Einstellungen → Über*) bzw. [THIRD_PARTY_LICENSES.md](THIRD_PARTY_LICENSES.md).

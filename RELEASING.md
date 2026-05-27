# Release-Runbook

Schritt-für-Schritt-Anleitung für jedes Release von `Notchkeep`. Erfüllt die
Pflichten aus GPL-3.0 §§4–7 sowie die Notice-Pflichten aller Drittbibliotheken.

> **Konvention**: Releases werden semver-versioniert (`vMAJOR.MINOR.PATCH`),
> z. B. `v0.6.0`. Im Folgenden steht `vX.Y.Z` für die neue Version.

---

## Phase 1 — Vor dem Release (Vorbereitung)

### 1.1 Code-Stand

- [ ] Auf `main` und sauberes Working-Tree (`git status` leer, alles gemergt).
- [ ] Letzten `git pull --rebase` gemacht.
- [ ] CI grün (falls vorhanden).

### 1.2 Version bumpen

Vier Stellen müssen synchron sein:

| Datei                                  | Feld                       |
| -------------------------------------- | -------------------------- |
| `package.json`                         | `"version"`                |
| `src-tauri/Cargo.toml`                 | `version`                  |
| `src-tauri/tauri.conf.json`            | `"version"`                |
| `src/lib/i18n/strings.ts`              | `common.version` (`de`+`en`) |

- [ ] Alle vier Stellen auf `X.Y.Z` gesetzt.
- [ ] `pnpm install` einmal laufen lassen, damit `pnpm-lock.yaml` aktualisiert wird.
- [ ] `cargo update -p notchkeep` (aktualisiert `Cargo.lock` für die neue Version).

### 1.3 Qualität sicherstellen

- [ ] `npm run check` → 0 Errors / 0 Warnings
- [ ] `cargo test --manifest-path src-tauri/Cargo.toml` → grün
- [ ] (Optional, langsam) `cargo mutants --manifest-path src-tauri/Cargo.toml` zeigt
      keine neuen Survivors gegenüber dem Stand vor diesem Release.

### 1.4 Lizenzen (GPL-Pflicht)

- [ ] `pnpm gen:licenses:check` → Exit 0 (kein Crate mit nicht-akzeptierter Lizenz)
- [ ] `pnpm gen:licenses` → neue `static/licenses.html` erzeugt
- [ ] `git diff static/licenses.html` geprüft (Änderungen plausibel)
- [ ] `static/licenses.html` committen

### 1.5 Doku / About

- [ ] Source-URL in `src/routes/settings/+page.svelte` (`SOURCE_URL`) zeigt auf
      das tatsächliche Repository.
- [ ] „Über Notchkeep"-Karte in *Einstellungen* lokal geprüft (`pnpm tauri dev`):
      Lizenz, Copyright, Source-Link funktionieren, `licenses.html` öffnet sich.
- [ ] README-Build-Anleitung passt zur aktuellen Toolchain.

### 1.6 Changelog

- [ ] `CHANGELOG.md` (falls vorhanden) aktualisiert mit Features / Fixes seit
      letztem Release. Konventionen siehe [keepachangelog.com](https://keepachangelog.com/de/1.1.0/).
- [ ] User-sichtbare Breaking-Changes klar markiert.

### 1.7 Release-Commit

- [ ] `git commit -am "chore: release vX.Y.Z"` (Versions-Bumps + License-Bundle)
- [ ] `git push`

---

## Phase 2 — Build (Release-Artefakte erzeugen)

Ausführen auf jeder Ziel-Plattform (Linux, macOS, Windows) bzw. via CI.

```bash
# Reproducible Build
pnpm install --frozen-lockfile
pnpm gen:licenses              # darf gegenüber Phase 1 keine Änderungen produzieren
pnpm tauri build               # bundelt nach src-tauri/target/release/bundle/
```

- [ ] Build erfolgreich auf allen Zielplattformen.
- [ ] Artefakte unter `src-tauri/target/release/bundle/` vorhanden
      (`*.AppImage`, `*.deb`, `*.dmg`, `*.msi`, `*.exe`, …).
- [ ] Smoke-Test pro Plattform: Bundle installiert/öffnet sich, Hauptansicht lädt,
      *Einstellungen → Über → Drittanbieter-Lizenzen* öffnet `licenses.html`
      mit allen Einträgen.

---

## Phase 3 — Tag & GitHub-Release

### 3.1 Tag setzen

```bash
git tag -a vX.Y.Z -m "release vX.Y.Z"
git push origin vX.Y.Z
```

- [ ] Tag erstellt und gepusht.
- [ ] Der Tag zeigt auf den Release-Commit aus Phase 1.7.

### 3.2 GitHub-Release anlegen

```bash
gh release create vX.Y.Z \
  --title "vX.Y.Z" \
  --notes-file RELEASE_NOTES.md \
  src-tauri/target/release/bundle/appimage/*.AppImage \
  src-tauri/target/release/bundle/deb/*.deb \
  src-tauri/target/release/bundle/dmg/*.dmg \
  src-tauri/target/release/bundle/msi/*.msi
```

- [ ] Alle Binary-Bundles als Assets hochgeladen.
- [ ] Release-Notes enthalten:
  - Versions-Highlights
  - Lizenz-Hinweis: „Notchkeep ist freie Software unter
    [GPL-3.0-or-later](../COPYING). Der vollständige Source-Code dieses
    Releases ist über den Tag `vX.Y.Z` und das automatisch erzeugte
    Source-Archiv (`Source code (tar.gz)`) verfügbar."
  - SHA256-Summen der Binaries (`sha256sum *.AppImage *.deb …`).

### 3.3 Source-Archiv (automatisch durch GitHub)

GitHub generiert `Source code (tar.gz)` und `(zip)` aus dem Tag automatisch.
Diese decken GPLv3 §6c ab — kein manueller Schritt nötig, **wenn das Repo
public ist**.

- [ ] Repository ist `public`.
- [ ] Tarball-URL ist im Release sichtbar.

---

## Phase 4 — Nach dem Release

### 4.1 Verifikation

- [ ] Mindestens eines der Binary-Assets heruntergeladen, installiert, gestartet.
- [ ] *Einstellungen → Über* zeigt korrekte Version `vX.Y.Z`.
- [ ] Lizenz-Link öffnet die volle `licenses.html`.
- [ ] Source-Link öffnet das öffentliche Repo.
- [ ] SHA256-Prüfsummen der heruntergeladenen Bundles stimmen mit Release-Notes überein.

### 4.2 Aufräumen

- [ ] Lokales `node_modules`, `target/` und `build/` können (müssen aber nicht)
      gelöscht werden — Disk-Aufräumung.
- [ ] Branch-Cleanup (`git fetch --prune`, `git branch --merged | xargs git branch -d`).

### 4.3 Kommunikation (optional)

- [ ] Release auf relevanten Kanälen ankündigen (Blog, Mastodon, …).
- [ ] Falls Issues / Diskussionen das Release betreffen: dort Link zum Release posten.

### 4.4 Nächsten Entwicklungszyklus starten

- [ ] Version in den vier Dateien aus 1.2 auf nächste `MINOR-dev` bumpen
      (z. B. `0.6.1-dev`), damit dev-Builds nicht mit der Release-Version
      verwechselt werden. (Optional, Konventionssache.)
- [ ] Eventuelle Hotfix-Branches aus `vX.Y.Z` ableiten, falls nötig.

---

## Quick-Reference (Kurzform für erfahrene Releases)

```bash
# 1. Vorbereiten
$EDITOR package.json src-tauri/Cargo.toml src-tauri/tauri.conf.json \
        src/lib/i18n/strings.ts CHANGELOG.md
pnpm install && cargo update -p notchkeep
npm run check && cargo test --manifest-path src-tauri/Cargo.toml
pnpm gen:licenses
git commit -am "chore: release vX.Y.Z" && git push

# 2. Bauen
pnpm install --frozen-lockfile && pnpm tauri build

# 3. Taggen & Releasen
git tag -a vX.Y.Z -m "release vX.Y.Z" && git push origin vX.Y.Z
gh release create vX.Y.Z --notes-file RELEASE_NOTES.md \
   src-tauri/target/release/bundle/**/*

# 4. Verifizieren
# → Bundle herunterladen, starten, Über-Sektion prüfen, SHA256 vergleichen
```

---

## Was bei einem Hotfix-Release anders ist

- Branch `hotfix/vX.Y.(Z+1)` aus dem Tag `vX.Y.Z` ableiten.
- Nur den Fix einspielen, **nicht** mit `main` mergen (das passiert über
  einen separaten PR nach dem Release).
- Phasen 1–4 wie oben, aber `--target hotfix/vX.Y.(Z+1)` beim Tag.

## Was bei einer Backports / Sicherheitslücke anders ist

- Coordinated Disclosure via `SECURITY.md` (falls vorhanden) vorab.
- Source-Tag und Binary müssen **gleichzeitig** verfügbar sein — GPLv3 §6
  verlangt, dass die Source-Verfügbarkeit nicht hinter der Binary herhinkt.

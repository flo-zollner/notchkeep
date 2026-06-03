# Notchkeep v0.2.4-rc.1 — Release Candidate (CI-Sicherheit & Release-Tuning)

**Simple Personal Networth.**

> **Hinweis: Dies ist ein Release Candidate (Vorabversion).** RC-Builds sind als
> *Prerelease* markiert und werden **nicht** automatisch als Update angeboten —
> die Auto-Update-Kette für stabile Nutzer bleibt unberührt. Zum Testen manuell
> installieren. Keine Breaking Changes, keine Datenmigration — der bestehende
> lokale Datenbestand bleibt unverändert.

## Änderungen (intern)

- **Security-/Quality-Pipeline in der CI:** statische Analyse, CVE-/Advisory-Scan
  (cargo-deny, OSV), Pentest-Checks (Dateirechte, Logfile-PII, Zertifikats-,
  Versions- und Update-Endpoint-Prüfung), SBOM-Erzeugung und ein SARIF-Report.
  Lokale Check-Skripte und ein ESLint-Bug-Gate ergänzen das bestehende Testgate.
- **DB-Dateirechte:** lokale SQLite-Datei wird mit `0600` (nur Eigentümer) angelegt.
- **Schlankere Release-Artefakte:** die Windows-`.msi` (WiX) entfällt — die
  NSIS-`.exe` deckt Installation **und** Update ab. Die redundanten standalone
  `.sig`-Assets werden entfernt (die Signatur liegt inline im Update-Manifest).
- **RC-Kanal:** Tags der Form `vX.Y.Z-rc.N` werden als Prerelease veröffentlicht
  und vom Updater nie an stabile Nutzer ausgeliefert.

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
- Windows (`.exe`, NSIS)
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

# Notchkeep v0.2.4-rc.2 — Release Candidate (Update-Channel: Stable/Beta)

**Simple Personal Networth.**

> **Hinweis: Dies ist ein Release Candidate (Vorabversion).** RC-Builds sind als
> *Prerelease* markiert und werden **nicht** automatisch als Update angeboten —
> die Auto-Update-Kette für stabile Nutzer bleibt unberührt. Zum Testen manuell
> installieren. Keine Breaking Changes, keine Datenmigration — der bestehende
> lokale Datenbestand bleibt unverändert.

## Neu

- **Wählbarer Update-Channel (Stable / Beta).** In den Einstellungen lässt sich
  der Release-Channel umstellen:
  - **Stable** (Default): nur stabile Updates.
  - **Beta**: erhält zusätzlich Release Candidates (Vorabversionen).

  Der Wechsel **zu Beta** zeigt jedes Mal eine Warnung mit Bestätigung — inkl.
  Hinweis, dass man auf einer Vorabversion bleibt, bis Stable sie überholt (es
  gibt kein automatisches Zurückstufen). Zurück auf Stable ist ohne Nachfrage.

## Technisch

- **Channel-spezifische Update-Quelle:** Stable liest GitHub `/releases/latest/`
  (immer das neueste Nicht-Prerelease), Beta ein fortlaufendes Manifest inkl.
  Vorabversionen. Desktop schaltet die Quelle zur Laufzeit über die offizielle
  Updater-Maschinerie um (Signaturprüfung und Download/Install bleiben
  unverändert); Android über den bestehenden APK-Updater. **Keine neuen
  Abhängigkeiten.**
- **Korrektere Versions-Reihenfolge:** Prerelease-Präzedenz nach SemVer
  (`0.3.0` ist neuer als `0.3.0-rc.2`, `rc.2` neuer als `rc.1`).

## Hinweise

- Der Beta-Channel wird erst ab dem ersten regulären Release scharf, das das
  fortlaufende Update-Manifest erzeugt; **Stable funktioniert sofort.**

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

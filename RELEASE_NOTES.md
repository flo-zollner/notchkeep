# Notchkeep v0.2.2 — Auto-Updates

**Simple Personal Networth.**

Dieses Release bringt **optionale automatische Updates**. Keine Breaking
Changes, keine Datenmigration — der bestehende lokale Datenbestand bleibt
unverändert.

## Neu: Auto-Updates (opt-in)

- **Einmal aktivieren, dann still prüfen:** Beim Start fragt die App einmalig,
  ob sie nach Updates suchen darf. Erst nach Zustimmung wird GitHub kontaktiert
  — es werden **keine persönlichen Daten** übertragen.
- **Update-Dialog nur bei echtem Update:** Versionsinfo + Änderungsnotizen,
  Download mit Fortschritt, dann Neustart-Prompt (Desktop). Eine konkrete
  Version lässt sich gezielt überspringen.
- **Steuerung in den Einstellungen:** Schalter „Automatisch nach Updates
  suchen" + Button „Jetzt prüfen".

### Plattformen

- **Desktop (`.AppImage`, Windows-`.exe`/NSIS, macOS-`.app`):** Self-Replace
  über den Tauri-Updater, signierte Artefakte.
- **Android (Sideload-`.apk`):** In-App-Updater lädt die signierte APK von der
  GitHub-Release-Seite, verifiziert sie (sha256 + minisign-Signatur) und startet
  den System-Installer. `.deb`/`.rpm` aktualisieren wie gehabt über den
  Paketmanager.

## Hinweis

Dies ist das **Baseline-Release** für Auto-Updates: Ab dieser Version finden
künftige Updates (nach Aktivierung) automatisch statt. Bestehende
Installationen müssen einmalig manuell auf v0.2.2 wechseln.

Sicherheit: Alle Update-Artefakte sind signiert; der Client installiert nur
verifizierte Pakete.

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

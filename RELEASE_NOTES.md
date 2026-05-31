# Notchkeep v0.2.1 — Onboarding

**Simple Personal Networth.**

Dieses Release bringt ein geführtes Erst-Start-Onboarding. Keine Breaking
Changes, keine Datenmigration nötig — der bestehende lokale Datenbestand wird
unverändert weiterverwendet.

## Neu: Onboarding

- **Setup-Assistent beim ersten Start:** Sprache (DE/EN) und Theme wählen,
  optional direkt das erste Konto anlegen, kurzer Import-Hinweis. Startet nur
  bei frischer Installation (kein Konto vorhanden); bestehende Installationen
  werden nicht unterbrochen.
- **Interaktive Feature-Tour:** Coach-Marks über Dashboard, Navigation,
  Transaktionen, „Neue Transaktion", Budgets und Import — mit Spotlight auf das
  jeweils passende Bedienelement.
- **Responsiv über alle Größen:** Der Assistent wird auf dem Telefon zum
  Bottom-Sheet, auf größeren Schirmen zum zentrierten Dialog; die Tour
  highlightet je nach Breite die volle Seitenleiste, die eingeklappte Icon-Leiste
  (Tablet) oder die untere Tab-Leiste (Telefon).
- Erneut startbar über *Einstellungen* (Assistent und Tour einzeln).

Die Sicherheits- und Härtungs-Änderungen aus v0.2.0 sind weiterhin enthalten
(siehe unten).

## Sicherheit & Härtung

**WebView / Tauri**
- **Content Security Policy gesetzt** (war zuvor deaktiviert): `default-src 'self'`,
  `script-src 'self'`, `object-src 'none'`, `base-uri 'self'` + `freezePrototype`.
  Begrenzt den Schaden eines etwaigen XSS auf den IPC-Layer.
- Übrig gebliebener `greet`-Debug-Command aus dem IPC-Handler entfernt.

**Privatsphäre (keine PII in Logs/Fehlern)**
- IBANs werden nicht mehr in Fehlermeldungen zurückgegeben.
- Geräte-Hostnames in Logs maskiert.
- Absolute Datenbank-Pfade aus Fehlermeldungen entfernt.
- Importer-Fehler geben keine rohen CSV-Feldwerte mehr aus.

**Datenintegrität & Robustheit**
- CSV-Betragsparser nutzt geprüfte Arithmetik (kein stiller Integer-Overflow
  bei manipuliertem Betrag).
- Kursabruf: nicht-endliche/überlaufende Werte (NaN/∞) werden abgewiesen statt
  als Null- oder Maximalwert in die DB zu gelangen; negative FX-Raten werden
  zurückgewiesen.
- Netzwerk: `https_only` erzwungen (kein HTTP-Downgrade via Redirect) +
  Connect-Timeout (kein Hängen bei Netzwerkproblemen).
- Regel-Aktualisierung läuft in einer Transaktion (keine inkonsistenten
  Teil-Schreibvorgänge mehr).
- SQLite `busy_timeout` gesetzt (robuster bei parallelem Zugriff / Syncthing).

**Eingabe-Validierung**
- Manuelle Kurs-Eingabe validiert das Datumsformat.
- Backup-Validierung über typsichere Connection-Optionen (read-only) statt
  String-URL.

## Privatsphäre

Daten bleiben lokal in SQLite. Optional via Syncthing zwischen deinen
Geräten synchronisiert — keine Cloud, keine Telemetrie.

## Plattformen

Binary-Bundles für:
- Linux (`.AppImage`, `.deb`, `.rpm`)
- macOS (`.dmg`)
- Windows (`.msi`)
- Android (`.apk` über `pnpm tauri android build`)

Dieses Release enthält die lokal gebauten **Linux**-Bundles. Checksummen
(SHA-256) stehen unten im Release-Eintrag.

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
Keine neuen Abhängigkeiten gegenüber v0.1.0.

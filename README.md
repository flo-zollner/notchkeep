# Notchkeep

**Simple Personal Networth.**

Lokales Haushaltsbuch (Tauri 2 + SvelteKit + TypeScript + SQLite/sqlx).

## Konzept

Daten werden lokal in SQLite gehalten; optional via Syncthing zwischen Geräten
synchronisiert. Trade-Republic-CSVs werden direkt importiert; weitere Banken
sind über das `Importer`-Trait andockbar.

**Kontomodell:**

- **Institute** (Banken/Broker): Container mit Name, Icon, Farbe, optional
  BIC und Land. Ein Institut kann 0–N Konten verschiedener Typen tragen
  (z. B. Trade Republic: ein Verrechnungskonto + ein Depot).
- **Konten** (`accounts`): Giro (`bank`), Tagesgeld (`savings`), Depot
  (`broker`), Kredit (`credit`), Bargeld (`cash`), Schuld (`loan`). Optional
  einem Institut zugeordnet (`institution_id`); Bargeld/Schuld bleiben
  typischerweise ohne Institut.
- **Subkonten-Hierarchie** über `parent_id` orthogonal zum Institut, für
  virtuelle Aufteilungen unterhalb eines Kontos.
- **Töpfe** (`buckets`) für projektbezogene Sparziele, unabhängig von
  Kategorien.
- **Wertpapiere & Trades** an Broker-Konten (FIFO-Cost-Basis,
  Online-Kurse via Yahoo Finance).

Alle Beträge in Cent (Integer); Konvertierung erst an der UI-Grenze.

## Build aus dem Quellcode

### Voraussetzungen

| Komponente   | Version                                                   |
| ------------ | --------------------------------------------------------- |
| Node.js      | ≥ 20 (LTS)                                                |
| pnpm         | ≥ 9 (`corepack enable pnpm`)                              |
| Rust         | aktuelle Stable-Toolchain via [rustup](https://rustup.rs) |
| Tauri 2 deps | Plattform-Abhängigkeiten siehe https://tauri.app/start/prerequisites |

### Entwicklung

```bash
pnpm install
pnpm tauri dev
```

### Release-Build

```bash
pnpm install --frozen-lockfile
pnpm gen:licenses                  # generiert static/licenses.html
pnpm tauri build                   # bundelt Binaries unter src-tauri/target/release/bundle/
```

Der Befehl `pnpm tauri build` erzeugt plattform-spezifische Bundles (`.deb`,
`.AppImage`, `.dmg`, `.msi`, …). Die generierte `licenses.html` ist Teil des
SvelteKit-Builds (`static/`) und wird in jedes Bundle eingebettet.

## Lizenz

Copyright © 2026 Florian Zollner

Dieses Programm ist freie Software: Du darfst es weitergeben und/oder modifizieren
unter den Bedingungen der **GNU General Public License**, wie von der
Free Software Foundation veröffentlicht — Version 3 der Lizenz oder (nach
deiner Wahl) jede spätere Version.

Dieses Programm wird in der Hoffnung verteilt, dass es nützlich sein wird,
jedoch **OHNE JEDE GEWÄHRLEISTUNG**; auch ohne die implizite Gewährleistung
der MARKTGÄNGIGKEIT oder EIGNUNG FÜR EINEN BESTIMMTEN ZWECK.
Siehe die GNU General Public License für Details.

Der vollständige Lizenztext liegt in [`COPYING`](COPYING).
Lizenz-Audit und Drittabhängigkeiten:
[`THIRD_PARTY_LICENSES.md`](THIRD_PARTY_LICENSES.md).
Der vollständige Notice-Bericht (Volltext aller Dep-Lizenzen) wird beim
Release in `static/licenses.html` gebaut und ist in der App unter
*Einstellungen → Über* erreichbar.

SPDX-License-Identifier: `GPL-3.0-or-later`

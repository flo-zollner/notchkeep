# Notchkeep

**Simple Personal Networth.**

Local personal finance tracker (Tauri 2 + SvelteKit + TypeScript + SQLite/sqlx).

## Concept

Data is stored locally in SQLite and optionally synced between devices via Syncthing.
Trade Republic CSVs are imported directly; additional banks can be plugged in via the
`Importer` trait.

**Account model:**

- **Institutions** (banks/brokers): containers with a name, icon, color, and optional
  BIC and country. An institution can hold 0–N accounts of different types
  (e.g. Trade Republic: one settlement account + one brokerage account).
- **Accounts** (`accounts`): checking (`bank`), savings (`savings`), brokerage
  (`broker`), credit (`credit`), cash (`cash`), loan (`loan`). Optionally assigned
  to an institution (`institution_id`); cash and loan accounts typically remain
  without an institution.
- **Sub-account hierarchy** via `parent_id` orthogonal to the institution, for
  virtual splits below an account.
- **Buckets** (`buckets`) for project-based savings goals, independent of categories.
- **Securities & trades** on brokerage accounts (FIFO cost basis,
  live prices via Yahoo Finance).

All amounts in cents (integer); conversion to decimals only at the UI boundary.

## Building from source

### Prerequisites

| Component    | Version                                                   |
| ------------ | --------------------------------------------------------- |
| Node.js      | ≥ 22 (LTS)                                                |
| pnpm         | ≥ 11 (`corepack enable pnpm`)                             |
| Rust         | current stable toolchain via [rustup](https://rustup.rs) |
| Tauri 2 deps | platform dependencies: https://tauri.app/start/prerequisites |

### Development

```bash
pnpm install
pnpm tauri dev
```

### Release build

```bash
pnpm install --frozen-lockfile
pnpm gen:licenses                  # generates static/licenses.html
pnpm tauri build                   # bundles binaries under src-tauri/target/release/bundle/
```

`pnpm tauri build` produces platform-specific bundles (`.deb`, `.AppImage`,
`.dmg`, `.msi`, …). The generated `licenses.html` is part of the SvelteKit
build (`static/`) and is embedded in every bundle.

## License

Copyright © 2026 Florian Zollner

This program is free software: you can redistribute it and/or modify it under
the terms of the **GNU General Public License** as published by the Free Software
Foundation — either version 3 of the License, or (at your option) any later
version.

This program is distributed in the hope that it will be useful, but
**WITHOUT ANY WARRANTY**; without even the implied warranty of
MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
See the GNU General Public License for more details.

The full license text is in [`COPYING`](COPYING).
License audit and third-party dependencies:
[`THIRD_PARTY_LICENSES.md`](THIRD_PARTY_LICENSES.md).
The complete notice report (full text of all dependency licenses) is built at
release time into `static/licenses.html` and is accessible in the app under
*Settings → About*.

SPDX-License-Identifier: `GPL-3.0-or-later`

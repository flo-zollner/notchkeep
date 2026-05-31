# Security Policy

[English](#english) · [Deutsch](#deutsch)

---

## English

### Supported versions

Notchkeep is pre-1.0 and ships from a single release line. Security fixes are
made against the **latest release** and `main`. Please always reproduce on the
latest version before reporting.

| Version            | Supported |
| ------------------ | --------- |
| Latest release     | ✅        |
| Older releases     | ❌        |

### Reporting a vulnerability

**Please do not open a public issue for security problems.**

Report privately through GitHub's **"Report a vulnerability"** flow:

1. Go to the [Security tab](https://github.com/flo-zollner/notchkeep/security)
   of the repository.
2. Click **"Report a vulnerability"** (Private Vulnerability Reporting).
3. Describe the issue, affected version/commit, and steps to reproduce.

> ⚠️ When attaching logs, screenshots or sample files, **redact all real
> financial data** (IBANs, account numbers, names, amounts). Use synthetic data
> wherever possible.

We aim to acknowledge a report within a few days and to keep you updated as we
work on a fix. Once a fix is released we are happy to credit you, unless you
prefer to stay anonymous.

### Threat model &amp; scope

Notchkeep is a **local-first desktop/Android app**. There is no server backend,
and your data lives in a local SQLite file that only you control. Sync, if
enabled, runs over your own [Syncthing](https://syncthing.net) setup. There is no
telemetry.

In scope:

- IPC / Tauri command handlers and the WebView boundary (e.g. CSP bypass, XSS
  escalating to IPC).
- Parsing of **untrusted import files** (CSV/PDF bank statements) — crashes,
  panics, integer overflow, path traversal, resource exhaustion.
- Local data handling: database access, backup/restore, file paths.
- Leakage of **PII into logs or error messages**.
- Vulnerable third-party dependencies.

Out of scope:

- Issues that require an already-compromised device / OS account.
- Social-engineering or physical access attacks.
- Findings in your own Syncthing or network configuration.

---

## Deutsch

### Unterstützte Versionen

Notchkeep ist vor 1.0 und wird aus einer einzigen Release-Linie ausgeliefert.
Sicherheits-Fixes erfolgen gegen das **aktuelle Release** und `main`. Bitte vor
einer Meldung immer auf der neuesten Version reproduzieren.

| Version          | Unterstützt |
| ---------------- | ----------- |
| Aktuelles Release| ✅          |
| Ältere Releases  | ❌          |

### Eine Sicherheitslücke melden

**Bitte für Sicherheitsprobleme kein öffentliches Issue öffnen.**

Melde sie privat über den GitHub-Ablauf **„Report a vulnerability"**:

1. Auf den [Security-Tab](https://github.com/flo-zollner/notchkeep/security) des
   Repositorys gehen.
2. **„Report a vulnerability"** anklicken (Private Vulnerability Reporting).
3. Problem, betroffene Version/Commit und Reproduktionsschritte beschreiben.

> ⚠️ Beim Anhängen von Logs, Screenshots oder Beispieldateien **alle echten
> Finanzdaten schwärzen** (IBANs, Kontonummern, Namen, Beträge). Wo möglich
> synthetische Daten verwenden.

Wir bestätigen den Eingang in der Regel innerhalb weniger Tage und halten dich
über den Fortschritt auf dem Laufenden. Nach einem Fix nennen wir dich gern als
Melder:in, sofern du nicht anonym bleiben möchtest.

### Threat-Model &amp; Geltungsbereich

Notchkeep ist eine **local-first Desktop-/Android-App**. Es gibt kein
Server-Backend; deine Daten liegen in einer lokalen SQLite-Datei, die nur du
kontrollierst. Sync läuft – falls aktiviert – über dein eigenes
[Syncthing](https://syncthing.net)-Setup. Es gibt keine Telemetrie.

Im Geltungsbereich:

- IPC-/Tauri-Command-Handler und die WebView-Grenze (z. B. CSP-Umgehung, XSS, das
  zum IPC eskaliert).
- Parsen **nicht vertrauenswürdiger Import-Dateien** (CSV/PDF-Auszüge) — Crashes,
  Panics, Integer-Overflow, Path-Traversal, Ressourcen-Erschöpfung.
- Lokale Datenverarbeitung: DB-Zugriff, Backup/Restore, Dateipfade.
- Abfluss von **PII in Logs oder Fehlermeldungen**.
- Verwundbare Drittanbieter-Abhängigkeiten.

Nicht im Geltungsbereich:

- Probleme, die ein bereits kompromittiertes Gerät / OS-Konto voraussetzen.
- Social-Engineering oder Angriffe mit physischem Zugriff.
- Befunde in deiner eigenen Syncthing- oder Netzwerk-Konfiguration.

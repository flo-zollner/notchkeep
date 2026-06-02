/**
 * Shim für @tauri-apps/plugin-os im Browser-only-Modus (Vite mode=mock).
 *
 * Die echte Plugin-API liest `window.__TAURI_OS_PLUGIN_INTERNALS__` synchron.
 * Im Browser existiert dieses Global nicht, weshalb jeder Aufruf von
 * `platform()` mit "Cannot read properties of undefined (reading 'platform')"
 * crasht — und damit jedes Modul killt, das beim Laden `platform()` aufruft
 * (z. B. `updater.svelte`, das den Update-Backend anhand der Plattform wählt).
 *
 * Aktiviert via Vite-Alias in vite.config.js (mode=mock). Im echten tauri-dev-
 * Modus wird dieser File nicht geladen. Gibt Desktop-/Web-Werte zurück, damit
 * die App im Mock bootet; die UI-Plattform-Schicht (Material Design auf
 * Android) wird unabhängig über `?platform=` / `data-platform` gesteuert.
 */
export function platform(): string {
  return 'web';
}
export function type(): string {
  return 'linux';
}
export function version(): string {
  return '0.0.0';
}
export function family(): string {
  return 'unix';
}
export function arch(): string {
  return 'x86_64';
}
export function locale(): Promise<string | null> {
  return Promise.resolve('de-DE');
}
export function eol(): string {
  return '\n';
}

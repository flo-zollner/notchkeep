/**
 * Shim für @tauri-apps/api/core's `invoke` im Browser-only-Modus
 * (Vite mode=mock). Dispatcht Tauri-Commands an registrierte Handler statt
 * an das Rust-Backend.
 *
 * Aktiviert via Vite alias in vite.config.js (mode=mock). Im normalen
 * tauri-dev-Modus wird dieser File nicht geladen.
 */

export type MockHandler = (args: unknown) => unknown | Promise<unknown>;

export type HandlerRegistry = Record<string, MockHandler>;

export interface MockCommandError {
  message: string;
}

/**
 * Baut eine `invoke`-Funktion, die Commands auf die übergebene Registry routet.
 * Unbekannte Commands rejecten mit einem CommandError-shaped Objekt
 * (kompatibel mit `errMsg()` in api.ts, das `.message` extrahiert).
 */
export function createMockInvoke(registry: HandlerRegistry) {
  return async function invoke<T = unknown>(cmd: string, args?: unknown): Promise<T> {
    const handler = registry[cmd];
    if (!handler) {
      const err: MockCommandError = {
        message: `Mock-Tauri: kein Handler für Command "${cmd}" registriert`,
      };
      throw err;
    }
    return (await handler(args)) as T;
  };
}

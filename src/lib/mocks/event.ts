/**
 * Shim für @tauri-apps/api/event's `listen`/`emit` im Browser-only-Modus
 * (Vite mode=mock). Im echten Tauri-WebView greift die echte Event-API auf
 * `window.__TAURI_INTERNALS__.transformCallback` zu — das existiert im
 * Browser nicht, weshalb jeder `listen(...)`-`$effect` ohne diesen Shim mit
 * "Cannot read properties of undefined (reading 'transformCallback')" crasht.
 *
 * Aktiviert via Vite alias in vite.config.js (mode=mock). Im normalen
 * tauri-dev-Modus wird dieser File nicht geladen.
 *
 * Wir halten Handler in einer In-Memory-Map pro Event-Name. Backend-Events
 * (`price_refresh_status`, `data_path_error`) feuern im Mock nie von selbst;
 * `emit` ist exportiert, damit Mock-Handler oder Tests Events simulieren
 * können (z. B. Import-/Price-Refresh-Progress).
 */

// Typen kompatibel zur echten @tauri-apps/api/event-Signatur gehalten.
export type EventName = string;

export interface Event<T> {
  event: EventName;
  id: number;
  payload: T;
}

export type EventCallback<T> = (event: Event<T>) => void;

export type UnlistenFn = () => void;

const listeners = new Map<EventName, Set<EventCallback<unknown>>>();
let nextId = 1;

export function listen<T>(
  event: EventName,
  handler: EventCallback<T>,
): Promise<UnlistenFn> {
  let set = listeners.get(event);
  if (!set) {
    set = new Set();
    listeners.set(event, set);
  }
  const cb = handler as EventCallback<unknown>;
  set.add(cb);

  return Promise.resolve(() => {
    set?.delete(cb);
  });
}

export function emit<T>(event: EventName, payload?: T): Promise<void> {
  const set = listeners.get(event);
  if (set) {
    const ev: Event<T> = { event, id: nextId++, payload: payload as T };
    // Kopie iterieren, damit ein Handler, der sich selbst abmeldet, das
    // laufende Dispatch nicht stört.
    for (const cb of [...set]) {
      (cb as EventCallback<T>)(ev);
    }
  }
  return Promise.resolve();
}

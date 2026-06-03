/**
 * Snackbar store (UX guide §19): one transient message with at most one action
 * (e.g. "Rückgängig"). Used for reversible deletes — show after a soft-delete,
 * the action restores it. 4–10 s display window, a new snackbar replaces the
 * previous one. Rendered by Snackbar.svelte (mounted once in +layout).
 */

export interface SnackbarState {
  id: number;
  message: string;
  actionLabel: string | null;
  onAction: (() => void | Promise<void>) | null;
}

export interface ShowOpts {
  actionLabel?: string;
  onAction?: () => void | Promise<void>;
  /** Display duration in ms (guide §19: 4000–10000). Default 6000. */
  durationMs?: number;
}

class SnackbarStore {
  current = $state<SnackbarState | null>(null);
  #timer: ReturnType<typeof setTimeout> | null = null;
  #seq = 0;

  show(message: string, opts: ShowOpts = {}) {
    this.#clearTimer();
    const id = ++this.#seq;
    this.current = {
      id,
      message,
      actionLabel: opts.actionLabel ?? null,
      onAction: opts.onAction ?? null,
    };
    const dur = Math.min(Math.max(opts.durationMs ?? 6000, 4000), 10000);
    this.#timer = setTimeout(() => {
      if (this.current?.id === id) this.dismiss();
    }, dur);
  }

  /** Convenience for the common "Gelöscht · Rückgängig" pattern. */
  showUndo(message: string, undoLabel: string, onUndo: () => void | Promise<void>) {
    this.show(message, { actionLabel: undoLabel, onAction: onUndo, durationMs: 7000 });
  }

  async runAction() {
    const c = this.current;
    this.#clearTimer();
    this.current = null;
    if (c?.onAction) await c.onAction();
  }

  dismiss() {
    this.#clearTimer();
    this.current = null;
  }

  #clearTimer() {
    if (this.#timer) {
      clearTimeout(this.#timer);
      this.#timer = null;
    }
  }
}

export const snackbar = new SnackbarStore();

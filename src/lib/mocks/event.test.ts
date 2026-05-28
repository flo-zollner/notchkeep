import { describe, it, expect, vi } from 'vitest';
import { listen, emit } from './event';

describe('mock event API', () => {
  it('listen registers a handler that receives emitted payloads', async () => {
    const cb = vi.fn();
    await listen<{ stage: string }>('price_refresh_status', cb);

    await emit('price_refresh_status', { stage: 'started' });

    expect(cb).toHaveBeenCalledTimes(1);
    const ev = cb.mock.calls[0][0];
    expect(ev.event).toBe('price_refresh_status');
    expect(ev.payload).toEqual({ stage: 'started' });
    expect(typeof ev.id).toBe('number');
  });

  it('listen returns a Promise resolving to an unlisten fn that stops delivery', async () => {
    const cb = vi.fn();
    const unlisten = await listen('data_path_error', cb);

    await emit('data_path_error', { path: '/x' });
    expect(cb).toHaveBeenCalledTimes(1);

    unlisten();
    await emit('data_path_error', { path: '/y' });
    expect(cb).toHaveBeenCalledTimes(1);
  });

  it('only delivers to handlers of the matching event name', async () => {
    const a = vi.fn();
    const b = vi.fn();
    await listen('event_a', a);
    await listen('event_b', b);

    await emit('event_a', 1);

    expect(a).toHaveBeenCalledTimes(1);
    expect(b).not.toHaveBeenCalled();
  });

  it('emitting an event with no listeners is a no-op', async () => {
    await expect(emit('nobody_listening', {})).resolves.toBeUndefined();
  });
});

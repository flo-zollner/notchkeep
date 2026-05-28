import { describe, it, expect } from 'vitest';
import { createMockInvoke, type MockHandler } from './tauri-shim';

describe('createMockInvoke', () => {
  it('dispatches to a registered handler with the provided args', async () => {
    const handler: MockHandler = (args) => ({ echoed: args });
    const invoke = createMockInvoke({ ping: handler });

    const result = await invoke<{ echoed: { msg: string } }>('ping', { msg: 'hi' });

    expect(result).toEqual({ echoed: { msg: 'hi' } });
  });

  it('passes undefined args through to the handler', async () => {
    let received: unknown = 'not-called';
    const invoke = createMockInvoke({
      noop: (args) => {
        received = args;
        return null;
      },
    });

    await invoke('noop');

    expect(received).toBeUndefined();
  });

  it('awaits async handlers and returns their resolved value', async () => {
    const invoke = createMockInvoke({
      slow: async () => {
        await new Promise((r) => setTimeout(r, 1));
        return 42;
      },
    });

    await expect(invoke<number>('slow')).resolves.toBe(42);
  });

  it('rejects with a CommandError-shaped object for unknown commands', async () => {
    const invoke = createMockInvoke({});

    await expect(invoke('does_not_exist')).rejects.toMatchObject({
      message: expect.stringContaining('does_not_exist'),
    });
  });
});

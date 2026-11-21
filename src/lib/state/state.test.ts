import { describe, expect, it, vi } from 'vitest';
import { createKeyHandler } from './keys';
import { applyTheme, theme } from './theme.svelte';

describe('theme and key state helpers', () => {
  it('applies light, dark, and live system themes on documentElement', () => {
    const listeners: Array<(event: MediaQueryListEvent) => void> = [];
    const mockMatchMedia = vi.fn().mockImplementation(() => ({
      matches: true,
      addEventListener: (_type: string, fn: (event: MediaQueryListEvent) => void) => {
        listeners.push(fn);
      },
      removeEventListener: (_type: string, fn: (event: MediaQueryListEvent) => void) => {
        const idx = listeners.indexOf(fn);
        if (idx >= 0) listeners.splice(idx, 1);
      },
    }));
    vi.stubGlobal('matchMedia', mockMatchMedia);

    applyTheme('dark');
    expect(theme.mode).toBe('dark');
    expect(theme.resolved).toBe('dark');
    expect(document.documentElement.dataset.theme).toBe('dark');

    applyTheme('light');
    expect(theme.mode).toBe('light');
    expect(theme.resolved).toBe('light');
    expect(document.documentElement.dataset.theme).toBeUndefined();

    applyTheme('system');
    expect(theme.mode).toBe('system');
    expect(theme.resolved).toBe('dark');
    expect(document.documentElement.dataset.theme).toBe('dark');

    for (const fn of listeners) {
      fn({ matches: false } as MediaQueryListEvent);
    }
    expect(theme.resolved).toBe('light');
    expect(document.documentElement.dataset.theme).toBeUndefined();

    vi.unstubAllGlobals();
  });

  it('ignores shortcuts inside text inputs unless allowInInput is true', () => {
    const onRefresh = vi.fn();
    const onEscape = vi.fn();

    const handler = createKeyHandler([
      { key: 'r', handler: onRefresh },
      { key: 'Escape', allowInInput: true, handler: onEscape },
    ]);

    const input = document.createElement('input');
    const div = document.createElement('div');
    const ce = document.createElement('div');
    ce.setAttribute('contenteditable', 'true');

    handler({ key: 'r', ctrlKey: false, metaKey: false, altKey: false, shiftKey: false, target: input, preventDefault: vi.fn() } as unknown as KeyboardEvent);
    handler({ key: 'r', ctrlKey: false, metaKey: false, altKey: false, shiftKey: false, target: ce, preventDefault: vi.fn() } as unknown as KeyboardEvent);
    expect(onRefresh).not.toHaveBeenCalled();

    handler({ key: 'Escape', ctrlKey: false, metaKey: false, altKey: false, shiftKey: false, target: input, preventDefault: vi.fn() } as unknown as KeyboardEvent);
    expect(onEscape).toHaveBeenCalledTimes(1);

    handler({ key: 'r', ctrlKey: false, metaKey: false, altKey: false, shiftKey: false, target: div, preventDefault: vi.fn() } as unknown as KeyboardEvent);
    expect(onRefresh).toHaveBeenCalledTimes(1);
  });
});

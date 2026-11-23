import '@testing-library/jest-dom/vitest';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { createShortcutHandler, type ShortcutHandlers } from './shortcuts';

function makeHandlers(): ShortcutHandlers {
  return {
    onPrevTab: vi.fn(),
    onNextTab: vi.fn(),
    onSort: vi.fn(),
    onRefresh: vi.fn(),
    onFocusSearch: vi.fn(),
    onClearSearch: vi.fn(),
  };
}

function dispatchKey(
  handler: (event: KeyboardEvent) => void,
  key: string,
  target: EventTarget = document.body,
  init: KeyboardEventInit = {},
): KeyboardEvent {
  const event = new KeyboardEvent('keydown', {
    key,
    bubbles: true,
    cancelable: true,
    ...init,
  });
  Object.defineProperty(event, 'target', { value: target, configurable: true });
  handler(event);
  return event;
}

describe('shortcuts', () => {
  afterEach(() => {
    document.body.innerHTML = '';
  });

  it('handles navigation, sort, refresh, search focus, and Escape when outside inputs', () => {
    const handlers = makeHandlers();
    const onKeydown = createShortcutHandler(handlers);

    dispatchKey(onKeydown, 'ArrowLeft');
    dispatchKey(onKeydown, 'ArrowRight');
    dispatchKey(onKeydown, 'd');
    dispatchKey(onKeydown, 't');
    dispatchKey(onKeydown, 'c');
    dispatchKey(onKeydown, 'r');
    const slashEvent = dispatchKey(onKeydown, '/');
    dispatchKey(onKeydown, 'Escape');

    expect(handlers.onPrevTab).toHaveBeenCalledTimes(1);
    expect(handlers.onNextTab).toHaveBeenCalledTimes(1);
    expect(handlers.onSort).toHaveBeenNthCalledWith(1, 'date');
    expect(handlers.onSort).toHaveBeenNthCalledWith(2, 'tokens');
    expect(handlers.onSort).toHaveBeenNthCalledWith(3, 'cost');
    expect(handlers.onRefresh).toHaveBeenCalledTimes(1);
    expect(handlers.onFocusSearch).toHaveBeenCalledTimes(1);
    expect(slashEvent.defaultPrevented).toBe(true);
    expect(handlers.onClearSearch).toHaveBeenCalledTimes(1);
  });

  it('ignores shortcuts inside input, textarea, and select, except Escape', () => {
    const handlers = makeHandlers();
    const onKeydown = createShortcutHandler(handlers);

    for (const tag of ['input', 'textarea', 'select'] as const) {
      const el = document.createElement(tag);
      document.body.appendChild(el);

      dispatchKey(onKeydown, 'ArrowLeft', el);
      dispatchKey(onKeydown, 'ArrowRight', el);
      dispatchKey(onKeydown, 'd', el);
      dispatchKey(onKeydown, 't', el);
      dispatchKey(onKeydown, 'c', el);
      dispatchKey(onKeydown, 'r', el);
      dispatchKey(onKeydown, '/', el);

      expect(handlers.onPrevTab).not.toHaveBeenCalled();
      expect(handlers.onNextTab).not.toHaveBeenCalled();
      expect(handlers.onSort).not.toHaveBeenCalled();
      expect(handlers.onRefresh).not.toHaveBeenCalled();
      expect(handlers.onFocusSearch).not.toHaveBeenCalled();

      dispatchKey(onKeydown, 'Escape', el);
    }

    expect(handlers.onClearSearch).toHaveBeenCalledTimes(3);
  });

  it('ignores all shortcuts including ArrowLeft, ArrowRight, and Escape when a modal is open', () => {
    const modal = document.createElement('div');
    modal.setAttribute('role', 'dialog');
    modal.setAttribute('aria-modal', 'true');
    document.body.appendChild(modal);

    const handlers = makeHandlers();
    const onKeydown = createShortcutHandler(handlers);

    for (const key of ['ArrowLeft', 'ArrowRight', 'd', 't', 'c', 'r', '/', 'Escape']) {
      const ev = dispatchKey(onKeydown, key);
      expect(ev.defaultPrevented).toBe(false);
    }

    expect(handlers.onPrevTab).not.toHaveBeenCalled();
    expect(handlers.onNextTab).not.toHaveBeenCalled();
    expect(handlers.onSort).not.toHaveBeenCalled();
    expect(handlers.onRefresh).not.toHaveBeenCalled();
    expect(handlers.onFocusSearch).not.toHaveBeenCalled();
    expect(handlers.onClearSearch).not.toHaveBeenCalled();
  });

  it('ignores shortcuts with meta/ctrl/alt modifiers and arrow keys inside tablist', () => {
    const handlers = makeHandlers();
    const onKeydown = createShortcutHandler(handlers);

    dispatchKey(onKeydown, 'r', document.body, { metaKey: true });
    dispatchKey(onKeydown, 'c', document.body, { ctrlKey: true });
    dispatchKey(onKeydown, 'ArrowLeft', document.body, { altKey: true });

    const tablist = document.createElement('div');
    tablist.setAttribute('role', 'tablist');
    const tabBtn = document.createElement('button');
    tablist.appendChild(tabBtn);
    document.body.appendChild(tablist);

    dispatchKey(onKeydown, 'ArrowLeft', tabBtn);
    dispatchKey(onKeydown, 'ArrowRight', tabBtn);

    expect(handlers.onRefresh).not.toHaveBeenCalled();
    expect(handlers.onSort).not.toHaveBeenCalled();
    expect(handlers.onPrevTab).not.toHaveBeenCalled();
    expect(handlers.onNextTab).not.toHaveBeenCalled();
  });

  it('triggers onOpenSettings on Cmd+, or Ctrl+,', () => {
    const onOpenSettings = vi.fn();
    const handlers: ShortcutHandlers = { ...makeHandlers(), onOpenSettings };
    const onKeydown = createShortcutHandler(handlers);

    const macEvent = dispatchKey(onKeydown, ',', document.body, { metaKey: true });
    expect(macEvent.defaultPrevented).toBe(true);
    expect(onOpenSettings).toHaveBeenCalledTimes(1);

    const winEvent = dispatchKey(onKeydown, ',', document.body, { ctrlKey: true });
    expect(winEvent.defaultPrevented).toBe(true);
    expect(onOpenSettings).toHaveBeenCalledTimes(2);
  });
});

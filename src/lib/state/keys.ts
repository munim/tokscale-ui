export interface ShortcutBinding {
  key: string;
  ctrl?: boolean;
  meta?: boolean;
  alt?: boolean;
  shift?: boolean;
  allowInInput?: boolean;
  preventDefault?: boolean;
  handler: (event: KeyboardEvent) => void;
}

export function isTextInputTarget(target: EventTarget | null): boolean {
  if (!target || typeof target !== 'object') {
    return false;
  }
  const el = target as {
    tagName?: string;
    isContentEditable?: boolean;
    getAttribute?: (name: string) => string | null;
    closest?: (selector: string) => unknown;
  };
  const tag = el.tagName?.toUpperCase();
  if (tag === 'INPUT' || tag === 'TEXTAREA' || tag === 'SELECT') {
    return true;
  }
  if (el.isContentEditable) {
    return true;
  }
  const ceAttr = typeof el.getAttribute === 'function' ? el.getAttribute('contenteditable') : null;
  if (ceAttr === 'true' || ceAttr === '') {
    return true;
  }
  if (typeof el.closest === 'function' && el.closest('[contenteditable="true"]')) {
    return true;
  }
  return false;
}

export function matchesShortcut(event: KeyboardEvent, binding: ShortcutBinding): boolean {
  const keyMatch =
    event.key === binding.key ||
    (binding.key.length === 1 && event.key.toLowerCase() === binding.key.toLowerCase());
  if (!keyMatch) {
    return false;
  }
  if (Boolean(binding.ctrl) !== event.ctrlKey) return false;
  if (Boolean(binding.meta) !== event.metaKey) return false;
  if (Boolean(binding.alt) !== event.altKey) return false;
  if (binding.shift !== undefined && binding.shift !== event.shiftKey) return false;
  return true;
}

export function createKeyHandler(
  bindings: ShortcutBinding[],
  options: { ignoreInputs?: boolean } = {},
): (event: KeyboardEvent) => void {
  const ignoreInputs = options.ignoreInputs ?? true;
  return (event: KeyboardEvent) => {
    const inInput = ignoreInputs && isTextInputTarget(event.target);
    for (const binding of bindings) {
      if (inInput && !binding.allowInInput) {
        continue;
      }
      if (matchesShortcut(event, binding)) {
        if (binding.preventDefault !== false) {
          event.preventDefault();
        }
        binding.handler(event);
        return;
      }
    }
  };
}

export function registerShortcuts(
  target: Pick<Window, 'addEventListener' | 'removeEventListener'>,
  bindings: ShortcutBinding[],
  options: { ignoreInputs?: boolean } = {},
): () => void {
  const listener = createKeyHandler(bindings, options) as EventListener;
  target.addEventListener('keydown', listener);
  return () => {
    target.removeEventListener('keydown', listener);
  };
}

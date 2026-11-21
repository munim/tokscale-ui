export interface ShortcutHandlers {
  onPrevTab: () => void;
  onNextTab: () => void;
  onSort: (sort: 'date' | 'tokens' | 'cost') => void;
  onRefresh: () => void;
  onFocusSearch: () => void;
  onClearSearch: () => void;
}

function isEditableTarget(target: EventTarget | null): boolean {
  if (!(target instanceof HTMLElement)) {
    return false;
  }
  const tag = target.tagName;
  if (tag === 'INPUT' || tag === 'TEXTAREA' || tag === 'SELECT') {
    return true;
  }
  return target.isContentEditable;
}

function isModalOpen(): boolean {
  if (typeof document === 'undefined') {
    return false;
  }
  return document.querySelector('[aria-modal="true"]') !== null;
}

function isArrowManagedWidget(target: EventTarget | null): boolean {
  if (!(target instanceof HTMLElement)) {
    return false;
  }
  return Boolean(
    target.closest('[role="tablist"], [role="radiogroup"], [role="slider"], .ui-chart-bar'),
  );
}

export function createShortcutHandler(handlers: ShortcutHandlers): (event: KeyboardEvent) => void {
  return (event: KeyboardEvent) => {
    if (isModalOpen()) {
      return;
    }

    if (event.metaKey || event.ctrlKey || event.altKey) {
      return;
    }

    if (event.key === 'Escape') {
      handlers.onClearSearch();
      return;
    }

    if (isEditableTarget(event.target)) {
      return;
    }

    if (event.defaultPrevented) {
      return;
    }

    if (event.key === 'ArrowLeft') {
      if (isArrowManagedWidget(event.target)) {
        return;
      }
      event.preventDefault();
      handlers.onPrevTab();
      return;
    }

    if (event.key === 'ArrowRight') {
      if (isArrowManagedWidget(event.target)) {
        return;
      }
      event.preventDefault();
      handlers.onNextTab();
      return;
    }

    if (event.key === '/') {
      event.preventDefault();
      handlers.onFocusSearch();
      return;
    }

    const lower = event.key.toLowerCase();
    if (lower === 'd') {
      handlers.onSort('date');
    } else if (lower === 't') {
      handlers.onSort('tokens');
    } else if (lower === 'c') {
      handlers.onSort('cost');
    } else if (lower === 'r') {
      handlers.onRefresh();
    }
  };
}

export function installDashboardShortcuts(
  handlers: ShortcutHandlers,
  target: Pick<Window, 'addEventListener' | 'removeEventListener'> | undefined = typeof window !== 'undefined'
    ? window
    : undefined,
): () => void {
  if (!target) {
    return () => {};
  }
  const listener = createShortcutHandler(handlers);
  target.addEventListener('keydown', listener as EventListener);
  return () => {
    target.removeEventListener('keydown', listener as EventListener);
  };
}

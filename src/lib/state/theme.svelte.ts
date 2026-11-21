export type ThemeMode = 'light' | 'dark' | 'system';
export type ResolvedTheme = 'light' | 'dark';

export const theme = $state<{ mode: ThemeMode; resolved: ResolvedTheme }>({
  mode: 'system',
  resolved: 'light',
});

let activeMediaQuery: MediaQueryList | null = null;
let activeListener: ((event: MediaQueryListEvent) => void) | null = null;

function detachSystemListener(): void {
  if (activeMediaQuery && activeListener) {
    if (typeof activeMediaQuery.removeEventListener === 'function') {
      activeMediaQuery.removeEventListener('change', activeListener);
    } else if (typeof activeMediaQuery.removeListener === 'function') {
      activeMediaQuery.removeListener(activeListener);
    }
  }
  activeMediaQuery = null;
  activeListener = null;
}

function writeDomTheme(resolved: ResolvedTheme): void {
  if (typeof document === 'undefined') {
    return;
  }
  const root = document.documentElement;
  if (resolved === 'dark') {
    root.dataset.theme = 'dark';
    root.style.colorScheme = 'dark';
  } else {
    delete root.dataset.theme;
    root.style.colorScheme = 'light';
  }
}

function resolveSystemTheme(): ResolvedTheme {
  if (typeof window === 'undefined' || typeof window.matchMedia !== 'function') {
    return 'light';
  }
  return window.matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'light';
}

export function applyTheme(mode: ThemeMode): void {
  detachSystemListener();
  theme.mode = mode;

  if (mode === 'system') {
    const resolved = resolveSystemTheme();
    theme.resolved = resolved;
    writeDomTheme(resolved);

    if (typeof window !== 'undefined' && typeof window.matchMedia === 'function') {
      const mq = window.matchMedia('(prefers-color-scheme: dark)');
      const onChange = (event: MediaQueryListEvent) => {
        if (theme.mode !== 'system') {
          return;
        }
        const next: ResolvedTheme = event.matches ? 'dark' : 'light';
        theme.resolved = next;
        writeDomTheme(next);
      };
      if (typeof mq.addEventListener === 'function') {
        mq.addEventListener('change', onChange);
      } else if (typeof mq.addListener === 'function') {
        mq.addListener(onChange);
      }
      activeMediaQuery = mq;
      activeListener = onChange;
    }
    return;
  }

  theme.resolved = mode;
  writeDomTheme(mode);
}

export function nextThemeMode(current: ThemeMode): ThemeMode {
  if (current === 'light') return 'dark';
  if (current === 'dark') return 'system';
  return 'light';
}

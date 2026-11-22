import {
  ipc,
  onDashboardNavigate,
  onPreferencesUpdated,
  onSnapshotUpdated,
  onSyncProgress,
  type CompanionPreferences,
  type DashboardNavigateEvent,
  type DashboardQuery,
  type DashboardSnapshot,
  type DashboardTab,
  type RangePreset,
  type SnapshotEvent,
  type SyncMode,
  type SyncProgressEvent,
  type SyncStatus,
  type UnlistenFn,
} from '$lib/contracts/ipc';
import { addDays, localDateKey } from '$lib/format/date';
import { applyTheme, type ThemeMode } from '$lib/state/theme.svelte';

export interface DashboardTabMeta {
  id: DashboardTab;
  label: string;
}

export const DASHBOARD_TABS: readonly DashboardTabMeta[] = [
  { id: 'overview', label: 'Overview' },
  { id: 'usage', label: 'Usage' },
  { id: 'models', label: 'Models' },
  { id: 'daily', label: 'Daily' },
  { id: 'hourly', label: 'Hourly' },
  { id: 'monthly', label: 'Monthly' },
  { id: 'sessions', label: 'Sessions' },
  { id: 'projects', label: 'Projects' },
  { id: 'stats', label: 'Stats' },
  { id: 'agents', label: 'Agents' },
];

const SEARCH_DEBOUNCE_MS = 120;

export interface DashboardRouterDeps {
  getDashboardSnapshot: (query: DashboardQuery) => Promise<DashboardSnapshot>;
  triggerSync: (mode: SyncMode) => Promise<SyncStatus>;
  getPreferences: () => Promise<CompanionPreferences>;
  updatePreferences: (patch: Partial<CompanionPreferences>) => Promise<CompanionPreferences>;
  onDashboardNavigate: (handler: (payload: DashboardNavigateEvent) => void) => Promise<UnlistenFn>;
  onSnapshotUpdated: (handler: (payload: SnapshotEvent) => void) => Promise<UnlistenFn>;
  onSyncProgress: (handler: (payload: SyncProgressEvent) => void) => Promise<UnlistenFn>;
  onPreferencesUpdated: (handler: (payload: CompanionPreferences) => void) => Promise<UnlistenFn>;
  applyTheme: (mode: ThemeMode) => void;
}

const defaultDeps: DashboardRouterDeps = {
  getDashboardSnapshot: (query) => ipc.getDashboardSnapshot(query),
  triggerSync: (mode) => ipc.triggerSync(mode),
  getPreferences: () => ipc.getPreferences(),
  updatePreferences: (patch) => ipc.updatePreferences(patch),
  onDashboardNavigate,
  onSnapshotUpdated,
  onSyncProgress,
  onPreferencesUpdated,
  applyTheme,
};

function cloneQuery(query: DashboardQuery): DashboardQuery {
  return {
    range: query.range,
    customSince: query.customSince,
    customUntil: query.customUntil,
    groupBy: query.groupBy,
    activeClients: [...query.activeClients],
    search: query.search,
    sort: query.sort,
  };
}

function normalizeThemeMode(raw: string | undefined): ThemeMode {
  if (raw === 'light' || raw === 'dark' || raw === 'system') {
    return raw;
  }
  return 'system';
}

export class DashboardRouter {
  activeTab = $state<DashboardTab>('overview');
  query = $state<DashboardQuery>({
    range: '30d',
    groupBy: 'model',
    activeClients: [],
    search: '',
    sort: 'cost',
  });
  snapshot = $state<DashboardSnapshot | null>(null);
  loading = $state(false);
  syncing = $state(false);
  syncMessage = $state('');
  lastSyncedAt = $state<number | undefined>(undefined);
  error = $state<string | null>(null);

  private readonly deps: DashboardRouterDeps;
  private searchTimer: ReturnType<typeof setTimeout> | null = null;
  private requestSeq = 0;
  private pendingHandoffConsumed = false;
  private destroyed = false;
  private syncInFlight = false;

  constructor(deps: Partial<DashboardRouterDeps> = {}) {
    this.deps = { ...defaultDeps, ...deps };
  }

  get customRangeError(): string | null {
    if (this.query.range !== 'custom') {
      return null;
    }
    const { customSince, customUntil } = this.query;
    if (customSince && customUntil && customSince > customUntil) {
      return 'Start date must be on or before end date';
    }
    return null;
  }

  setTab(tab: DashboardTab): void {
    this.activeTab = tab;
  }

  stepTab(delta: 1 | -1): void {
    const idx = DASHBOARD_TABS.findIndex((t) => t.id === this.activeTab);
    const base = idx >= 0 ? idx : 0;
    const nextIdx = (base + delta + DASHBOARD_TABS.length) % DASHBOARD_TABS.length;
    this.activeTab = DASHBOARD_TABS[nextIdx].id;
  }

  setRange(range: RangePreset): void {
    this.query.range = range;
    if (range === 'custom' && !this.query.customSince && !this.query.customUntil) {
      const today = localDateKey(new Date());
      this.query.customSince = addDays(today, -6);
      this.query.customUntil = today;
    }
    if (!this.customRangeError) {
      void this.fetchSnapshot();
    }
  }

  setCustomDates(since: string, until: string): void {
    this.query.customSince = since || undefined;
    this.query.customUntil = until || undefined;
    if (this.query.range === 'custom' && !this.customRangeError) {
      void this.fetchSnapshot();
    }
  }

  setGroupBy(groupBy: string): void {
    this.query.groupBy = groupBy;
    void this.fetchSnapshot();
  }

  setSort(sort: 'cost' | 'tokens' | 'date'): void {
    this.query.sort = sort;
    void this.fetchSnapshot();
  }

  toggleClient(clientId: string): void {
    const current = this.query.activeClients;
    if (current.includes(clientId)) {
      this.query.activeClients = current.filter((c) => c !== clientId);
    } else {
      this.query.activeClients = [...current, clientId];
    }
    void this.fetchSnapshot();
  }

  clearClients(): void {
    if (this.query.activeClients.length === 0) {
      return;
    }
    this.query.activeClients = [];
    void this.fetchSnapshot();
  }

  setSearch(value: string, immediate = false): void {
    this.query.search = value;
    this.clearSearchTimer();
    if (immediate) {
      void this.fetchSnapshot();
      return;
    }
    this.searchTimer = setTimeout(() => {
      this.searchTimer = null;
      void this.fetchSnapshot();
    }, SEARCH_DEBOUNCE_MS);
  }

  clearSearch(): void {
    if (!this.query.search) {
      return;
    }
    this.setSearch('', true);
  }

  applyHandoff(handoff: DashboardNavigateEvent): void {
    this.clearSearchTimer();
    const isKnownTab = DASHBOARD_TABS.some((t) => t.id === handoff.tab);
    if (isKnownTab) {
      this.activeTab = handoff.tab;
    }
    let queryChanged = false;
    if (handoff.range && handoff.range !== this.query.range) {
      this.query.range = handoff.range;
      queryChanged = true;
    }
    if (handoff.query !== undefined && handoff.query !== (this.query.search ?? '')) {
      this.query.search = handoff.query;
      queryChanged = true;
    }
    if (!this.customRangeError && (queryChanged || !this.snapshot)) {
      void this.fetchSnapshot();
    } else if (!this.customRangeError) {
      void this.fetchSnapshot();
    }
  }

  async fetchSnapshot(silent = false): Promise<void> {
    if (this.destroyed || this.customRangeError) {
      return;
    }
    const seq = ++this.requestSeq;
    if (!silent) {
      this.loading = true;
    }
    this.error = null;

    try {
      const payload = await this.deps.getDashboardSnapshot(cloneQuery(this.query));
      if (this.destroyed || seq !== this.requestSeq) {
        return;
      }

      if (!this.pendingHandoffConsumed) {
        this.pendingHandoffConsumed = true;
        if (payload.pendingHandoff) {
          const handoff = payload.pendingHandoff;
          const isKnownTab = DASHBOARD_TABS.some((t) => t.id === handoff.tab);
          if (isKnownTab) {
            this.activeTab = handoff.tab;
          }
          let needsRefetch = false;
          if (handoff.range && handoff.range !== this.query.range) {
            this.query.range = handoff.range;
            needsRefetch = true;
          }
          if (handoff.query !== undefined && handoff.query !== (this.query.search ?? '')) {
            this.query.search = handoff.query;
            needsRefetch = true;
          }
          this.snapshot = payload;
          this.lastSyncedAt = payload.syncStatus.lastSyncedAt;
          this.syncing = this.syncInFlight || payload.syncStatus.syncing;
          if (needsRefetch) {
            await this.fetchSnapshot(silent);
          }
          return;
        }
      }

      this.snapshot = payload;
      this.lastSyncedAt = payload.syncStatus.lastSyncedAt;
      this.syncing = this.syncInFlight || payload.syncStatus.syncing;
    } catch (err) {
      if (this.destroyed || seq !== this.requestSeq) {
        return;
      }
      this.error = err instanceof Error ? err.message : String(err);
    } finally {
      if (!this.destroyed && seq === this.requestSeq && !silent) {
        this.loading = false;
      }
    }
  }

  async refresh(): Promise<void> {
    if (this.destroyed || this.syncInFlight) {
      return;
    }
    this.syncInFlight = true;
    this.syncing = true;
    this.syncMessage = 'Syncing…';
    try {
      const status = await this.deps.triggerSync('local_offline');
      if (this.destroyed) {
        return;
      }
      if (status.lastSyncedAt !== undefined) {
        this.lastSyncedAt = status.lastSyncedAt;
      }
      await this.fetchSnapshot(true);
    } catch (err) {
      if (!this.destroyed) {
        this.error = err instanceof Error ? err.message : String(err);
      }
    } finally {
      this.syncInFlight = false;
      if (!this.destroyed) {
        this.syncing = this.snapshot?.syncStatus.syncing ?? false;
        if (!this.syncing) {
          this.syncMessage = '';
        }
      }
    }
  }

  async setThemeMode(mode: ThemeMode): Promise<void> {
    this.deps.applyTheme(mode);
    try {
      await this.deps.updatePreferences({ theme: mode });
    } catch {
      // Theme is already applied locally; ignore transient preference write failure.
    }
  }

  start(): () => void {
    this.destroyed = false;
    const unlisteners: UnlistenFn[] = [];
    let disposed = false;

    const register = (promise: Promise<UnlistenFn>) => {
      void promise.then((unlisten) => {
        if (disposed) {
          unlisten();
        } else {
          unlisteners.push(unlisten);
        }
      });
    };

    register(
      this.deps.onDashboardNavigate((handoff) => {
        this.applyHandoff(handoff);
      }),
    );

    register(
      this.deps.onSnapshotUpdated(() => {
        void this.fetchSnapshot(true);
      }),
    );

    register(
      this.deps.onSyncProgress((progress) => {
        this.syncing = progress.syncing;
        this.syncMessage = progress.syncing ? progress.message : '';
      }),
    );

    register(
      this.deps.onPreferencesUpdated((prefs) => {
        this.deps.applyTheme(normalizeThemeMode(prefs.theme));
      }),
    );

    void this.deps.getPreferences().then((prefs) => {
      if (!disposed) {
        this.deps.applyTheme(normalizeThemeMode(prefs.theme));
      }
    });

    void this.fetchSnapshot();

    return () => {
      disposed = true;
      this.destroyed = true;
      this.clearSearchTimer();
      for (const unlisten of unlisteners) {
        unlisten();
      }
      unlisteners.length = 0;
    };
  }

  private clearSearchTimer(): void {
    if (this.searchTimer !== null) {
      clearTimeout(this.searchTimer);
      this.searchTimer = null;
    }
  }
}

export function createDashboardRouter(deps?: Partial<DashboardRouterDeps>): DashboardRouter {
  return new DashboardRouter(deps);
}

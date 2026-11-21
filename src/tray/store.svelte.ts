import {
  ipc,
  onPreferencesUpdated,
  onSnapshotUpdated,
  onSyncProgress,
  type CliInstallResult,
  type CliResolutionInfo,
  type CompanionPreferences,
  type SnapshotEvent,
  type SourceWarning,
  type SyncMode,
  type SyncProgressEvent,
  type TraySnapshot,
  type UnlistenFn,
} from '$lib/contracts/ipc';
import { applyTheme } from '$lib/state';

export type TrayRange = 'today' | '7d' | '30d' | 'all';

export const TRAY_RANGES: readonly TrayRange[] = ['today', '7d', '30d', 'all'];

const DEFAULT_PREFERENCES: CompanionPreferences = {
  menubarDisplay: 'cost',
  theme: 'system',
  launchAtLogin: false,
  notificationsEnabled: false,
  quotaAlertPercent: 85,
  sessionTitleEnrichment: true,
};

export function createTrayStore() {
  let activeRange = $state<TrayRange>('today');
  let snapshots = $state<Partial<Record<TrayRange, TraySnapshot>>>({});
  let loadingRange = $state<TrayRange | null>(null);
  let error = $state<string | null>(null);

  let preferences = $state<CompanionPreferences>({ ...DEFAULT_PREFERENCES });
  let cliInfo = $state<CliResolutionInfo | null>(null);
  let cliInstalling = $state(false);
  let cliInstallResult = $state<CliInstallResult | null>(null);

  let settingsOpen = $state(false);
  let syncProgress = $state<SyncProgressEvent | null>(null);
  let manualSyncing = $state(false);

  let eventPartial = $state(false);
  let eventWarnings = $state<SourceWarning[]>([]);

  const inFlightRanges = new Map<TrayRange, Promise<TraySnapshot | null>>();

  let currentSnapshot = $derived<TraySnapshot | null>(snapshots[activeRange] ?? null);

  let isSyncing = $derived(
    manualSyncing || Boolean(syncProgress?.syncing) || Boolean(currentSnapshot?.syncStatus.syncing),
  );

  let warnings = $derived.by((): SourceWarning[] => {
    if (eventWarnings.length > 0) {
      return eventWarnings;
    }
    return currentSnapshot?.syncStatus.warnings ?? [];
  });

  let isPartial = $derived(eventPartial || warnings.length > 0);

  let isEmpty = $derived.by(() => {
    const snap = currentSnapshot;
    if (!snap) {
      return false;
    }
    return snap.total.cost <= 0 && snap.total.tokens <= 0 && snap.total.sessions <= 0;
  });

  async function fetchRange(range: TrayRange, force = false): Promise<TraySnapshot | null> {
    if (!force && snapshots[range]) {
      return snapshots[range]!;
    }

    const existing = inFlightRanges.get(range);
    if (existing && !force) {
      return existing;
    }

    loadingRange = range;
    error = null;

    const promise = ipc
      .getTraySnapshot(range)
      .then((data) => {
        snapshots = { ...snapshots, [range]: data };
        if (!cliInfo && data.cliInfo) {
          cliInfo = data.cliInfo;
        }
        return data;
      })
      .catch((err) => {
        error = err instanceof Error ? err.message : 'Failed to load usage snapshot';
        return null;
      })
      .finally(() => {
        inFlightRanges.delete(range);
        if (loadingRange === range) {
          loadingRange = null;
        }
      });

    inFlightRanges.set(range, promise);
    return promise;
  }

  function setRange(range: TrayRange): void {
    activeRange = range;
    if (snapshots[range]) {
      return;
    }
    void fetchRange(range);
  }

  async function triggerSync(mode: SyncMode = 'full_manual'): Promise<void> {
    manualSyncing = true;
    error = null;
    try {
      const status = await ipc.triggerSync(mode);
      eventPartial = status.warnings.length > 0;
      eventWarnings = status.warnings;

      const cachedRanges = Object.keys(snapshots) as TrayRange[];
      const nextCache: Partial<Record<TrayRange, TraySnapshot>> = {};
      if (snapshots[activeRange]) {
        nextCache[activeRange] = {
          ...snapshots[activeRange]!,
          syncStatus: status,
        };
      }
      snapshots = nextCache;

      await fetchRange(activeRange, true);
      if (cachedRanges.length > 1) {
        // Other cached ranges were invalidated above and will fetch fresh data on demand.
      }
    } catch (err) {
      error = err instanceof Error ? err.message : 'Sync failed';
    } finally {
      manualSyncing = false;
    }
  }

  async function updatePreferences(patch: Partial<CompanionPreferences>): Promise<void> {
    if (patch.theme) {
      applyTheme(patch.theme);
    }
    preferences = { ...preferences, ...patch };
    try {
      const saved = await ipc.updatePreferences(patch);
      preferences = saved;
      if (saved.theme) {
        applyTheme(saved.theme);
      }
    } catch (err) {
      error = err instanceof Error ? err.message : 'Failed to save preferences';
    }
  }

  async function installOrUpgradeCli(): Promise<void> {
    if (cliInstalling) {
      return;
    }
    cliInstalling = true;
    cliInstallResult = null;
    try {
      const result = await ipc.installOrUpgradeCli();
      cliInstallResult = result;
      if (result.ok) {
        try {
          cliInfo = await ipc.getCliInfo();
        } catch {
          // Keep previous CLI info if refresh fails.
        }
      }
    } catch (err) {
      cliInstallResult = {
        ok: false,
        message: err instanceof Error ? err.message : 'CLI installation failed',
      };
    } finally {
      cliInstalling = false;
    }
  }

  function toggleSettings(): void {
    settingsOpen = !settingsOpen;
  }

  function openModelInDashboard(model: string): void {
    void ipc.openDashboard({
      tab: 'models',
      range: activeRange,
      query: model,
    });
  }

  function openDashboard(): void {
    void ipc.openDashboard({
      tab: 'overview',
      range: activeRange,
    });
  }

  function dismiss(): void {
    void ipc.dismissTray();
  }

  function quit(): void {
    void ipc.quitApp();
  }

  function handleSnapshotUpdated(payload: SnapshotEvent): void {
    eventPartial = payload.partial || payload.warnings.length > 0;
    eventWarnings = payload.warnings;

    // Keep the currently visible snapshot rendered while refreshing, and invalidate other cached ranges.
    const current = snapshots[activeRange];
    snapshots = current ? { [activeRange]: current } : {};
    void fetchRange(activeRange, true);
  }

  function handleSyncProgress(payload: SyncProgressEvent): void {
    syncProgress = payload.syncing ? payload : null;
  }

  function handlePreferencesUpdated(payload: CompanionPreferences): void {
    preferences = payload;
    applyTheme(payload.theme);
  }

  function init(): () => void {
    let disposed = false;
    const unlisteners: UnlistenFn[] = [];

    const register = (promise: Promise<UnlistenFn>) => {
      void promise.then((unlisten) => {
        if (disposed) {
          unlisten();
        } else {
          unlisteners.push(unlisten);
        }
      });
    };

    register(onSnapshotUpdated(handleSnapshotUpdated));
    register(onSyncProgress(handleSyncProgress));
    register(onPreferencesUpdated(handlePreferencesUpdated));

    void fetchRange(activeRange);
    void ipc
      .getPreferences()
      .then((prefs) => {
        if (disposed) return;
        preferences = prefs;
        applyTheme(prefs.theme);
      })
      .catch(() => {});

    void ipc
      .getCliInfo()
      .then((info) => {
        if (disposed) return;
        cliInfo = info;
      })
      .catch(() => {});

    return () => {
      disposed = true;
      for (const unlisten of unlisteners) {
        unlisten();
      }
      unlisteners.length = 0;
    };
  }

  return {
    get activeRange() {
      return activeRange;
    },
    get snapshots() {
      return snapshots;
    },
    get snapshot() {
      return currentSnapshot;
    },
    get loading() {
      return loadingRange === activeRange && !currentSnapshot;
    },
    get error() {
      return error;
    },
    get preferences() {
      return preferences;
    },
    get cliInfo() {
      return cliInfo;
    },
    get cliInstalling() {
      return cliInstalling;
    },
    get cliInstallResult() {
      return cliInstallResult;
    },
    get settingsOpen() {
      return settingsOpen;
    },
    get syncProgress() {
      return syncProgress;
    },
    get isSyncing() {
      return isSyncing;
    },
    get warnings() {
      return warnings;
    },
    get isPartial() {
      return isPartial;
    },
    get isEmpty() {
      return isEmpty;
    },
    init,
    setRange,
    fetchRange,
    triggerSync,
    updatePreferences,
    installOrUpgradeCli,
    toggleSettings,
    openModelInDashboard,
    openDashboard,
    dismiss,
    quit,
  };
}

export type TrayStore = ReturnType<typeof createTrayStore>;

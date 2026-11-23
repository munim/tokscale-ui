import {
  ipc,
  onPreferencesUpdated,
  onSnapshotUpdated,
  onSyncProgress,
  onUpdateStatus,
  type CompanionPreferences,
  type SettingsSection,
  type SnapshotEvent,
  type SourceWarning,
  type SyncMode,
  type SyncProgressEvent,
  type TraySnapshot,
  type UnlistenFn,
  type UpdateStatusPayload,
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
  autoCheckUpdates: true,
};

export function createTrayStore() {
  let activeRange = $state<TrayRange>('today');
  let snapshots = $state<Partial<Record<TrayRange, TraySnapshot>>>({});
  let loadingRange = $state<TrayRange | null>(null);
  let error = $state<string | null>(null);

  let preferences = $state<CompanionPreferences>({ ...DEFAULT_PREFERENCES });
  let syncProgress = $state<SyncProgressEvent | null>(null);
  let manualSyncing = $state(false);
  let updateStatus = $state<UpdateStatusPayload | null>(null);
  let dismissedUpdateVersion = $state<string | null>(null);

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

  let visibleUpdate = $derived.by((): UpdateStatusPayload | null => {
    const status = updateStatus;
    if (!status) return null;
    if (status.phase !== 'available' && status.phase !== 'downloading' && status.phase !== 'ready') {
      return null;
    }
    if (status.phase === 'available') {
      if (status.availableVersion && status.availableVersion === preferences.skippedUpdateVersion) {
        return null;
      }
      if (status.availableVersion && status.availableVersion === dismissedUpdateVersion) {
        return null;
      }
    }
    return status;
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

      const nextCache: Partial<Record<TrayRange, TraySnapshot>> = {};
      if (snapshots[activeRange]) {
        nextCache[activeRange] = {
          ...snapshots[activeRange]!,
          syncStatus: status,
        };
      }
      snapshots = nextCache;

      await fetchRange(activeRange, true);
    } catch (err) {
      error = err instanceof Error ? err.message : 'Sync failed';
    } finally {
      manualSyncing = false;
    }
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

  function openSettings(section?: SettingsSection): void {
    void ipc.openSettings(section);
  }

  function dismiss(): void {
    void ipc.dismissTray();
  }

  function quit(): void {
    void ipc.quitApp();
  }

  function dismissUpdateBanner(): void {
    if (updateStatus?.availableVersion) {
      dismissedUpdateVersion = updateStatus.availableVersion;
    }
  }

  async function downloadAndInstallUpdate(): Promise<void> {
    try {
      const next = await ipc.downloadAndInstallUpdate();
      updateStatus = next;
    } catch (err) {
      error = err instanceof Error ? err.message : 'Update failed';
    }
  }

  async function restartToApplyUpdate(): Promise<void> {
    try {
      await ipc.restartToApplyUpdate();
    } catch (err) {
      error = err instanceof Error ? err.message : 'Restart failed';
    }
  }

  function handleSnapshotUpdated(payload: SnapshotEvent): void {
    eventPartial = payload.partial || payload.warnings.length > 0;
    eventWarnings = payload.warnings;

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
    register(
      onUpdateStatus((payload) => {
        updateStatus = payload;
        if (payload.phase === 'downloading' || payload.phase === 'ready') {
          dismissedUpdateVersion = null;
        }
      }),
    );

    void fetchRange(activeRange);
    void ipc
      .getUpdateStatus()
      .then((status) => {
        if (!disposed && status) {
          updateStatus = status;
        }
      })
      .catch(() => {});
    void ipc
      .getPreferences()
      .then((prefs) => {
        if (disposed) return;
        preferences = prefs;
        applyTheme(prefs.theme);
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
    get visibleUpdate() {
      return visibleUpdate;
    },
    init,
    setRange,
    fetchRange,
    triggerSync,
    openModelInDashboard,
    openDashboard,
    openSettings,
    dismiss,
    quit,
    dismissUpdateBanner,
    downloadAndInstallUpdate,
    restartToApplyUpdate,
  };
}

export type TrayStore = ReturnType<typeof createTrayStore>;

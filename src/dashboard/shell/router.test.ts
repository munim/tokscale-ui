import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import type {
  CompanionPreferences,
  DashboardNavigateEvent,
  DashboardQuery,
  DashboardSnapshot,
  SnapshotEvent,
  SyncProgressEvent,
} from '$lib/contracts/ipc';
import { mockDashboardSnapshot, mockPreferences } from '$lib/contracts/mocks';
import { createDashboardRouter } from './router.svelte';

async function flushPromises(): Promise<void> {
  await Promise.resolve();
  await Promise.resolve();
}

describe('DashboardRouter', () => {
  beforeEach(() => {
    vi.useFakeTimers();
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it('syncs tab, range, and query from tokscale://dashboard-navigate and refetches', async () => {
    let navHandler: ((payload: DashboardNavigateEvent) => void) | undefined;
    let snapshotHandler: ((payload: SnapshotEvent) => void) | undefined;
    let progressHandler: ((payload: SyncProgressEvent) => void) | undefined;
    let prefsHandler: ((payload: CompanionPreferences) => void) | undefined;

    const getDashboardSnapshot = vi.fn(async (q: DashboardQuery): Promise<DashboardSnapshot> =>
      mockDashboardSnapshot(q),
    );
    const applyTheme = vi.fn();

    const router = createDashboardRouter({
      getDashboardSnapshot,
      getPreferences: vi.fn(async () => mockPreferences()),
      updatePreferences: vi.fn(async (patch) => ({ ...mockPreferences(), ...patch })),
      triggerSync: vi.fn(async () => ({ syncing: false, lastSyncedAt: 1795262400000, warnings: [] })),
      onDashboardNavigate: async (h) => {
        navHandler = h;
        return () => {};
      },
      onSnapshotUpdated: async (h) => {
        snapshotHandler = h;
        return () => {};
      },
      onSyncProgress: async (h) => {
        progressHandler = h;
        return () => {};
      },
      onPreferencesUpdated: async (h) => {
        prefsHandler = h;
        return () => {};
      },
      applyTheme,
    });

    const stop = router.start();
    await flushPromises();

    expect(getDashboardSnapshot).toHaveBeenCalledTimes(1);
    expect(router.activeTab).toBe('overview');

    // Switching tabs directly does not re-fetch
    router.setTab('models');
    expect(router.activeTab).toBe('models');
    expect(getDashboardSnapshot).toHaveBeenCalledTimes(1);

    // Navigate event updates tab, range, search query, and fetches
    navHandler?.({ tab: 'sessions', range: '7d', query: 'claude-sonnet' });
    await flushPromises();

    expect(router.activeTab).toBe('sessions');
    expect(router.query.range).toBe('7d');
    expect(router.query.search).toBe('claude-sonnet');
    expect(getDashboardSnapshot).toHaveBeenCalledTimes(2);
    expect(getDashboardSnapshot).toHaveBeenLastCalledWith(
      expect.objectContaining({ range: '7d', search: 'claude-sonnet' }),
    );

    // Sync progress drives syncing indicator
    progressHandler?.({
      syncing: true,
      completedSources: 2,
      totalSources: 6,
      message: 'Syncing 2/6…',
    });
    expect(router.syncing).toBe(true);
    expect(router.syncMessage).toBe('Syncing 2/6…');

    progressHandler?.({
      syncing: false,
      completedSources: 6,
      totalSources: 6,
      message: 'Done',
    });
    expect(router.syncing).toBe(false);

    // Snapshot updated triggers silent refetch
    snapshotHandler?.({ timestamp: 1795262500000, partial: false, warnings: [] });
    await flushPromises();
    expect(getDashboardSnapshot).toHaveBeenCalledTimes(3);

    // Preferences updated applies theme
    prefsHandler?.({ ...mockPreferences(), theme: 'dark' });
    expect(applyTheme).toHaveBeenLastCalledWith('dark');

    stop();
  });

  it('applies pendingHandoff from initial cold load snapshot', async () => {
    const getDashboardSnapshot = vi.fn(async (q: DashboardQuery): Promise<DashboardSnapshot> => {
      const snap = mockDashboardSnapshot(q);
      if (getDashboardSnapshot.mock.calls.length === 1) {
        snap.pendingHandoff = { tab: 'models', range: '7d', query: 'opus' };
      }
      return snap;
    });

    const router = createDashboardRouter({
      getDashboardSnapshot,
      getPreferences: vi.fn(async () => mockPreferences()),
      onDashboardNavigate: async () => () => {},
      onSnapshotUpdated: async () => () => {},
      onSyncProgress: async () => () => {},
      onPreferencesUpdated: async () => () => {},
      applyTheme: vi.fn(),
    });

    const stop = router.start();
    await flushPromises();

    expect(router.activeTab).toBe('models');
    expect(router.query.range).toBe('7d');
    expect(router.query.search).toBe('opus');
    expect(getDashboardSnapshot).toHaveBeenCalledTimes(2);
    expect(getDashboardSnapshot).toHaveBeenLastCalledWith(
      expect.objectContaining({ range: '7d', search: 'opus' }),
    );

    stop();
  });

  it('debounces search by 120ms and blocks invalid custom ranges', async () => {
    const getDashboardSnapshot = vi.fn(async (q: DashboardQuery) => mockDashboardSnapshot(q));
    const router = createDashboardRouter({
      getDashboardSnapshot,
      getPreferences: vi.fn(async () => mockPreferences()),
      onDashboardNavigate: async () => () => {},
      onSnapshotUpdated: async () => () => {},
      onSyncProgress: async () => () => {},
      onPreferencesUpdated: async () => () => {},
      applyTheme: vi.fn(),
    });

    const stop = router.start();
    await flushPromises();
    expect(getDashboardSnapshot).toHaveBeenCalledTimes(1);

    router.setSearch('sonnet');
    expect(getDashboardSnapshot).toHaveBeenCalledTimes(1);

    vi.advanceTimersByTime(119);
    expect(getDashboardSnapshot).toHaveBeenCalledTimes(1);

    vi.advanceTimersByTime(1);
    await flushPromises();
    expect(getDashboardSnapshot).toHaveBeenCalledTimes(2);
    expect(getDashboardSnapshot).toHaveBeenLastCalledWith(
      expect.objectContaining({ search: 'sonnet' }),
    );

    router.setRange('custom');
    await flushPromises();
    const callsBeforeInvalid = getDashboardSnapshot.mock.calls.length;

    router.setCustomDates('2026-11-20', '2026-11-10');
    expect(router.customRangeError).toBe('Start date must be on or before end date');
    await flushPromises();
    expect(getDashboardSnapshot).toHaveBeenCalledTimes(callsBeforeInvalid);

    router.setCustomDates('2026-11-10', '2026-11-20');
    expect(router.customRangeError).toBeNull();
    await flushPromises();
    expect(getDashboardSnapshot).toHaveBeenCalledTimes(callsBeforeInvalid + 1);

    stop();
  });

  it('clears active client filters via clearClients() and skips fetch when already empty', async () => {
    const getDashboardSnapshot = vi.fn(async (q: DashboardQuery) => mockDashboardSnapshot(q));
    const router = createDashboardRouter({
      getDashboardSnapshot,
      getPreferences: vi.fn(async () => mockPreferences()),
      onDashboardNavigate: async () => () => {},
      onSnapshotUpdated: async () => () => {},
      onSyncProgress: async () => () => {},
      onPreferencesUpdated: async () => () => {},
      applyTheme: vi.fn(),
    });

    const stop = router.start();
    await flushPromises();
    expect(getDashboardSnapshot).toHaveBeenCalledTimes(1);

    // No-op when activeClients is already empty
    router.clearClients();
    await flushPromises();
    expect(getDashboardSnapshot).toHaveBeenCalledTimes(1);

    router.toggleClient('claude');
    await flushPromises();
    router.toggleClient('codex');
    await flushPromises();
    expect(router.query.activeClients).toEqual(['claude', 'codex']);
    expect(getDashboardSnapshot).toHaveBeenCalledTimes(3);

    router.clearClients();
    await flushPromises();
    expect(router.query.activeClients).toEqual([]);
    expect(getDashboardSnapshot).toHaveBeenCalledTimes(4);
    expect(getDashboardSnapshot).toHaveBeenLastCalledWith(
      expect.objectContaining({ activeClients: [] }),
    );

    stop();
  });
});

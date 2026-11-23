import '@testing-library/jest-dom/vitest';
import { cleanup, fireEvent, render, screen, waitFor, within } from '@testing-library/svelte';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import {
  COMMANDS,
  EVENTS,
  type RangePreset,
  type TraySnapshot,
  type UpdateStatusPayload,
} from '$lib/contracts/ipc';
import {
  mockCliInfo,
  mockPreferences,
  mockTraySnapshot,
  mockUpdateStatus,
} from '$lib/contracts/mocks';

const { invokeMock, listenMock, eventHandlers, unlistenMocks } = vi.hoisted(() => {
  const eventHandlers = new Map<string, Set<(event: { payload: unknown }) => void>>();
  const unlistenMocks: Array<ReturnType<typeof vi.fn>> = [];
  const invokeMock = vi.fn();
  const listenMock = vi.fn(async (eventName: string, cb: (event: { payload: unknown }) => void) => {
    let listeners = eventHandlers.get(eventName);
    if (!listeners) {
      listeners = new Set();
      eventHandlers.set(eventName, listeners);
    }
    listeners.add(cb);
    const unlisten = vi.fn(() => {
      listeners?.delete(cb);
    });
    unlistenMocks.push(unlisten);
    return unlisten;
  });
  return { invokeMock, listenMock, eventHandlers, unlistenMocks };
});

vi.mock('@tauri-apps/api/core', () => ({
  invoke: invokeMock,
}));

vi.mock('@tauri-apps/api/event', () => ({
  listen: listenMock,
}));

import TrayApp from './TrayApp.svelte';

function emitTauriEvent(eventName: string, payload: unknown): void {
  const listeners = eventHandlers.get(eventName);
  if (!listeners) return;
  for (const cb of listeners) {
    cb({ payload });
  }
}

describe('TrayApp', () => {
  let snapshotOverrides: Partial<Record<RangePreset, TraySnapshot>> = {};
  let prefsState = mockPreferences();

  beforeEach(() => {
    snapshotOverrides = {};
    prefsState = mockPreferences();
    eventHandlers.clear();
    unlistenMocks.length = 0;
    invokeMock.mockReset();
    listenMock.mockClear();
    delete document.documentElement.dataset.theme;
    document.documentElement.style.colorScheme = '';

    Object.defineProperty(window, '__TAURI_INTERNALS__', {
      value: {},
      configurable: true,
    });

    invokeMock.mockImplementation(async (cmd: string, args?: Record<string, unknown>) => {
      switch (cmd) {
        case COMMANDS.getTraySnapshot: {
          const range = (args?.range as RangePreset) ?? 'today';
          return snapshotOverrides[range] ?? mockTraySnapshot(range);
        }
        case COMMANDS.getPreferences:
          return { ...prefsState };
        case COMMANDS.updatePreferences: {
          const patch = (args?.patch as Partial<typeof prefsState>) ?? {};
          prefsState = { ...prefsState, ...patch };
          return { ...prefsState };
        }
        case COMMANDS.getCliInfo:
          return mockCliInfo();
        case COMMANDS.installOrUpgradeCli:
          return { ok: true, message: 'tokscale 4.24.0 installed' };
        case COMMANDS.triggerSync:
          return { syncing: false, lastSyncedAt: 1795262500000, warnings: [] };
        case COMMANDS.openDashboard:
        case COMMANDS.dismissTray:
        case COMMANDS.resizeTray:
        case COMMANDS.quitApp:
          return undefined;
        case COMMANDS.getUpdateStatus:
          return mockUpdateStatus();
        case COMMANDS.downloadAndInstallUpdate:
          return {
            ...mockUpdateStatus(),
            phase: 'ready',
            availableVersion: '0.2.0',
          } satisfies UpdateStatusPayload;
        case COMMANDS.restartToApplyUpdate:
          return undefined;
        default:
          return undefined;
      }
    });
  });

  afterEach(() => {
    cleanup();
    delete (window as unknown as Record<string, unknown>).__TAURI_INTERNALS__;
  });

  it('focuses Sync now on open and renders KPIs, chart, legend, and top 3 models', async () => {
    render(TrayApp);

    const syncBtn = screen.getByRole('button', { name: 'Sync now' });
    expect(document.activeElement).toBe(syncBtn);

    const todaySnap = mockTraySnapshot('today');
    await screen.findByText(`$${todaySnap.total.cost.toFixed(2)}`);

    const kpiSection = screen.getByLabelText('Usage summary');
    expect(within(kpiSection).getByText('Cost')).toBeInTheDocument();
    expect(within(kpiSection).getByText('Tokens')).toBeInTheDocument();
    expect(within(kpiSection).getByText('Sessions')).toBeInTheDocument();

    expect(screen.getByRole('img', { name: 'Usage cost by harness' })).toBeInTheDocument();
    const modelsSection = screen.getByLabelText('Top models');
    expect(within(modelsSection).getAllByRole('button')).toHaveLength(
      Math.min(3, todaySnap.topModels.length),
    );
  });

  it('caches per-range snapshots in memory and makes zero extra IPC calls when switching back and forth', async () => {
    render(TrayApp);

    const todaySnap = mockTraySnapshot('today');
    await screen.findByText(`$${todaySnap.total.cost.toFixed(2)}`);

    const snapshotCalls = () =>
      invokeMock.mock.calls.filter(([cmd]) => cmd === COMMANDS.getTraySnapshot);

    expect(snapshotCalls()).toHaveLength(1);
    expect(snapshotCalls()[0][1]).toEqual({ range: 'today' });

    const radios = screen.getAllByRole('radio');
    const radio7d = radios.find((el) => el.textContent?.trim() === '7D')!;
    const radioToday = radios.find((el) => el.textContent?.trim() === 'Today')!;
    const radio30d = radios.find((el) => el.textContent?.trim() === '30D')!;

    await fireEvent.click(radio7d);
    const snap7d = mockTraySnapshot('7d');
    await screen.findByText(`$${snap7d.total.cost.toFixed(2)}`);
    expect(snapshotCalls()).toHaveLength(2);
    expect(snapshotCalls()[1][1]).toEqual({ range: '7d' });

    // Switch back to Today -> 0 additional IPC calls
    await fireEvent.click(radioToday);
    await screen.findByText(`$${todaySnap.total.cost.toFixed(2)}`);
    expect(snapshotCalls()).toHaveLength(2);

    // Switch back to 7D -> 0 additional IPC calls
    await fireEvent.click(radio7d);
    await screen.findByText(`$${snap7d.total.cost.toFixed(2)}`);
    expect(snapshotCalls()).toHaveLength(2);

    // Fetch 30D once -> 3rd call, then switch back to Today and 30D with 0 extra calls
    await fireEvent.click(radio30d);
    const snap30d = mockTraySnapshot('30d');
    await screen.findByText(`$${snap30d.total.cost.toFixed(2)}`);
    expect(snapshotCalls()).toHaveLength(3);

    await fireEvent.click(radioToday);
    await screen.findByText(`$${todaySnap.total.cost.toFixed(2)}`);
    await fireEvent.click(radio30d);
    await screen.findByText(`$${snap30d.total.cost.toFixed(2)}`);
    expect(snapshotCalls()).toHaveLength(3);

    // snapshot-updated refreshes active range and invalidates cache
    emitTauriEvent(EVENTS.snapshotUpdated, {
      timestamp: 1795262600000,
      partial: false,
      warnings: [],
    });
    await waitFor(() => {
      expect(snapshotCalls()).toHaveLength(4);
    });
    expect(snapshotCalls()[3][1]).toEqual({ range: '30d' });
  });

  it('renders Empty state when range has zero usage and hides the chart', async () => {
    const emptySnap: TraySnapshot = {
      ...mockTraySnapshot('today'),
      total: { cost: 0, tokens: 0, sessions: 0 },
      bars: [],
      harnesses: [],
      topModels: [],
    };
    snapshotOverrides.today = emptySnap;

    render(TrayApp);

    await screen.findByText('No usage in this range');
    expect(screen.queryByRole('img', { name: 'Usage cost by harness' })).not.toBeInTheDocument();
    expect(screen.queryByLabelText('Top models')).not.toBeInTheDocument();
  });

  it('renders sleek Syncing progress hairline in footer without in-flow banner shift', async () => {
    const { container } = render(TrayApp);

    const todaySnap = mockTraySnapshot('today');
    await screen.findByText(`$${todaySnap.total.cost.toFixed(2)}`);
    expect(container.querySelector('.tray-sync-line')).not.toBeInTheDocument();

    emitTauriEvent(EVENTS.syncProgress, {
      syncing: true,
      completedSources: 2,
      totalSources: 6,
      message: 'Scanning Codex sessions…',
    });

    await screen.findByText('Scanning Codex sessions…');
    expect(screen.getByRole('button', { name: 'Sync now' })).toHaveClass('is-syncing');
    expect(container.querySelector('.tray-footer .tray-sync-line')).toBeInTheDocument();
    expect(container.querySelector('.tray-sync-bar')).not.toBeInTheDocument();
    expect(container.querySelector('.tray-footer-status')).toHaveTextContent(
      'Scanning Codex sessions…',
    );
    expect(screen.getByText(`$${todaySnap.total.cost.toFixed(2)}`)).toBeInTheDocument();
    expect(screen.getByRole('img', { name: 'Usage cost by harness' })).toBeInTheDocument();
  });

  it('renders Partial warn banner on snapshot-updated with partial warnings and retries', async () => {
    render(TrayApp);

    const todaySnap = mockTraySnapshot('today');
    await screen.findByText(`$${todaySnap.total.cost.toFixed(2)}`);

    emitTauriEvent(EVENTS.snapshotUpdated, {
      timestamp: 1795262700000,
      partial: true,
      warnings: [{ source: 'cursor', message: 'Token expired' }],
    });

    await screen.findByText('Token expired');
    expect(screen.getByText('cursor')).toBeInTheDocument();

    const retryBtn = screen.getByRole('button', { name: 'Retry' });
    await fireEvent.click(retryBtn);

    expect(invokeMock).toHaveBeenCalledWith(COMMANDS.triggerSync, {
      mode: { retryClient: 'cursor' },
    });
  });

  it('drills down into Models tab on TopModels click and Overview tab on Open dashboard', async () => {
    render(TrayApp);

    const todaySnap = mockTraySnapshot('today');
    await screen.findByText(`$${todaySnap.total.cost.toFixed(2)}`);

    const firstModel = todaySnap.topModels[0];
    const modelsSection = screen.getByLabelText('Top models');
    const firstModelBtn = within(modelsSection).getAllByRole('button')[0];

    await fireEvent.click(firstModelBtn);
    expect(invokeMock).toHaveBeenCalledWith(COMMANDS.openDashboard, {
      handoff: {
        tab: 'models',
        range: 'today',
        query: firstModel.model,
      },
    });

    await fireEvent.click(screen.getByRole('button', { name: 'Open dashboard' }));
    expect(invokeMock).toHaveBeenCalledWith(COMMANDS.openDashboard, {
      handoff: {
        tab: 'overview',
        range: 'today',
      },
    });
  });

  it('opens the separate Settings window via gear icon or Cmd+, and handles preferences-updated, Escape, and Quit', async () => {
    const { unmount } = render(TrayApp);

    const todaySnap = mockTraySnapshot('today');
    await screen.findByText(`$${todaySnap.total.cost.toFixed(2)}`);

    const gearBtn = screen.getByRole('button', { name: 'Settings' });
    await fireEvent.click(gearBtn);
    expect(invokeMock).toHaveBeenCalledWith(COMMANDS.openSettings, {});

    // Cmd+, also opens the Settings window
    await fireEvent.keyDown(window, { key: ',', metaKey: true });
    expect(
      invokeMock.mock.calls.filter(([cmd]) => cmd === COMMANDS.openSettings),
    ).toHaveLength(2);

    // preferences-updated event applies theme immediately
    emitTauriEvent(EVENTS.preferencesUpdated, {
      ...prefsState,
      theme: 'dark',
    });
    await waitFor(() => {
      expect(document.documentElement.dataset.theme).toBe('dark');
    });

    emitTauriEvent(EVENTS.preferencesUpdated, {
      ...prefsState,
      theme: 'light',
    });
    await waitFor(() => {
      expect(document.documentElement.dataset.theme).toBeUndefined();
    });

    // Escape calls dismiss_tray
    await fireEvent.keyDown(window, { key: 'Escape' });
    expect(invokeMock).toHaveBeenCalledWith(COMMANDS.dismissTray);

    // Quit calls quit_app
    await fireEvent.click(screen.getByRole('button', { name: 'Quit' }));
    expect(invokeMock).toHaveBeenCalledWith(COMMANDS.quitApp);

    // Listeners are cleaned up on unmount
    unmount();
    expect(unlistenMocks).toHaveLength(4);
    for (const unlisten of unlistenMocks) {
      expect(unlisten).toHaveBeenCalledTimes(1);
    }
  });

  it('resizes native tray popover to shell height and formats Unix-second lastSyncedAt', async () => {
    let shellHeight = 428;
    let triggerResize: () => void = () => {};
    const disconnectSpy = vi.fn();

    class MockResizeObserver {
      constructor(cb: ResizeObserverCallback) {
        triggerResize = () => cb([], this as unknown as ResizeObserver);
      }
      observe() {}
      unobserve() {}
      disconnect() {
        disconnectSpy();
      }
    }

    vi.stubGlobal('ResizeObserver', MockResizeObserver);
    const rectSpy = vi
      .spyOn(HTMLElement.prototype, 'getBoundingClientRect')
      .mockImplementation(() => ({
        x: 0,
        y: 0,
        top: 0,
        left: 0,
        right: 360,
        bottom: shellHeight,
        width: 360,
        height: shellHeight,
        toJSON: () => ({}),
      }));

    try {
      const nowSecs = Math.floor(Date.now() / 1000);
      snapshotOverrides.today = {
        ...mockTraySnapshot('today'),
        syncStatus: {
          syncing: false,
          lastSyncedAt: nowSecs - 120,
          warnings: [],
        },
      };

      const { unmount } = render(TrayApp);

      await screen.findByText('Synced 2m ago');
      expect(screen.queryByText(/20758d ago/)).not.toBeInTheDocument();

      const resizeCalls = () =>
        invokeMock.mock.calls.filter(([cmd]) => cmd === COMMANDS.resizeTray);

      await waitFor(() => {
        expect(resizeCalls()).toEqual([[COMMANDS.resizeTray, { height: 428 }]]);
      });

      // Unchanged height should not emit duplicate IPC calls
      triggerResize();
      expect(resizeCalls()).toHaveLength(1);

      // Shrinking content (e.g. switching to empty range) updates native popover height
      shellHeight = 304;
      triggerResize();
      await waitFor(() => {
        expect(resizeCalls()).toEqual([
          [COMMANDS.resizeTray, { height: 428 }],
          [COMMANDS.resizeTray, { height: 304 }],
        ]);
      });

      unmount();
      expect(disconnectSpy).toHaveBeenCalledTimes(1);
    } finally {
      rectSpy.mockRestore();
      vi.unstubAllGlobals();
    }
  });

  it('shows UpdateBanner on tokscale://update-status and handles update, restart, and dismiss', async () => {
    render(TrayApp);
    await screen.findByRole('button', { name: 'Open dashboard' });
    await waitFor(() => {
      expect(eventHandlers.get(EVENTS.updateStatus)?.size ?? 0).toBeGreaterThan(0);
    });

    emitTauriEvent(EVENTS.updateStatus, {
      ...mockUpdateStatus(),
      phase: 'available',
      availableVersion: '0.2.0',
    } satisfies UpdateStatusPayload);

    expect(await screen.findByText('Update available · v0.2.0')).toBeInTheDocument();

    await fireEvent.click(screen.getByRole('button', { name: 'Update' }));
    await waitFor(() => {
      expect(invokeMock).toHaveBeenCalledWith(COMMANDS.downloadAndInstallUpdate);
    });

    expect(await screen.findByText('v0.2.0 ready')).toBeInTheDocument();
    await fireEvent.click(screen.getByRole('button', { name: 'Restart' }));
    expect(invokeMock).toHaveBeenCalledWith(COMMANDS.restartToApplyUpdate);

    // Emit another available update and dismiss it for this session
    emitTauriEvent(EVENTS.updateStatus, {
      ...mockUpdateStatus(),
      phase: 'available',
      availableVersion: '0.3.0',
    } satisfies UpdateStatusPayload);
    expect(await screen.findByText('Update available · v0.3.0')).toBeInTheDocument();

    await fireEvent.click(screen.getByRole('button', { name: 'Dismiss update notice' }));
    expect(screen.queryByText('Update available · v0.3.0')).not.toBeInTheDocument();
  });
});

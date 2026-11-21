import '@testing-library/jest-dom/vitest';
import { cleanup, render, screen, waitFor } from '@testing-library/svelte';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import {
  COMMANDS,
  EVENTS,
  isMockMode,
  type DashboardNavigateEvent,
  type DashboardQuery,
  type SessionTitlesUpdatedEvent,
  type SessionsQuery,
} from '$lib/contracts/ipc';
import {
  mockDashboardSnapshot,
  mockPreferences,
  mockSessionsPayload,
} from '$lib/contracts/mocks';

const { invokeMock, listenMock, eventHandlers, unlistenMocks } = vi.hoisted(() => {
  const eventHandlers = new Map<string, Set<(event: { payload: unknown }) => void>>();
  const unlistenMocks: Array<ReturnType<typeof vi.fn>> = [];
  const invokeMock = vi.fn();
  const listenMock = vi.fn(
    async (eventName: string, cb: (event: { payload: unknown }) => void) => {
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
    },
  );
  return { invokeMock, listenMock, eventHandlers, unlistenMocks };
});

vi.mock('@tauri-apps/api/core', () => ({
  invoke: invokeMock,
}));

vi.mock('@tauri-apps/api/event', () => ({
  listen: listenMock,
}));

import DashboardApp from './dashboard/DashboardApp.svelte';

function emitTauriEvent(eventName: string, payload: unknown): void {
  const listeners = eventHandlers.get(eventName);
  if (!listeners) return;
  for (const cb of listeners) {
    cb({ payload });
  }
}

describe('Dashboard E2E IPC integration', () => {
  beforeEach(() => {
    eventHandlers.clear();
    unlistenMocks.length = 0;
    invokeMock.mockReset();
    listenMock.mockClear();
    delete document.documentElement.dataset.theme;
    document.documentElement.style.colorScheme = '';

    (window as unknown as Record<string, unknown>).__TAURI_INTERNALS__ = {};

    invokeMock.mockImplementation(async (cmd: string, args?: Record<string, unknown>) => {
      switch (cmd) {
        case COMMANDS.getDashboardSnapshot: {
          const query = args?.query as DashboardQuery;
          return mockDashboardSnapshot(query);
        }
        case COMMANDS.getPreferences:
          return mockPreferences();
        case COMMANDS.getSessionsView: {
          const query = args?.query as SessionsQuery;
          return mockSessionsPayload(query);
        }
        default:
          return undefined;
      }
    });
  });

  afterEach(() => {
    cleanup();
    delete (window as unknown as Record<string, unknown>).__TAURI_INTERNALS__;
  });

  it('drives DashboardApp through real ipc invoke/listen for dashboard-navigate and session-titles-updated', async () => {
    expect(isMockMode()).toBe(false);

    render(DashboardApp);

    await waitFor(() => {
      expect(invokeMock).toHaveBeenCalledWith(COMMANDS.getDashboardSnapshot, {
        query: expect.objectContaining({ range: '30d' }),
      });
    });

    await waitFor(() => {
      expect(eventHandlers.get(EVENTS.dashboardNavigate)?.size ?? 0).toBeGreaterThan(0);
    });

    const overviewTab = screen.getByRole('tab', { name: 'Overview' });
    expect(overviewTab).toHaveAttribute('aria-selected', 'true');

    const navigatePayload: DashboardNavigateEvent = {
      tab: 'sessions',
      range: '7d',
      query: 'tokscale',
    };
    emitTauriEvent(EVENTS.dashboardNavigate, navigatePayload);

    await waitFor(() => {
      const sessionsTab = screen.getByRole('tab', { name: 'Sessions' });
      expect(sessionsTab).toHaveAttribute('aria-selected', 'true');
    });

    await waitFor(() => {
      expect(invokeMock).toHaveBeenCalledWith(COMMANDS.getSessionsView, {
        query: expect.objectContaining({
          query: expect.objectContaining({ range: '7d', search: 'tokscale' }),
        }),
      });
    });

    await waitFor(() => {
      expect(eventHandlers.get(EVENTS.sessionTitlesUpdated)?.size ?? 0).toBeGreaterThan(0);
    });

    const sessionsCallsBefore = invokeMock.mock.calls.filter(
      ([cmd]) => cmd === COMMANDS.getSessionsView,
    ).length;

    const titlesEvent: SessionTitlesUpdatedEvent = { updatedCount: 3 };
    emitTauriEvent(EVENTS.sessionTitlesUpdated, titlesEvent);

    await waitFor(() => {
      const sessionsCallsAfter = invokeMock.mock.calls.filter(
        ([cmd]) => cmd === COMMANDS.getSessionsView,
      ).length;
      expect(sessionsCallsAfter).toBeGreaterThan(sessionsCallsBefore);
    });

    expect(await screen.findByText('Titles updated (3)')).toBeInTheDocument();
  });
});

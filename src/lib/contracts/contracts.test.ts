import { describe, expect, it } from 'vitest';
import { COMMANDS, EVENTS, ipc, isMockMode } from './ipc';
import {
  mockCliInfo,
  mockDashboardSnapshot,
  mockPreferences,
  mockSessionDetail,
  mockSessionsPayload,
  mockTraySnapshot,
} from './mocks';

describe('frozen contracts and deterministic mocks', () => {
  it('exposes frozen command and event names', () => {
    expect(COMMANDS.getTraySnapshot).toBe('get_tray_snapshot');
    expect(COMMANDS.getDashboardSnapshot).toBe('get_dashboard_snapshot');
    expect(COMMANDS.exportJson).toBe('export_json');
    expect(EVENTS.snapshotUpdated).toBe('tokscale://snapshot-updated');
    expect(EVENTS.dashboardNavigate).toBe('tokscale://dashboard-navigate');
  });

  it('produces deterministic mock snapshots across repeated calls', () => {
    const a = mockTraySnapshot('7d');
    const b = mockTraySnapshot('7d');
    expect(a).toEqual(b);

    const dashA = mockDashboardSnapshot({
      range: 'all',
      groupBy: 'model',
      activeClients: [],
      sort: 'cost',
    });
    const dashB = mockDashboardSnapshot({
      range: 'all',
      groupBy: 'model',
      activeClients: [],
      sort: 'cost',
    });
    expect(dashA).toEqual(dashB);
  });

  it('includes >= 5 harnesses plus unknown futureclient, 1440+ hourly rows, 365 daily rows, 52 weeks, and >=80% quota', () => {
    const dash = mockDashboardSnapshot({
      range: 'all',
      groupBy: 'model',
      activeClients: [],
      sort: 'cost',
    });

    const harnessIds = new Set(dash.harnesses.map((h) => h.id));
    for (const expected of ['claude', 'codex', 'opencode', 'gemini', 'capy', 'futureclient']) {
      expect(harnessIds.has(expected)).toBe(true);
    }

    expect(dash.daily.length).toBe(365);
    expect(dash.hourly.length).toBeGreaterThanOrEqual(1400);
    expect(dash.monthly.length).toBe(12);
    expect(dash.graph.weeks.length).toBe(52);
    expect(dash.agents.length).toBeGreaterThanOrEqual(3);
    expect(dash.sources.clients.length).toBeGreaterThanOrEqual(6);

    const hasNearLimit = dash.usage.some((p) =>
      p.metrics.some((m) => (m.usedPercent ?? 0) >= 80),
    );
    expect(hasNearLimit).toBe(true);
  });

  it('falls back cleanly to mocks when Tauri internals are absent', async () => {
    expect(isMockMode()).toBe(true);
    const tray = await ipc.getTraySnapshot('today');
    expect(tray.range).toBe('today');
    expect(tray.topModels.length).toBeLessThanOrEqual(3);

    const sessions = await ipc.getSessionsView({
      query: { range: '30d', groupBy: 'model', activeClients: [] },
      enrichTitles: false,
    });
    expect(sessions.sessions.length).toBeGreaterThan(0);
    expect(sessions.sessions[0].title).toBeUndefined();
    expect(sessions.projects.length).toBeGreaterThan(0);

    const detail = await ipc.getSessionDetail(
      sessions.sessions[0].client,
      sessions.sessions[0].sessionId,
    );
    expect(detail.models.length).toBeGreaterThan(0);
    expect(detail.models[0].tokens.cacheWrite).toBeGreaterThanOrEqual(0);

    const prefs = await ipc.getPreferences();
    expect(prefs).toEqual(mockPreferences());

    const cli = await ipc.getCliInfo();
    expect(cli).toEqual(mockCliInfo());
  });

  it('builds consistent sessions payload and session detail', () => {
    const payload = mockSessionsPayload();
    const first = payload.sessions[0];
    const detail = mockSessionDetail(first.client, first.sessionId);
    expect(detail.session.sessionId).toBe(first.sessionId);
  });
});

import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import {
  mockCliInfo,
  mockDashboardSnapshot,
  mockPreferences,
  mockSessionDetail,
  mockSessionsPayload,
  mockTraySnapshot,
} from './mocks';

export type RangePreset = 'today' | '7d' | '30d' | 'all' | 'custom';
export type DashboardTab =
  | 'overview'
  | 'usage'
  | 'models'
  | 'daily'
  | 'hourly'
  | 'monthly'
  | 'sessions'
  | 'projects'
  | 'stats'
  | 'agents';
export interface TokenCounts {
  input: number;
  output: number;
  cacheRead: number;
  cacheWrite: number;
  reasoning: number;
}
export interface SourceWarning {
  source: string;
  message: string;
}
export interface DashboardQuery {
  range: RangePreset;
  customSince?: string;
  customUntil?: string;
  groupBy: string;
  activeClients: string[];
  search?: string;
  sort?: 'cost' | 'tokens' | 'date';
}
export interface SessionsQuery {
  query: DashboardQuery;
  enrichTitles: boolean;
}
export type SyncMode = 'local_offline' | 'full_manual' | { retryClient: string };
export interface SyncStatus {
  syncing: boolean;
  lastSyncedAt?: number;
  warnings: SourceWarning[];
}
export interface CompanionPreferences {
  menubarDisplay: 'cost' | 'tokens' | 'icon';
  theme: 'light' | 'dark' | 'system';
  launchAtLogin: boolean;
  globalShortcut?: string;
  notificationsEnabled: boolean;
  quotaAlertPercent: number;
  dailyBudgetUsd?: number;
  sessionTitleEnrichment: boolean;
  customTokscalePath?: string;
}
export interface CliResolutionInfo {
  resolvedVia: string;
  binaryPath?: string;
  version?: string;
  fallback?: string;
}
export interface CliInstallResult {
  ok: boolean;
  message: string;
}
export interface SnapshotEvent {
  timestamp: number;
  partial: boolean;
  warnings: SourceWarning[];
}
export interface SyncProgressEvent {
  syncing: boolean;
  completedSources: number;
  totalSources: number;
  message: string;
}
export interface DashboardNavigateEvent {
  tab: DashboardTab;
  range?: RangePreset;
  query?: string;
}
export interface SessionTitlesUpdatedEvent {
  updatedCount: number;
}

export const COMMANDS = {
  getTraySnapshot: 'get_tray_snapshot',
  getDashboardSnapshot: 'get_dashboard_snapshot',
  getSessionsView: 'get_sessions_view',
  getSessionDetail: 'get_session_detail',
  triggerSync: 'trigger_sync',
  openDashboard: 'open_dashboard',
  dismissTray: 'dismiss_tray',
  resizeTray: 'resize_tray',
  quitApp: 'quit_app',
  getPreferences: 'get_preferences',
  updatePreferences: 'update_preferences',
  getCliInfo: 'get_cli_info',
  installOrUpgradeCli: 'install_or_upgrade_cli',
  exportJson: 'export_json',
} as const;

export const EVENTS = {
  snapshotUpdated: 'tokscale://snapshot-updated',
  syncProgress: 'tokscale://sync-progress',
  dashboardNavigate: 'tokscale://dashboard-navigate',
  sessionTitlesUpdated: 'tokscale://session-titles-updated',
  preferencesUpdated: 'tokscale://preferences-updated',
} as const;

export interface HarnessSeriesItem {
  id: string;
  label: string;
  color: string;
  monogram: string;
  cost: number;
  tokens: number;
  sharePercent: number;
  sessions?: number;
}
export interface ChartBar {
  slot: string;
  label: string;
  totalCost: number;
  totalTokens: number;
  segments: Array<{ harnessId: string; label: string; color: string; cost: number; tokens: number }>;
}
export interface TopModelSummary {
  model: string;
  client: string;
  provider: string;
  cost: number;
  tokens: number;
  sharePercent: number;
}
export interface SourceHealthRow {
  client: string;
  label: string;
  sessionsPath: string;
  sessionsPathExists: boolean;
  messageCount: number;
  headlessSupported: boolean;
  headlessMessageCount: number;
  status: 'ok' | 'warn' | 'danger';
  credentialed: boolean;
  apiBacked: boolean;
  lastSyncedAt?: number;
  warning?: string;
}

export interface TraySnapshot {
  range: RangePreset;
  total: { cost: number; tokens: number; sessions: number };
  bars: ChartBar[];
  harnesses: HarnessSeriesItem[];
  topModels: TopModelSummary[];
  sources: SourceHealthRow[];
  syncStatus: SyncStatus;
  cliInfo: CliResolutionInfo;
}

export interface ModelViewRow {
  model: string;
  colorKey: string;
  provider: string;
  client: string;
  workspaceKey?: string;
  workspaceLabel?: string;
  sessionId?: string;
  tokens: TokenCounts;
  cost: number;
  sessionCount: number;
  messageCount: number;
  msPer1KTokens?: number;
  groupKey: string;
}
export interface DailyViewRow {
  date: string;
  tokens: TokenCounts;
  cost: number;
  messageCount: number;
  turnCount?: number;
  sourceBreakdown: Array<{ client: string; label: string; tokens: TokenCounts; cost: number }>;
}
export interface HourlyViewRow {
  datetime: string;
  tokens: TokenCounts;
  cost: number;
  clients: string[];
  topModel?: string;
  messageCount: number;
  turnCount?: number;
}
export interface MonthlyViewRow {
  month: string;
  models: string[];
  tokens: TokenCounts;
  messageCount: number;
  sessionCount: number;
  activeDays: number;
  cost: number;
}
export interface GraphCell {
  date: string;
  tokens: TokenCounts;
  cost: number;
  intensity: 0 | 1 | 2 | 3 | 4;
}
export interface StatsSummary {
  activeDays: number;
  currentStreak: number;
  longestStreak: number;
  bestDay?: { date: string; cost: number };
  avgCostPerActiveDay: number;
}
export interface UsageMetricView {
  label: string;
  remainingLabel?: string;
  remainingPercent?: number;
  usedPercent?: number;
  resetsAt?: string;
}
export interface ProviderUsageView {
  provider: string;
  email?: string;
  plan?: string;
  metrics: UsageMetricView[];
}
export interface AgentPersonaRow {
  agent: string;
  clients: string;
  tokens: TokenCounts;
  cost: number;
  messageCount: number;
}

export interface DashboardSnapshot {
  query: DashboardQuery;
  kpis: { totalCost: number; totalTokens: number; sessions: number; costPerMillion: number; activeDays: number };
  chart: ChartBar[];
  harnesses: HarnessSeriesItem[];
  models: ModelViewRow[];
  daily: DailyViewRow[];
  hourly: HourlyViewRow[];
  monthly: MonthlyViewRow[];
  graph: { weeks: Array<Array<GraphCell | null>> };
  stats: StatsSummary;
  usage: ProviderUsageView[];
  agents: AgentPersonaRow[];
  sources: { headlessRoots: string[]; note: string; clients: SourceHealthRow[] };
  syncStatus: SyncStatus;
  pendingHandoff?: DashboardNavigateEvent;
}

export interface SessionRow {
  sessionId: string;
  shortId: string;
  client: string;
  title?: string;
  displayLabel: string;
  workspace?: string;
  workspaceLabel: string;
  modelsUsed: string[];
  messageCount: number;
  durationMinutes: number;
  inputTokens: number;
  outputTokens: number;
  cacheReadTokens: number;
  totalCost: number;
  createdAt?: string;
  lastActive?: string;
}
export interface ProjectRow {
  workspaceKey: string | null;
  workspaceLabel: string;
  sessionCount: number;
  totalTokens: number;
  totalCost: number;
  models: string[];
  lastActive: string | null;
}
export interface SessionsPayload {
  sessions: SessionRow[];
  projects: ProjectRow[];
}
export interface SessionModelBreakdown {
  model: string;
  provider: string;
  tokens: TokenCounts;
  cost: number;
  messageCount: number;
}
export interface SessionDetail {
  session: SessionRow;
  taskCategory?: string;
  description?: string;
  complexity?: string;
  taskGroup?: string;
  totalTokens: TokenCounts;
  totalCost: number;
  models: SessionModelBreakdown[];
}

export type { UnlistenFn };

export function isMockMode(): boolean {
  if (import.meta.env?.VITE_USE_MOCKS === '1') {
    return true;
  }
  if (typeof window === 'undefined') {
    return true;
  }
  return !('__TAURI_INTERNALS__' in window);
}

let mockPrefsState: CompanionPreferences | null = null;

function getMockPreferencesState(): CompanionPreferences {
  if (!mockPrefsState) {
    mockPrefsState = mockPreferences();
  }
  return mockPrefsState;
}

export const ipc = {
  async getTraySnapshot(range: RangePreset): Promise<TraySnapshot> {
    if (isMockMode()) {
      return mockTraySnapshot(range);
    }
    return invoke<TraySnapshot>(COMMANDS.getTraySnapshot, { range });
  },

  async getDashboardSnapshot(query: DashboardQuery): Promise<DashboardSnapshot> {
    if (isMockMode()) {
      return mockDashboardSnapshot(query);
    }
    return invoke<DashboardSnapshot>(COMMANDS.getDashboardSnapshot, { query });
  },

  async getSessionsView(query: SessionsQuery): Promise<SessionsPayload> {
    if (isMockMode()) {
      return mockSessionsPayload(query);
    }
    return invoke<SessionsPayload>(COMMANDS.getSessionsView, { query });
  },

  async getSessionDetail(client: string, sessionId: string): Promise<SessionDetail> {
    if (isMockMode()) {
      return mockSessionDetail(client, sessionId);
    }
    return invoke<SessionDetail>(COMMANDS.getSessionDetail, { client, sessionId });
  },

  async triggerSync(mode: SyncMode): Promise<SyncStatus> {
    if (isMockMode()) {
      return {
        syncing: false,
        lastSyncedAt: 1795262400000,
        warnings:
          typeof mode === 'object' && 'retryClient' in mode
            ? []
            : [{ source: 'cursor', message: 'Cached snapshot refreshed (mock mode)' }],
      };
    }
    return invoke<SyncStatus>(COMMANDS.triggerSync, { mode });
  },

  async openDashboard(handoff: DashboardNavigateEvent): Promise<void> {
    if (isMockMode()) {
      return;
    }
    await invoke<void>(COMMANDS.openDashboard, { handoff });
  },

  async dismissTray(): Promise<void> {
    if (isMockMode()) {
      return;
    }
    await invoke<void>(COMMANDS.dismissTray);
  },

  async resizeTray(height: number): Promise<void> {
    if (!Number.isFinite(height) || height <= 0 || isMockMode()) {
      return;
    }
    await invoke<void>(COMMANDS.resizeTray, { height });
  },

  async quitApp(): Promise<void> {
    if (isMockMode()) {
      return;
    }
    await invoke<void>(COMMANDS.quitApp);
  },

  async getPreferences(): Promise<CompanionPreferences> {
    if (isMockMode()) {
      return { ...getMockPreferencesState() };
    }
    return invoke<CompanionPreferences>(COMMANDS.getPreferences);
  },

  async updatePreferences(patch: Partial<CompanionPreferences>): Promise<CompanionPreferences> {
    if (isMockMode()) {
      mockPrefsState = { ...getMockPreferencesState(), ...patch };
      return { ...mockPrefsState };
    }
    return invoke<CompanionPreferences>(COMMANDS.updatePreferences, { patch });
  },

  async getCliInfo(): Promise<CliResolutionInfo> {
    if (isMockMode()) {
      return mockCliInfo();
    }
    return invoke<CliResolutionInfo>(COMMANDS.getCliInfo);
  },

  async installOrUpgradeCli(): Promise<CliInstallResult> {
    if (isMockMode()) {
      return { ok: true, message: 'tokscale 4.23.0 is installed (mock)' };
    }
    return invoke<CliInstallResult>(COMMANDS.installOrUpgradeCli);
  },

  async exportJson(query: DashboardQuery): Promise<string> {
    if (isMockMode()) {
      return JSON.stringify(mockDashboardSnapshot(query), null, 2);
    }
    return invoke<string>(COMMANDS.exportJson, { query });
  },
};

export async function onEvent<T>(eventName: string, handler: (payload: T) => void): Promise<UnlistenFn> {
  if (isMockMode()) {
    return () => {};
  }
  return listen<T>(eventName, (event) => {
    handler(event.payload);
  });
}

export function onSnapshotUpdated(handler: (payload: SnapshotEvent) => void): Promise<UnlistenFn> {
  return onEvent<SnapshotEvent>(EVENTS.snapshotUpdated, handler);
}

export function onSyncProgress(handler: (payload: SyncProgressEvent) => void): Promise<UnlistenFn> {
  return onEvent<SyncProgressEvent>(EVENTS.syncProgress, handler);
}

export function onDashboardNavigate(handler: (payload: DashboardNavigateEvent) => void): Promise<UnlistenFn> {
  return onEvent<DashboardNavigateEvent>(EVENTS.dashboardNavigate, handler);
}

export function onSessionTitlesUpdated(
  handler: (payload: SessionTitlesUpdatedEvent) => void,
): Promise<UnlistenFn> {
  return onEvent<SessionTitlesUpdatedEvent>(EVENTS.sessionTitlesUpdated, handler);
}

export function onPreferencesUpdated(
  handler: (payload: CompanionPreferences) => void,
): Promise<UnlistenFn> {
  return onEvent<CompanionPreferences>(EVENTS.preferencesUpdated, handler);
}

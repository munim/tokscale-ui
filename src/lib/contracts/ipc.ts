import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import {
  mockCliInfo,
  mockCustomPricing,
  mockDashboardSnapshot,
  mockPreferences,
  mockProvidersStatus,
  mockSessionDetail,
  mockSessionsPayload,
  mockSocialStatus,
  mockTokscaleSettings,
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

export type SettingsSection = 'app' | 'sources' | 'providers' | 'pricing' | 'social' | 'cli';

export interface SettingsNavigateEvent {
  section: SettingsSection;
}

export interface TokscaleSettingsState {
  path: string;
  exists: boolean;
  configDir: string;
  appPreferencesPath?: string;
  bucketTimezone?: string;
  defaultClients: string[];
  extraScanPaths: Record<string, string[]>;
  opencodeDbPaths: string[];
  localMachineLabel?: string;
  includeUnusedModels: boolean;
  modelAliases: Record<string, string>;
  nativeTimeoutMs: number;
  lightWriteCache: boolean;
  autoRefreshEnabled: boolean;
  autoRefreshMs: number;
  minutelyTabEnabled: boolean;
  colorPalette: string;
  tuiLanguage: string;
}

export interface TokscaleSettingsPatch {
  defaultClients?: string[];
  extraScanPaths?: Record<string, string[]>;
  opencodeDbPaths?: string[];
  localMachineLabel?: string | null;
  includeUnusedModels?: boolean;
  modelAliases?: Record<string, string>;
  nativeTimeoutMs?: number;
  lightWriteCache?: boolean;
  autoRefreshEnabled?: boolean;
  autoRefreshMs?: number;
  minutelyTabEnabled?: boolean;
  colorPalette?: string;
  tuiLanguage?: string;
}

export interface CustomPricingEntry {
  scope: 'model' | 'provider';
  providerId?: string;
  modelId: string;
  inputPerMillion?: number;
  outputPerMillion?: number;
  cacheReadPerMillion?: number;
  cacheWritePerMillion?: number;
  notes?: string;
  source?: string;
}

export interface CustomPricingState {
  path: string;
  exists: boolean;
  entries: CustomPricingEntry[];
  valid: boolean;
  validationMessage?: string;
}

export interface ModelPriceLookupResult {
  modelId: string;
  matchedKey?: string;
  source?: string;
  scope?: string;
  inputPerMillion?: number;
  outputPerMillion?: number;
  cacheReadPerMillion?: number;
  cacheWritePerMillion?: number;
  submissionSafe?: boolean;
}

export interface ProviderAccountInfo {
  name: string;
  active: boolean;
  email?: string;
  plan?: string;
}

export interface ProviderStatusItem {
  id: string;
  label: string;
  connected: boolean;
  supportsMultiAccount: boolean;
  summary: string;
  lastSyncedAt?: string;
  accounts: ProviderAccountInfo[];
  diagnostics: string[];
  variants: string[];
}

export interface ProvidersStatusPayload {
  providers: ProviderStatusItem[];
}

export interface ProviderActionRequest {
  provider: string;
  action: string;
  accountName?: string;
  token?: string;
  apiKey?: string;
  isCookie?: boolean;
  variant?: string;
  purgeCache?: boolean;
  allAccounts?: boolean;
  users?: string[];
  apiUrl?: string;
  tenant?: string;
}

export interface CliActionResult {
  ok: boolean;
  message: string;
  output?: string;
}

export interface AutosubmitStatus {
  enabled: boolean;
  intervalMinutes: number;
  scheduler?: string;
  clients: string[];
  today: boolean;
  yesterday: boolean;
  week: boolean;
  month: boolean;
  since?: string;
  until?: string;
  year?: string;
  lastRunAtMs?: number;
  lastError?: string;
}

export interface SocialStatusPayload {
  loggedIn: boolean;
  username?: string;
  detail?: string;
  autosubmit: AutosubmitStatus;
}

export interface SocialActionRequest {
  action: 'login_browser' | 'login_token' | 'logout' | 'submit_dry_run' | 'submit' | 'autosubmit_run';
  token?: string;
  clients?: string[];
  range?: string;
}

export interface AutosubmitConfigRequest {
  enabled: boolean;
  interval?: string;
  clients?: string[];
  range?: string;
}

export interface DashboardSources {
  headlessRoots: string[];
  note: string;
  clients: SourceHealthRow[];
}

export const COMMANDS = {
  getTraySnapshot: 'get_tray_snapshot',
  getDashboardSnapshot: 'get_dashboard_snapshot',
  getSessionsView: 'get_sessions_view',
  getSessionDetail: 'get_session_detail',
  triggerSync: 'trigger_sync',
  openDashboard: 'open_dashboard',
  openSettings: 'open_settings',
  takePendingSettingsSection: 'take_pending_settings_section',
  dismissTray: 'dismiss_tray',
  resizeTray: 'resize_tray',
  quitApp: 'quit_app',
  getPreferences: 'get_preferences',
  updatePreferences: 'update_preferences',
  getCliInfo: 'get_cli_info',
  installOrUpgradeCli: 'install_or_upgrade_cli',
  exportJson: 'export_json',
  getSourcesHealth: 'get_sources_health',
  getTokscaleSettings: 'get_tokscale_settings',
  updateTokscaleSettings: 'update_tokscale_settings',
  recoverTokscaleTimezone: 'recover_tokscale_timezone',
  getCustomPricing: 'get_custom_pricing',
  saveCustomPricing: 'save_custom_pricing',
  lookupModelPricing: 'lookup_model_pricing',
  refreshPricingCatalog: 'refresh_pricing_catalog',
  getProvidersStatus: 'get_providers_status',
  runProviderAction: 'run_provider_action',
  getSocialStatus: 'get_social_status',
  runSocialAction: 'run_social_action',
  configureAutosubmit: 'configure_autosubmit',
  revealConfigPath: 'reveal_config_path',
  pickPath: 'pick_path',
} as const;

export const EVENTS = {
  snapshotUpdated: 'tokscale://snapshot-updated',
  syncProgress: 'tokscale://sync-progress',
  dashboardNavigate: 'tokscale://dashboard-navigate',
  sessionTitlesUpdated: 'tokscale://session-titles-updated',
  preferencesUpdated: 'tokscale://preferences-updated',
  settingsNavigate: 'tokscale://settings-navigate',
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
let mockTokscaleSettingsState: TokscaleSettingsState | null = null;
let mockCustomPricingState: CustomPricingState | null = null;
let mockProvidersState: ProvidersStatusPayload | null = null;
let mockSocialState: SocialStatusPayload | null = null;

function getMockPreferencesState(): CompanionPreferences {
  if (!mockPrefsState) {
    mockPrefsState = mockPreferences();
  }
  return mockPrefsState;
}

function getMockTokscaleSettingsState(): TokscaleSettingsState {
  if (!mockTokscaleSettingsState) {
    mockTokscaleSettingsState = mockTokscaleSettings();
  }
  return mockTokscaleSettingsState;
}

function getMockCustomPricingState(): CustomPricingState {
  if (!mockCustomPricingState) {
    mockCustomPricingState = mockCustomPricing();
  }
  return mockCustomPricingState;
}

function getMockProvidersState(): ProvidersStatusPayload {
  if (!mockProvidersState) {
    mockProvidersState = mockProvidersStatus();
  }
  return mockProvidersState;
}

function getMockSocialState(): SocialStatusPayload {
  if (!mockSocialState) {
    mockSocialState = mockSocialStatus();
  }
  return mockSocialState;
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

  async openSettings(section?: SettingsSection): Promise<void> {
    if (isMockMode()) {
      return;
    }
    await invoke<void>(COMMANDS.openSettings, section ? { section } : {});
  },

  async takePendingSettingsSection(): Promise<SettingsSection | null> {
    if (isMockMode()) {
      return null;
    }
    return invoke<SettingsSection | null>(COMMANDS.takePendingSettingsSection);
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

  async getSourcesHealth(): Promise<DashboardSources> {
    if (isMockMode()) {
      return mockDashboardSnapshot().sources;
    }
    return invoke<DashboardSources>(COMMANDS.getSourcesHealth);
  },

  async getTokscaleSettings(): Promise<TokscaleSettingsState> {
    if (isMockMode()) {
      return structuredClone(getMockTokscaleSettingsState());
    }
    return invoke<TokscaleSettingsState>(COMMANDS.getTokscaleSettings);
  },

  async updateTokscaleSettings(patch: TokscaleSettingsPatch): Promise<TokscaleSettingsState> {
    if (isMockMode()) {
      const current = getMockTokscaleSettingsState();
      const next: TokscaleSettingsState = {
        ...current,
        ...(patch.defaultClients !== undefined ? { defaultClients: [...patch.defaultClients] } : {}),
        ...(patch.extraScanPaths !== undefined ? { extraScanPaths: structuredClone(patch.extraScanPaths) } : {}),
        ...(patch.opencodeDbPaths !== undefined ? { opencodeDbPaths: [...patch.opencodeDbPaths] } : {}),
        ...(patch.localMachineLabel !== undefined
          ? { localMachineLabel: patch.localMachineLabel ?? undefined }
          : {}),
        ...(patch.includeUnusedModels !== undefined ? { includeUnusedModels: patch.includeUnusedModels } : {}),
        ...(patch.modelAliases !== undefined ? { modelAliases: { ...patch.modelAliases } } : {}),
        ...(patch.nativeTimeoutMs !== undefined ? { nativeTimeoutMs: patch.nativeTimeoutMs } : {}),
        ...(patch.lightWriteCache !== undefined ? { lightWriteCache: patch.lightWriteCache } : {}),
        ...(patch.autoRefreshEnabled !== undefined ? { autoRefreshEnabled: patch.autoRefreshEnabled } : {}),
        ...(patch.autoRefreshMs !== undefined ? { autoRefreshMs: patch.autoRefreshMs } : {}),
        ...(patch.minutelyTabEnabled !== undefined ? { minutelyTabEnabled: patch.minutelyTabEnabled } : {}),
        ...(patch.colorPalette !== undefined ? { colorPalette: patch.colorPalette } : {}),
        ...(patch.tuiLanguage !== undefined ? { tuiLanguage: patch.tuiLanguage } : {}),
      };
      mockTokscaleSettingsState = next;
      return structuredClone(next);
    }
    return invoke<TokscaleSettingsState>(COMMANDS.updateTokscaleSettings, { patch });
  },

  async recoverTokscaleTimezone(): Promise<CliActionResult> {
    if (isMockMode()) {
      return { ok: true, message: 'Timezone verified: Europe/Berlin (mock)' };
    }
    return invoke<CliActionResult>(COMMANDS.recoverTokscaleTimezone);
  },

  async getCustomPricing(): Promise<CustomPricingState> {
    if (isMockMode()) {
      return structuredClone(getMockCustomPricingState());
    }
    return invoke<CustomPricingState>(COMMANDS.getCustomPricing);
  },

  async saveCustomPricing(entries: CustomPricingEntry[]): Promise<CustomPricingState> {
    if (isMockMode()) {
      mockCustomPricingState = {
        ...getMockCustomPricingState(),
        exists: true,
        entries: structuredClone(entries),
        valid: true,
        validationMessage: 'Validated by tokscale CLI (mock)',
      };
      return structuredClone(mockCustomPricingState);
    }
    return invoke<CustomPricingState>(COMMANDS.saveCustomPricing, { entries });
  },

  async lookupModelPricing(modelId: string): Promise<ModelPriceLookupResult> {
    if (isMockMode()) {
      return {
        modelId,
        matchedKey: modelId,
        source: 'LiteLLM',
        scope: 'upstream',
        inputPerMillion: 3.0,
        outputPerMillion: 15.0,
        cacheReadPerMillion: 0.3,
        cacheWritePerMillion: 3.75,
        submissionSafe: true,
      };
    }
    return invoke<ModelPriceLookupResult>(COMMANDS.lookupModelPricing, { modelId });
  },

  async refreshPricingCatalog(): Promise<CliActionResult> {
    if (isMockMode()) {
      return { ok: true, message: 'Pricing catalogs refreshed (mock)' };
    }
    return invoke<CliActionResult>(COMMANDS.refreshPricingCatalog);
  },

  async getProvidersStatus(): Promise<ProvidersStatusPayload> {
    if (isMockMode()) {
      return structuredClone(getMockProvidersState());
    }
    return invoke<ProvidersStatusPayload>(COMMANDS.getProvidersStatus);
  },

  async runProviderAction(request: ProviderActionRequest): Promise<CliActionResult> {
    if (isMockMode()) {
      return {
        ok: true,
        message: `${request.provider} ${request.action} completed (mock)`,
      };
    }
    return invoke<CliActionResult>(COMMANDS.runProviderAction, { request });
  },

  async getSocialStatus(): Promise<SocialStatusPayload> {
    if (isMockMode()) {
      return structuredClone(getMockSocialState());
    }
    return invoke<SocialStatusPayload>(COMMANDS.getSocialStatus);
  },

  async runSocialAction(request: SocialActionRequest): Promise<CliActionResult> {
    if (isMockMode()) {
      if (request.action === 'submit_dry_run') {
        return {
          ok: true,
          message: 'Dry run ready: 142,800 tokens across 3 clients ($4.12)',
          output: 'Dry run summary\n  Days: 7\n  Clients: claude, codex, opencode\n  Estimated spend: $4.12\n  Excluded unpriced messages: 0',
        };
      }
      if (request.action === 'login_token' || request.action === 'login_browser') {
        mockSocialState = {
          ...getMockSocialState(),
          loggedIn: true,
          username: 'devuser',
          detail: 'Logged in as @devuser',
        };
      } else if (request.action === 'logout') {
        mockSocialState = {
          ...getMockSocialState(),
          loggedIn: false,
          username: undefined,
          detail: 'Not logged in',
        };
      }
      return {
        ok: true,
        message: `${request.action.replace(/_/g, ' ')} succeeded (mock)`,
      };
    }
    return invoke<CliActionResult>(COMMANDS.runSocialAction, { request });
  },

  async configureAutosubmit(request: AutosubmitConfigRequest): Promise<CliActionResult> {
    if (isMockMode()) {
      const current = getMockSocialState();
      const minutes =
        request.interval === '30m'
          ? 30
          : request.interval === '1h'
            ? 60
            : request.interval === '6h'
              ? 360
              : 1440;
      mockSocialState = {
        ...current,
        autosubmit: {
          ...current.autosubmit,
          enabled: request.enabled,
          intervalMinutes: minutes,
          scheduler: request.enabled ? 'launchd' : undefined,
          clients: request.clients ?? [],
          today: request.range === 'today',
          week: request.range === 'week',
          month: request.range === 'month',
        },
      };
      return {
        ok: true,
        message: request.enabled ? 'Autosubmit enabled (mock)' : 'Autosubmit disabled (mock)',
      };
    }
    return invoke<CliActionResult>(COMMANDS.configureAutosubmit, { request });
  },

  async revealConfigPath(path: string): Promise<void> {
    if (isMockMode()) {
      return;
    }
    await invoke<void>(COMMANDS.revealConfigPath, { path });
  },

  async pickPath(kind: 'file' | 'folder'): Promise<string | null> {
    if (isMockMode()) {
      return kind === 'folder' ? '/Users/dev/projects/.codex' : '/Users/dev/.bun/bin/tokscale';
    }
    return invoke<string | null>(COMMANDS.pickPath, { kind });
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

export function onSettingsNavigate(
  handler: (payload: SettingsNavigateEvent) => void,
): Promise<UnlistenFn> {
  return onEvent<SettingsNavigateEvent>(EVENTS.settingsNavigate, handler);
}

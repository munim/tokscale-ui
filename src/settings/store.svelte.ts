import {
  ipc,
  onPreferencesUpdated,
  onSettingsNavigate,
  onSnapshotUpdated,
  type AutosubmitConfigRequest,
  type CliActionResult,
  type CliInstallResult,
  type CliResolutionInfo,
  type CompanionPreferences,
  type CustomPricingEntry,
  type CustomPricingState,
  type DashboardSources,
  type ModelPriceLookupResult,
  type ProviderActionRequest,
  type ProvidersStatusPayload,
  type SettingsSection,
  type SocialActionRequest,
  type SocialStatusPayload,
  type TokscaleSettingsPatch,
  type TokscaleSettingsState,
  type UnlistenFn,
} from '$lib/contracts/ipc';
import { applyTheme } from '$lib/state';

export interface ToastNotice {
  tone: 'ok' | 'danger' | 'info';
  text: string;
}

const DEFAULT_PREFS: CompanionPreferences = {
  menubarDisplay: 'cost',
  theme: 'system',
  launchAtLogin: false,
  notificationsEnabled: false,
  quotaAlertPercent: 85,
  sessionTitleEnrichment: true,
};

export function createSettingsStore() {
  let activeSection = $state<SettingsSection>('app');
  let searchQuery = $state('');
  let highlightedSettingId = $state<string | null>(null);

  let preferences = $state<CompanionPreferences>({ ...DEFAULT_PREFS });
  let cliInfo = $state<CliResolutionInfo | null>(null);
  let sources = $state<DashboardSources | null>(null);
  let tokscaleSettings = $state<TokscaleSettingsState | null>(null);
  let customPricing = $state<CustomPricingState | null>(null);
  let providersStatus = $state<ProvidersStatusPayload | null>(null);
  let socialStatus = $state<SocialStatusPayload | null>(null);

  let loadingCore = $state(true);
  let loadingProviders = $state(false);
  let loadingSocial = $state(false);
  let providersLoaded = $state(false);
  let socialLoaded = $state(false);

  let cliInstalling = $state(false);
  let cliInstallResult = $state<CliInstallResult | null>(null);
  let syncingClient = $state<string | null>(null);
  let syncingAllProviders = $state(false);
  let refreshingPricing = $state(false);
  let lookingUpPrice = $state(false);
  let priceLookupResult = $state<ModelPriceLookupResult | null>(null);
  let priceLookupError = $state<string | null>(null);

  let lastActionOutput = $state<string | null>(null);
  let toast = $state<ToastNotice | null>(null);
  let toastTimer: ReturnType<typeof setTimeout> | null = null;

  function showToast(tone: ToastNotice['tone'], text: string, durationMs = 2800): void {
    toast = { tone, text };
    if (toastTimer) {
      clearTimeout(toastTimer);
    }
    toastTimer = setTimeout(() => {
      toast = null;
      toastTimer = null;
    }, durationMs);
  }

  function setSection(section: SettingsSection, highlightId: string | null = null): void {
    activeSection = section;
    highlightedSettingId = highlightId;
    if (section === 'providers' && !providersLoaded && !loadingProviders) {
      void refreshProviders();
    }
    if (section === 'social' && !socialLoaded && !loadingSocial) {
      void refreshSocial();
    }
  }

  function setSearchQuery(value: string): void {
    searchQuery = value;
  }

  function clearSearch(): void {
    searchQuery = '';
  }

  async function loadInitial(): Promise<void> {
    loadingCore = true;
    try {
      const [pendingSection, prefs, cli, src, tsSettings, pricing] = await Promise.all([
        ipc.takePendingSettingsSection().catch(() => null),
        ipc.getPreferences().catch(() => ({ ...DEFAULT_PREFS })),
        ipc.getCliInfo().catch(() => null),
        ipc.getSourcesHealth().catch(() => null),
        ipc.getTokscaleSettings().catch(() => null),
        ipc.getCustomPricing().catch(() => null),
      ]);

      preferences = prefs;
      applyTheme(prefs.theme);
      cliInfo = cli;
      sources = src;
      tokscaleSettings = tsSettings;
      customPricing = pricing;

      if (pendingSection) {
        activeSection = pendingSection;
      }

      if (activeSection === 'providers') {
        void refreshProviders();
      } else if (activeSection === 'social') {
        void refreshSocial();
      } else {
        // Warm providers status in background so connected badge stays live.
        void refreshProviders();
      }
    } finally {
      loadingCore = false;
    }
  }

  async function updatePreferences(patch: Partial<CompanionPreferences>): Promise<void> {
    if (patch.theme) {
      applyTheme(patch.theme);
    }
    const previous = { ...preferences };
    preferences = { ...preferences, ...patch };

    try {
      const saved = await ipc.updatePreferences(patch);
      preferences = saved;
      applyTheme(saved.theme);
      if ('customTokscalePath' in patch) {
        cliInfo = await ipc.getCliInfo().catch(() => cliInfo);
      }
      showToast('ok', 'Saved');
    } catch (err) {
      preferences = previous;
      applyTheme(previous.theme);
      showToast('danger', err instanceof Error ? err.message : 'Failed to save preferences', 4200);
    }
  }

  async function updateTokscaleSettings(patch: TokscaleSettingsPatch): Promise<void> {
    try {
      const saved = await ipc.updateTokscaleSettings(patch);
      tokscaleSettings = saved;
      showToast('ok', 'Updated settings.json');
    } catch (err) {
      showToast(
        'danger',
        err instanceof Error ? err.message : 'Failed to update tokscale settings',
        4500,
      );
    }
  }

  async function recoverTimezone(): Promise<CliActionResult> {
    const result = await ipc.recoverTokscaleTimezone();
    if (result.ok) {
      tokscaleSettings = await ipc.getTokscaleSettings().catch(() => tokscaleSettings);
      showToast('ok', result.message);
    } else {
      showToast('danger', result.message, 4500);
    }
    return result;
  }

  async function retrySource(client: string): Promise<void> {
    syncingClient = client;
    try {
      const status = await ipc.triggerSync({ retryClient: client });
      sources = await ipc.getSourcesHealth().catch(() => sources);
      if (status.warnings.length > 0) {
        showToast('danger', status.warnings[0].message, 4200);
      } else {
        showToast('ok', `Refreshed ${client}`);
      }
    } catch (err) {
      showToast('danger', err instanceof Error ? err.message : `Failed to sync ${client}`, 4200);
    } finally {
      syncingClient = null;
    }
  }

  async function refreshProviders(): Promise<void> {
    loadingProviders = true;
    try {
      providersStatus = await ipc.getProvidersStatus();
      providersLoaded = true;
    } catch (err) {
      showToast('danger', err instanceof Error ? err.message : 'Failed to inspect providers', 4000);
    } finally {
      loadingProviders = false;
    }
  }

  async function runProviderAction(request: ProviderActionRequest): Promise<CliActionResult> {
    try {
      const result = await ipc.runProviderAction(request);
      if (result.ok) {
        showToast('ok', result.message);
        await refreshProviders();
        sources = await ipc.getSourcesHealth().catch(() => sources);
      } else {
        showToast('danger', result.message, 4800);
      }
      return result;
    } catch (err) {
      const message = err instanceof Error ? err.message : 'Provider action failed';
      showToast('danger', message, 4800);
      return { ok: false, message };
    }
  }

  async function syncAllConnectedProviders(): Promise<void> {
    syncingAllProviders = true;
    try {
      const status = await ipc.triggerSync('full_manual');
      await refreshProviders();
      sources = await ipc.getSourcesHealth().catch(() => sources);
      if (status.warnings.length > 0) {
        showToast('danger', status.warnings[0].message, 4500);
      } else {
        showToast('ok', 'Synced all connected providers');
      }
    } catch (err) {
      showToast('danger', err instanceof Error ? err.message : 'Sync failed', 4500);
    } finally {
      syncingAllProviders = false;
    }
  }

  async function saveCustomPricing(entries: CustomPricingEntry[]): Promise<boolean> {
    try {
      const saved = await ipc.saveCustomPricing(entries);
      customPricing = saved;
      if (saved.valid) {
        showToast('ok', saved.validationMessage ?? 'Saved custom-pricing.json');
      } else {
        showToast('danger', saved.validationMessage ?? 'Saved with validation warnings', 5000);
      }
      return saved.valid;
    } catch (err) {
      showToast('danger', err instanceof Error ? err.message : 'Failed to save pricing', 4500);
      return false;
    }
  }

  async function lookupPrice(modelId: string): Promise<void> {
    const trimmed = modelId.trim();
    if (!trimmed) {
      priceLookupError = 'Enter a model ID (e.g. claude-sonnet-4-5)';
      return;
    }
    lookingUpPrice = true;
    priceLookupError = null;
    try {
      priceLookupResult = await ipc.lookupModelPricing(trimmed);
    } catch (err) {
      priceLookupResult = null;
      priceLookupError = err instanceof Error ? err.message : 'Model price not found';
    } finally {
      lookingUpPrice = false;
    }
  }

  async function refreshPricingCatalog(): Promise<void> {
    refreshingPricing = true;
    try {
      const result = await ipc.refreshPricingCatalog();
      showToast(result.ok ? 'ok' : 'danger', result.message, result.ok ? 3000 : 4500);
    } catch (err) {
      showToast('danger', err instanceof Error ? err.message : 'Catalog refresh failed', 4500);
    } finally {
      refreshingPricing = false;
    }
  }

  async function refreshSocial(): Promise<void> {
    loadingSocial = true;
    try {
      socialStatus = await ipc.getSocialStatus();
      socialLoaded = true;
    } catch (err) {
      showToast('danger', err instanceof Error ? err.message : 'Failed to load social status', 4000);
    } finally {
      loadingSocial = false;
    }
  }

  async function runSocialAction(request: SocialActionRequest): Promise<CliActionResult> {
    try {
      const result = await ipc.runSocialAction(request);
      if (result.output) {
        lastActionOutput = result.output;
      }
      showToast(result.ok ? 'ok' : 'danger', result.message, result.ok ? 3400 : 4800);
      await refreshSocial();
      return result;
    } catch (err) {
      const message = err instanceof Error ? err.message : 'Social action failed';
      showToast('danger', message, 4800);
      return { ok: false, message };
    }
  }

  async function configureAutosubmit(request: AutosubmitConfigRequest): Promise<CliActionResult> {
    try {
      const result = await ipc.configureAutosubmit(request);
      showToast(result.ok ? 'ok' : 'danger', result.message, result.ok ? 3200 : 4800);
      await refreshSocial();
      return result;
    } catch (err) {
      const message = err instanceof Error ? err.message : 'Autosubmit configuration failed';
      showToast('danger', message, 4800);
      return { ok: false, message };
    }
  }

  async function installOrUpgradeCli(): Promise<void> {
    if (cliInstalling) return;
    cliInstalling = true;
    cliInstallResult = null;
    try {
      const result = await ipc.installOrUpgradeCli();
      cliInstallResult = result;
      if (result.ok) {
        cliInfo = await ipc.getCliInfo().catch(() => cliInfo);
        showToast('ok', result.message);
      } else {
        showToast('danger', result.message, 4800);
      }
    } catch (err) {
      const message = err instanceof Error ? err.message : 'CLI install failed';
      cliInstallResult = { ok: false, message };
      showToast('danger', message, 4800);
    } finally {
      cliInstalling = false;
    }
  }

  async function pickPath(kind: 'file' | 'folder'): Promise<string | null> {
    try {
      return await ipc.pickPath(kind);
    } catch (err) {
      showToast('danger', err instanceof Error ? err.message : 'Failed to open picker', 3800);
      return null;
    }
  }

  async function revealPath(path: string): Promise<void> {
    try {
      await ipc.revealConfigPath(path);
    } catch (err) {
      showToast('danger', err instanceof Error ? err.message : 'Could not reveal path', 3800);
    }
  }

  function init(): () => void {
    let disposed = false;
    const unlisteners: UnlistenFn[] = [];

    const register = (p: Promise<UnlistenFn>) => {
      void p.then((fn) => {
        if (disposed) {
          fn();
        } else {
          unlisteners.push(fn);
        }
      });
    };

    register(
      onPreferencesUpdated((next) => {
        preferences = next;
        applyTheme(next.theme);
      }),
    );

    register(
      onSettingsNavigate((payload) => {
        setSection(payload.section);
      }),
    );

    register(
      onSnapshotUpdated(() => {
        void ipc.getSourcesHealth().then((src) => {
          if (!disposed) sources = src;
        }).catch(() => {});
      }),
    );

    void loadInitial();

    return () => {
      disposed = true;
      if (toastTimer) {
        clearTimeout(toastTimer);
      }
      for (const unlisten of unlisteners) {
        unlisten();
      }
      unlisteners.length = 0;
    };
  }

  let connectedProviderCount = $derived(
    (providersStatus?.providers ?? []).filter((p) => p.connected).length,
  );

  let detectedSourcesCount = $derived(
    (sources?.clients ?? []).filter((c) => c.sessionsPathExists).length,
  );

  let hasSourceWarnings = $derived(
    (sources?.clients ?? []).some((c) => c.status === 'danger' || Boolean(c.warning)),
  );

  return {
    get activeSection() {
      return activeSection;
    },
    get searchQuery() {
      return searchQuery;
    },
    get highlightedSettingId() {
      return highlightedSettingId;
    },
    get preferences() {
      return preferences;
    },
    get cliInfo() {
      return cliInfo;
    },
    get sources() {
      return sources;
    },
    get tokscaleSettings() {
      return tokscaleSettings;
    },
    get customPricing() {
      return customPricing;
    },
    get providersStatus() {
      return providersStatus;
    },
    get socialStatus() {
      return socialStatus;
    },
    get loadingCore() {
      return loadingCore;
    },
    get loadingProviders() {
      return loadingProviders;
    },
    get loadingSocial() {
      return loadingSocial;
    },
    get cliInstalling() {
      return cliInstalling;
    },
    get cliInstallResult() {
      return cliInstallResult;
    },
    get syncingClient() {
      return syncingClient;
    },
    get syncingAllProviders() {
      return syncingAllProviders;
    },
    get refreshingPricing() {
      return refreshingPricing;
    },
    get lookingUpPrice() {
      return lookingUpPrice;
    },
    get priceLookupResult() {
      return priceLookupResult;
    },
    get priceLookupError() {
      return priceLookupError;
    },
    get lastActionOutput() {
      return lastActionOutput;
    },
    get toast() {
      return toast;
    },
    get connectedProviderCount() {
      return connectedProviderCount;
    },
    get detectedSourcesCount() {
      return detectedSourcesCount;
    },
    get hasSourceWarnings() {
      return hasSourceWarnings;
    },
    init,
    setSection,
    setSearchQuery,
    clearSearch,
    updatePreferences,
    updateTokscaleSettings,
    recoverTimezone,
    retrySource,
    refreshProviders,
    runProviderAction,
    syncAllConnectedProviders,
    saveCustomPricing,
    lookupPrice,
    refreshPricingCatalog,
    refreshSocial,
    runSocialAction,
    configureAutosubmit,
    installOrUpgradeCli,
    pickPath,
    revealPath,
    showToast,
  };
}

export type SettingsStore = ReturnType<typeof createSettingsStore>;

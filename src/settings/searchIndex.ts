import type { SettingsSection } from '$lib/contracts/ipc';

export interface SettingsNavItem {
  id: SettingsSection;
  label: string;
  subtitle: string;
  keywords: string[];
}

export interface SearchableSetting {
  id: string;
  section: SettingsSection;
  label: string;
  description: string;
  keywords: string[];
}

export const SETTINGS_NAV: readonly SettingsNavItem[] = [
  {
    id: 'app',
    label: 'App & Alerts',
    subtitle: 'Menubar readout, theme, shortcuts, notifications, budgets, and updates.',
    keywords: ['general', 'appearance', 'theme', 'dark', 'light', 'menubar', 'tray', 'shortcut', 'hotkey', 'launch', 'login', 'alerts', 'budget', 'quota', 'notifications', 'titles', 'summarizer', 'update', 'updates', 'version', 'about', 'upgrade'],
  },
  {
    id: 'sources',
    label: 'Sources & Scanner',
    subtitle: 'Local harness health, default client filters, extra scan roots, and timezone.',
    keywords: ['sources', 'scanner', 'clients', 'harnesses', 'paths', 'opencode', 'sqlite', 'db', 'timezone', 'machine', 'label', 'folders'],
  },
  {
    id: 'providers',
    label: 'Providers',
    subtitle: 'API-backed providers, account switching, authentication, and cache sync.',
    keywords: ['providers', 'accounts', 'cursor', 'codex', 'trae', 'warp', 'oz', 'capy', 'antigravity', 'amp', 'hindsight', 'oauth', 'sync', 'login', 'logout'],
  },
  {
    id: 'pricing',
    label: 'Pricing & Models',
    subtitle: 'Pricing catalog refresh, model lookup, custom overrides, and model aliases.',
    keywords: ['pricing', 'models', 'overrides', 'custom-pricing', 'cost', 'free', 'aliases', 'unused', 'litellm', 'openrouter', 'lookup', 'catalog'],
  },
  {
    id: 'social',
    label: 'Social & Autosubmit',
    subtitle: 'Tokscale leaderboard account, manual submit, and OS-scheduled autosubmit.',
    keywords: ['social', 'leaderboard', 'whoami', 'github', 'token', 'submit', 'dry-run', 'autosubmit', 'schedule', 'launchd', 'cron', 'interval'],
  },
  {
    id: 'cli',
    label: 'CLI & Advanced',
    subtitle: 'Tokscale binary path, upgrades, timeouts, TUI preferences, and config files.',
    keywords: ['cli', 'binary', 'path', 'install', 'upgrade', 'timeout', 'cache', 'tui', 'palette', 'language', 'minutely', 'settings.json', 'preferences'],
  },
];

export const SEARCHABLE_SETTINGS: readonly SearchableSetting[] = [
  { id: 'theme', section: 'app', label: 'Appearance / Theme', description: 'Light, dark, or system theme across all windows', keywords: ['light', 'dark', 'system', 'color'] },
  { id: 'menubar', section: 'app', label: 'Menubar display', description: 'Show today cost, tokens, or icon-only in menubar', keywords: ['tray', 'cost', 'tokens', 'icon', 'readout'] },
  { id: 'launch', section: 'app', label: 'Launch at login', description: 'Start Tokscale UI automatically on sign in', keywords: ['autostart', 'boot', 'startup'] },
  { id: 'shortcut', section: 'app', label: 'Global shortcut', description: 'Keyboard shortcut to toggle the menubar popover', keywords: ['hotkey', 'keybind', 'recorder'] },
  { id: 'titles', section: 'app', label: 'Session title enrichment', description: 'Free local on-device summarizer for recent sessions', keywords: ['summarizer', 'report', 'apple'] },
  { id: 'notifications', section: 'app', label: 'Quota & budget alerts', description: 'Desktop notifications when near quota or daily budget', keywords: ['notify', 'threshold', 'percent', 'budget', 'usd', 'spend'] },
  { id: 'updates', section: 'app', label: 'App updates & version', description: 'Automatic update checks, release notes, and current app version', keywords: ['update', 'updates', 'version', 'about', 'upgrade', 'release', 'restart'] },
  { id: 'sources-health', section: 'sources', label: 'Detected local harnesses', description: 'Scan path health, message counts, and per-client retry', keywords: ['clients', 'scan', 'retry', 'detected'] },
  { id: 'default-clients', section: 'sources', label: 'Default clients filter', description: 'Default client IDs applied by tokscale CLI', keywords: ['defaultClients', 'filter'] },
  { id: 'extra-paths', section: 'sources', label: 'Extra scan paths', description: 'Additional per-client directories for session logs', keywords: ['extraScanPaths', 'folders', 'codex', 'hermes'] },
  { id: 'opencode-db', section: 'sources', label: 'OpenCode database paths', description: 'Extra OpenCode SQLite database files to include', keywords: ['opencodeDbPaths', 'sqlite', 'db'] },
  { id: 'timezone', section: 'sources', label: 'Bucket timezone & machine label', description: 'IANA timezone for daily bucketing and machine name', keywords: ['bucketTimezone', 'localMachineLabel', 'iana'] },
  { id: 'cursor', section: 'providers', label: 'Cursor accounts & sync', description: 'Switch active Cursor accounts, login, sync, or logout', keywords: ['cursor', 'accounts', 'switch'] },
  { id: 'codex', section: 'providers', label: 'Codex accounts & OAuth', description: 'Import, switch, check quota, or remove Codex accounts', keywords: ['codex', 'openai', 'oauth'] },
  { id: 'trae', section: 'providers', label: 'Trae IDE integration', description: 'Auto-detect Trae desktop credentials and sync usage', keywords: ['trae', 'solo', 'ide'] },
  { id: 'warp', section: 'providers', label: 'Warp / Oz integration', description: 'Save bearer token or cookie header for Warp usage sync', keywords: ['warp', 'oz', 'graphql'] },
  { id: 'capy', section: 'providers', label: 'Capy integration', description: 'Save Capy API key and optional member filter', keywords: ['capy', 'api key'] },
  { id: 'antigravity', section: 'providers', label: 'Antigravity, Amp & Hindsight', description: 'Local daemon, amp CLI thread export, and Hindsight API sync', keywords: ['antigravity', 'amp', 'hindsight', 'purge'] },
  { id: 'price-lookup', section: 'pricing', label: 'Model price lookup & catalog refresh', description: 'Inspect resolved per-1M token pricing or update catalogs', keywords: ['pricing', 'refresh', 'litellm', 'openrouter'] },
  { id: 'custom-pricing', section: 'pricing', label: 'Custom pricing overrides', description: 'Exact model or provider-scoped prices in custom-pricing.json', keywords: ['free', 'override', 'per million', 'validate'] },
  { id: 'model-aliases', section: 'pricing', label: 'Model aliases & unused models', description: 'Map alias names to canonical model IDs', keywords: ['modelAliases', 'includeUnusedModels'] },
  { id: 'social-auth', section: 'social', label: 'Tokscale leaderboard account', description: 'Sign in with GitHub browser auth or personal API token', keywords: ['whoami', 'login', 'logout', 'token'] },
  { id: 'manual-submit', section: 'social', label: 'Submit usage & dry-run', description: 'Preview or submit local usage totals to Tokscale', keywords: ['submit', 'dry-run', 'publish'] },
  { id: 'autosubmit', section: 'social', label: 'Scheduled autosubmit', description: 'Periodic OS-scheduled submission interval and filters', keywords: ['launchd', 'systemd', 'cron', 'interval'] },
  { id: 'cli-binary', section: 'cli', label: 'Tokscale CLI binary & upgrade', description: 'Resolved binary path, custom override, and Bun/npm upgrade', keywords: ['customTokscalePath', 'install', 'upgrade', 'version'] },
  { id: 'cli-advanced', section: 'cli', label: 'Advanced CLI & TUI settings', description: 'Native timeout, light cache write, TUI palette & refresh', keywords: ['nativeTimeoutMs', 'writeCache', 'colorPalette', 'minutelyTabEnabled', 'tuiLanguage'] },
  { id: 'config-files', section: 'cli', label: 'Configuration files on disk', description: 'Reveal ~/.config/tokscale or preferences.json in Finder', keywords: ['finder', 'reveal', 'settings.json', 'preferences.json'] },
];

export function matchSettingsSearch(query: string): SearchableSetting[] {
  const q = query.trim().toLowerCase();
  if (!q) {
    return [];
  }
  const terms = q.split(/\s+/).filter(Boolean);
  return SEARCHABLE_SETTINGS.filter((item) => {
    const haystack = `${item.label} ${item.description} ${item.keywords.join(' ')} ${item.section}`.toLowerCase();
    return terms.every((term) => haystack.includes(term));
  });
}

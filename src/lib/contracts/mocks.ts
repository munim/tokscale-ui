import type {
  AgentPersonaRow,
  ChartBar,
  CliResolutionInfo,
  CompanionPreferences,
  DailyViewRow,
  DashboardQuery,
  DashboardSnapshot,
  GraphCell,
  HarnessSeriesItem,
  HourlyViewRow,
  ModelViewRow,
  MonthlyViewRow,
  ProjectRow,
  ProviderUsageView,
  RangePreset,
  SessionDetail,
  SessionModelBreakdown,
  SessionRow,
  SessionsPayload,
  SessionsQuery,
  SourceHealthRow,
  TokenCounts,
  TopModelSummary,
  TraySnapshot,
} from './ipc';

const MOCK_SEED = 20261121;
const ANCHOR_DATE = '2026-11-21';
const ANCHOR_TIMESTAMP_MS = 1795262400000; // 2026-11-21T12:00:00Z

export function mulberry32(seed: number): () => number {
  let a = seed >>> 0;
  return function next(): number {
    a |= 0;
    a = (a + 0x6d2b79f5) | 0;
    let t = Math.imul(a ^ (a >>> 15), 1 | a);
    t = (t + Math.imul(t ^ (t >>> 7), 61 | t)) ^ t;
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
}

interface HarnessSeedDef {
  id: string;
  label: string;
  color: string;
  monogram: string;
  weight: number;
  provider: string;
  models: Array<{ name: string; provider: string; inputRate: number; outputRate: number }>;
  sessionsPath: string;
  sessionsPathExists: boolean;
  headlessSupported: boolean;
  apiBacked: boolean;
  credentialed: boolean;
  status: 'ok' | 'warn' | 'danger';
  warning?: string;
}

const HARNESS_DEFS: HarnessSeedDef[] = [
  {
    id: 'claude',
    label: 'Claude Code',
    color: 'var(--c-claude)',
    monogram: 'CL',
    weight: 0.36,
    provider: 'anthropic',
    models: [
      { name: 'claude-sonnet-4-5', provider: 'anthropic', inputRate: 3.0, outputRate: 15.0 },
      { name: 'claude-opus-4-1', provider: 'anthropic', inputRate: 15.0, outputRate: 75.0 },
    ],
    sessionsPath: '~/.claude/projects',
    sessionsPathExists: true,
    headlessSupported: true,
    apiBacked: false,
    credentialed: false,
    status: 'ok',
  },
  {
    id: 'codex',
    label: 'Codex',
    color: 'var(--c-codex)',
    monogram: 'CO',
    weight: 0.24,
    provider: 'openai',
    models: [
      { name: 'gpt-5.2-codex', provider: 'openai', inputRate: 2.0, outputRate: 8.0 },
      { name: 'o3-mini', provider: 'openai', inputRate: 1.1, outputRate: 4.4 },
    ],
    sessionsPath: '~/.codex/sessions',
    sessionsPathExists: true,
    headlessSupported: true,
    apiBacked: false,
    credentialed: false,
    status: 'ok',
  },
  {
    id: 'opencode',
    label: 'OpenCode',
    color: 'var(--c-opencode)',
    monogram: 'OP',
    weight: 0.16,
    provider: 'anthropic',
    models: [
      { name: 'claude-sonnet-4-5', provider: 'anthropic', inputRate: 3.0, outputRate: 15.0 },
      { name: 'deepseek-r1', provider: 'deepseek', inputRate: 0.55, outputRate: 2.19 },
    ],
    sessionsPath: '~/.local/share/opencode/storage',
    sessionsPathExists: true,
    headlessSupported: true,
    apiBacked: false,
    credentialed: false,
    status: 'ok',
  },
  {
    id: 'gemini',
    label: 'Gemini CLI',
    color: 'var(--c-gemini)',
    monogram: 'GE',
    weight: 0.12,
    provider: 'google',
    models: [
      { name: 'gemini-2.5-pro', provider: 'google', inputRate: 1.25, outputRate: 5.0 },
      { name: 'gemini-2.5-flash', provider: 'google', inputRate: 0.15, outputRate: 0.6 },
    ],
    sessionsPath: '~/.gemini/tmp',
    sessionsPathExists: true,
    headlessSupported: false,
    apiBacked: false,
    credentialed: false,
    status: 'ok',
  },
  {
    id: 'capy',
    label: 'Capy',
    color: 'oklch(62% 0.12 195)',
    monogram: 'CA',
    weight: 0.07,
    provider: 'capy',
    models: [{ name: 'capy-coder-v2', provider: 'capy', inputRate: 1.5, outputRate: 6.0 }],
    sessionsPath: '~/.config/capy/sessions',
    sessionsPathExists: true,
    headlessSupported: false,
    apiBacked: true,
    credentialed: true,
    status: 'ok',
  },
  {
    id: 'futureclient',
    label: 'futureclient',
    color: 'oklch(62% 0.12 112)',
    monogram: 'FU',
    weight: 0.05,
    provider: 'future',
    models: [{ name: 'future-alpha-1', provider: 'future', inputRate: 1.8, outputRate: 7.2 }],
    sessionsPath: '~/.futureclient/logs',
    sessionsPathExists: true,
    headlessSupported: false,
    apiBacked: false,
    credentialed: false,
    status: 'ok',
  },
];

const EXTRA_SOURCES: SourceHealthRow[] = [
  {
    client: 'cursor',
    label: 'Cursor',
    sessionsPath: '~/Library/Application Support/Cursor/User/workspaceStorage',
    sessionsPathExists: true,
    messageCount: 1420,
    headlessSupported: false,
    headlessMessageCount: 0,
    status: 'ok',
    credentialed: true,
    apiBacked: true,
    lastSyncedAt: ANCHOR_TIMESTAMP_MS - 300_000,
  },
  {
    client: 'trae',
    label: 'Trae',
    sessionsPath: '~/.trae/sessions',
    sessionsPathExists: false,
    messageCount: 0,
    headlessSupported: false,
    headlessMessageCount: 0,
    status: 'warn',
    credentialed: false,
    apiBacked: true,
    warning: 'Not found on this machine',
  },
  {
    client: 'warp',
    label: 'Warp',
    sessionsPath: '~/.warp/ai-history',
    sessionsPathExists: false,
    messageCount: 0,
    headlessSupported: false,
    headlessMessageCount: 0,
    status: 'warn',
    credentialed: false,
    apiBacked: true,
    warning: 'Not signed in',
  },
  {
    client: 'antigravity',
    label: 'Antigravity',
    sessionsPath: '~/.antigravity/sessions',
    sessionsPathExists: false,
    messageCount: 0,
    headlessSupported: false,
    headlessMessageCount: 0,
    status: 'warn',
    credentialed: false,
    apiBacked: true,
    warning: 'Not found on this machine',
  },
];

function parseDateUtc(dateStr: string): number {
  const [y, m, d] = dateStr.split('-').map(Number);
  return Date.UTC(y, m - 1, d);
}

function formatDateUtc(ms: number): string {
  const dt = new Date(ms);
  const y = dt.getUTCFullYear();
  const m = String(dt.getUTCMonth() + 1).padStart(2, '0');
  const d = String(dt.getUTCDate()).padStart(2, '0');
  return `${y}-${m}-${d}`;
}

function shiftDate(dateStr: string, deltaDays: number): string {
  return formatDateUtc(parseDateUtc(dateStr) + deltaDays * 86_400_000);
}

function roundUsd(n: number): number {
  return Math.round(n * 100) / 100;
}

function emptyTokens(): TokenCounts {
  return { input: 0, output: 0, cacheRead: 0, cacheWrite: 0, reasoning: 0 };
}

function addTokens(a: TokenCounts, b: TokenCounts): TokenCounts {
  return {
    input: a.input + b.input,
    output: a.output + b.output,
    cacheRead: a.cacheRead + b.cacheRead,
    cacheWrite: a.cacheWrite + b.cacheWrite,
    reasoning: a.reasoning + b.reasoning,
  };
}

function sumTokenCounts(t: TokenCounts): number {
  return t.input + t.output + t.cacheRead + t.cacheWrite + t.reasoning;
}

interface GeneratedDataset {
  daily: DailyViewRow[];
  hourly: HourlyViewRow[];
  monthly: MonthlyViewRow[];
  modelsByRange: Map<string, ModelViewRow[]>;
  allModels: ModelViewRow[];
  weeks: Array<Array<GraphCell | null>>;
  sessions: SessionRow[];
  projects: ProjectRow[];
  sessionDetails: Map<string, SessionDetail>;
  agents: AgentPersonaRow[];
  sources: SourceHealthRow[];
  usage: ProviderUsageView[];
}

function buildMockDataset(enrichTitles = true): GeneratedDataset {
  const rng = mulberry32(MOCK_SEED);
  const daily: DailyViewRow[] = [];
  const hourly: HourlyViewRow[] = [];
  const monthAccum = new Map<
    string,
    {
      models: Set<string>;
      tokens: TokenCounts;
      messageCount: number;
      sessionCount: number;
      activeDays: number;
      cost: number;
    }
  >();

  const modelAccum = new Map<string, ModelViewRow>();
  const harnessMsgCounts = new Map<string, number>();

  // 365 days ending at ANCHOR_DATE (index 0 = newest, index 364 = oldest)
  for (let dayOffset = 0; dayOffset < 365; dayOffset++) {
    const date = shiftDate(ANCHOR_DATE, -dayOffset);
    const weekday = new Date(parseDateUtc(date)).getUTCDay();
    const isWeekend = weekday === 0 || weekday === 6;
    const activityFactor = isWeekend ? 0.55 + rng() * 0.6 : 0.8 + rng() * 1.1;
    // Ensure a small streak break far in the past, active recent streak
    const quietDay = dayOffset > 25 && dayOffset % 19 === 0;

    let dayTokens = emptyTokens();
    let dayCost = 0;
    let dayMessages = 0;
    let dayTurns = 0;
    const sourceBreakdown: DailyViewRow['sourceBreakdown'] = [];

    if (!quietDay) {
      for (const h of HARNESS_DEFS) {
        if (rng() > 0.92 && dayOffset > 0) {
          continue;
        }
        const scale = h.weight * activityFactor * (0.75 + rng() * 0.5);
        const input = Math.round(12_000 * scale + rng() * 4_000);
        const output = Math.round(3_600 * scale + rng() * 1_200);
        const cacheRead = Math.round(95_000 * scale + rng() * 25_000);
        const cacheWrite = Math.round(8_000 * scale + rng() * 3_000);
        const reasoning = Math.round(2_400 * scale + rng() * 900);
        const hTokens: TokenCounts = { input, output, cacheRead, cacheWrite, reasoning };
        const modelDef = h.models[Math.floor(rng() * h.models.length)];
        const hCost = roundUsd(
          (input * modelDef.inputRate +
            output * modelDef.outputRate +
            cacheRead * modelDef.inputRate * 0.1 +
            cacheWrite * modelDef.inputRate * 0.25 +
            reasoning * modelDef.outputRate * 0.5) /
            1_000_000,
        );
        const hMsgs = Math.max(2, Math.round(18 * scale + rng() * 10));
        const hTurns = Math.max(1, Math.round(hMsgs * 0.45));

        dayTokens = addTokens(dayTokens, hTokens);
        dayCost = roundUsd(dayCost + hCost);
        dayMessages += hMsgs;
        dayTurns += hTurns;
        harnessMsgCounts.set(h.id, (harnessMsgCounts.get(h.id) ?? 0) + hMsgs);

        sourceBreakdown.push({
          client: h.id,
          label: h.label,
          tokens: hTokens,
          cost: hCost,
        });

        const mKey = `${h.id}::${modelDef.name}`;
        const existingModel = modelAccum.get(mKey);
        if (existingModel) {
          existingModel.tokens = addTokens(existingModel.tokens, hTokens);
          existingModel.cost = roundUsd(existingModel.cost + hCost);
          existingModel.messageCount += hMsgs;
          existingModel.sessionCount += 1;
        } else {
          modelAccum.set(mKey, {
            model: modelDef.name,
            colorKey: h.id,
            provider: modelDef.provider,
            client: h.id,
            workspaceKey: '/Users/dev/toksacleui',
            workspaceLabel: 'toksacleui',
            tokens: hTokens,
            cost: hCost,
            sessionCount: 1,
            messageCount: hMsgs,
            msPer1KTokens: Math.round(180 + rng() * 240),
            groupKey: `${h.id}:${modelDef.name}`,
          });
        }

        const monthKey = date.slice(0, 7);
        const mAcc = monthAccum.get(monthKey) ?? {
          models: new Set<string>(),
          tokens: emptyTokens(),
          messageCount: 0,
          sessionCount: 0,
          activeDays: 0,
          cost: 0,
        };
        mAcc.models.add(modelDef.name);
        mAcc.tokens = addTokens(mAcc.tokens, hTokens);
        mAcc.messageCount += hMsgs;
        mAcc.sessionCount += 1;
        mAcc.cost = roundUsd(mAcc.cost + hCost);
        monthAccum.set(monthKey, mAcc);
      }
      const monthKey = date.slice(0, 7);
      const mAcc = monthAccum.get(monthKey);
      if (mAcc) {
        mAcc.activeDays += 1;
      }
    }

    daily.push({
      date,
      tokens: dayTokens,
      cost: dayCost,
      messageCount: dayMessages,
      turnCount: dayTurns,
      sourceBreakdown,
    });

    // Generate 24 hourly rows for the latest 60 days = 1,440 hourly rows
    if (dayOffset < 60) {
      for (let hour = 23; hour >= 0; hour--) {
        const hh = String(hour).padStart(2, '0');
        const datetime = `${date} ${hh}:00`;
        const isActiveHour = hour >= 8 && hour <= 22 && !quietDay;
        const hFactor = isActiveHour ? 0.06 + rng() * 0.09 : 0.004 + rng() * 0.01;
        const input = Math.round(dayTokens.input * hFactor + (isActiveHour ? 500 : 40));
        const output = Math.round(dayTokens.output * hFactor + (isActiveHour ? 180 : 12));
        const cacheRead = Math.round(dayTokens.cacheRead * hFactor + (isActiveHour ? 3_500 : 120));
        const cacheWrite = Math.round(dayTokens.cacheWrite * hFactor + (isActiveHour ? 320 : 15));
        const reasoning = Math.round(dayTokens.reasoning * hFactor + (isActiveHour ? 110 : 5));
        const tokens: TokenCounts = { input, output, cacheRead, cacheWrite, reasoning };
        const cost = roundUsd(Math.max(0.01, dayCost * hFactor));
        const clientCount = isActiveHour ? 1 + Math.floor(rng() * 3) : 1;
        const clients: string[] = [];
        for (let c = 0; c < clientCount; c++) {
          const picked = HARNESS_DEFS[(hour + c + dayOffset) % HARNESS_DEFS.length].id;
          if (!clients.includes(picked)) {
            clients.push(picked);
          }
        }
        const primaryHarness = HARNESS_DEFS.find((d) => d.id === clients[0]) ?? HARNESS_DEFS[0];
        const topModel = primaryHarness.models[0].name;
        const messageCount = Math.max(1, Math.round(dayMessages * hFactor));
        const turnCount = Math.max(1, Math.round(messageCount * 0.5));
        hourly.push({
          datetime,
          tokens,
          cost,
          clients,
          topModel,
          messageCount,
          turnCount,
        });
      }
    }
  }

  // Monthly: keep newest 12 months
  const monthly: MonthlyViewRow[] = Array.from(monthAccum.entries())
    .sort((a, b) => b[0].localeCompare(a[0]))
    .slice(0, 12)
    .map(([month, acc]) => ({
      month,
      models: Array.from(acc.models),
      tokens: acc.tokens,
      messageCount: acc.messageCount,
      sessionCount: acc.sessionCount,
      activeDays: acc.activeDays,
      cost: roundUsd(acc.cost),
    }));

  // 52-week contribution graph (oldest week to newest week, 7 days per week)
  const chronologicalDays = [...daily].reverse();
  const latest52WeeksDays = chronologicalDays.slice(-52 * 7);
  const maxCost = Math.max(0.01, ...latest52WeeksDays.map((d) => d.cost));
  const weeks: Array<Array<GraphCell | null>> = [];
  for (let w = 0; w < 52; w++) {
    const col: Array<GraphCell | null> = [];
    for (let d = 0; d < 7; d++) {
      const idx = w * 7 + d;
      const row = latest52WeeksDays[idx];
      if (!row) {
        col.push(null);
        continue;
      }
      const ratio = row.cost / maxCost;
      const intensity: GraphCell['intensity'] =
        row.cost <= 0 ? 0 : ratio < 0.2 ? 1 : ratio < 0.45 ? 2 : ratio < 0.75 ? 3 : 4;
      col.push({
        date: row.date,
        tokens: row.tokens,
        cost: row.cost,
        intensity,
      });
    }
    weeks.push(col);
  }

  const allModels = Array.from(modelAccum.values()).sort((a, b) => b.cost - a.cost);

  // Sessions & Projects
  const workspaces = [
    { key: '/Users/dev/toksacleui', label: 'toksacleui' },
    { key: '/Users/dev/opencode', label: 'opencode' },
    { key: '/Users/dev/api-gateway', label: 'api-gateway' },
    { key: '/Users/dev/design-tokens', label: 'design-tokens' },
    { key: null, label: '(no workspace)' },
  ];
  const titles = [
    'Implement multi-window Tauri scaffold and IPC contracts',
    'Refactor cache slicing engine for 0ms range updates',
    'Add windowed DataTable and oklch harness badges',
    'Investigate bunx EEXIST symlink lock under concurrent CLI calls',
    'Build 52-week contribution heatmap with keyboard navigation',
    'Wire menubar readout and macOS Dock activation policy',
    'Audit dark theme contrast ratios in tokens.css',
    'Optimize session-to-model join for large report outputs',
  ];
  const categories = ['Feature', 'Refactor', 'Debugging', 'Performance', 'UI Polish'];
  const complexities = ['low', 'medium', 'high'];

  const sessions: SessionRow[] = [];
  const sessionDetails = new Map<string, SessionDetail>();
  const projectMap = new Map<string, ProjectRow>();

  for (let i = 0; i < 24; i++) {
    const harness = HARNESS_DEFS[i % HARNESS_DEFS.length];
    const ws = workspaces[i % workspaces.length];
    const hexId = ((0xa1b2c0 + i * 7919) >>> 0).toString(16).padStart(8, '0');
    const sessionId = `sess-${harness.id}-${hexId}`;
    const shortId = sessionId.slice(-6);
    const enrichedTitle = titles[i % titles.length];
    const title = enrichTitles ? enrichedTitle : undefined;
    const fallbackLabel = `${ws.label} · #${shortId}`;
    const displayLabel = title ?? fallbackLabel;

    const inputTokens = Math.round(14_000 + rng() * 85_000);
    const outputTokens = Math.round(4_200 + rng() * 24_000);
    const cacheReadTokens = Math.round(80_000 + rng() * 420_000);
    const cacheWriteTokens = Math.round(6_000 + rng() * 35_000);
    const reasoningTokens = Math.round(1_500 + rng() * 14_000);
    const totalCost = roundUsd(0.18 + rng() * 6.4);
    const messageCount = Math.round(8 + rng() * 64);
    const durationMinutes = Math.round(4 + rng() * 95);
    const dateStr = shiftDate(ANCHOR_DATE, -(i % 28));
    const hh = String(9 + (i % 11)).padStart(2, '0');
    const mm = String((i * 13) % 60).padStart(2, '0');
    const createdAt = `${dateStr} ${hh}:00`;
    const lastActive = `${dateStr} ${hh}:${mm}`;
    const modelsUsed = harness.models.map((m) => m.name);

    const row: SessionRow = {
      sessionId,
      shortId,
      client: harness.id,
      title,
      displayLabel,
      workspace: ws.key ?? undefined,
      workspaceLabel: ws.label,
      modelsUsed,
      messageCount,
      durationMinutes,
      inputTokens,
      outputTokens,
      cacheReadTokens,
      totalCost,
      createdAt,
      lastActive,
    };
    sessions.push(row);

    const modelsBreakdown: SessionModelBreakdown[] = harness.models.map((m, idx) => {
      const share = idx === 0 ? 0.72 : 0.28;
      return {
        model: m.name,
        provider: m.provider,
        tokens: {
          input: Math.round(inputTokens * share),
          output: Math.round(outputTokens * share),
          cacheRead: Math.round(cacheReadTokens * share),
          cacheWrite: Math.round(cacheWriteTokens * share),
          reasoning: Math.round(reasoningTokens * share),
        },
        cost: roundUsd(totalCost * share),
        messageCount: Math.max(1, Math.round(messageCount * share)),
      };
    });

    sessionDetails.set(`${harness.id}::${sessionId}`, {
      session: row,
      taskCategory: categories[i % categories.length],
      description: `Automated coding session in ${ws.label} using ${harness.label} (${modelsUsed.join(', ')}).`,
      complexity: complexities[i % complexities.length],
      taskGroup: categories[i % categories.length].toLowerCase(),
      totalTokens: {
        input: inputTokens,
        output: outputTokens,
        cacheRead: cacheReadTokens,
        cacheWrite: cacheWriteTokens,
        reasoning: reasoningTokens,
      },
      totalCost,
      models: modelsBreakdown,
    });

    const pKey = ws.key ?? '__none__';
    const existingProj = projectMap.get(pKey);
    const rowTokens = inputTokens + outputTokens + cacheReadTokens + cacheWriteTokens + reasoningTokens;
    if (existingProj) {
      existingProj.sessionCount += 1;
      existingProj.totalTokens += rowTokens;
      existingProj.totalCost = roundUsd(existingProj.totalCost + totalCost);
      for (const m of modelsUsed) {
        if (!existingProj.models.includes(m)) {
          existingProj.models.push(m);
        }
      }
      if (!existingProj.lastActive || lastActive > existingProj.lastActive) {
        existingProj.lastActive = lastActive;
      }
    } else {
      projectMap.set(pKey, {
        workspaceKey: ws.key,
        workspaceLabel: ws.label,
        sessionCount: 1,
        totalTokens: rowTokens,
        totalCost,
        models: [...modelsUsed],
        lastActive,
      });
    }
  }

  const projects = Array.from(projectMap.values()).sort((a, b) => b.totalCost - a.totalCost);

  const agents: AgentPersonaRow[] = [
    {
      agent: 'explore',
      clients: 'claude, opencode',
      tokens: { input: 420_000, output: 95_000, cacheRead: 2_850_000, cacheWrite: 180_000, reasoning: 42_000 },
      cost: 18.42,
      messageCount: 312,
    },
    {
      agent: 'general',
      clients: 'claude, codex, opencode',
      tokens: { input: 310_000, output: 112_000, cacheRead: 1_940_000, cacheWrite: 120_000, reasoning: 64_000 },
      cost: 14.85,
      messageCount: 248,
    },
    {
      agent: 'plan',
      clients: 'claude, gemini',
      tokens: { input: 185_000, output: 68_000, cacheRead: 980_000, cacheWrite: 74_000, reasoning: 92_000 },
      cost: 9.74,
      messageCount: 136,
    },
    {
      agent: 'qa',
      clients: 'codex, opencode',
      tokens: { input: 142_000, output: 49_000, cacheRead: 620_000, cacheWrite: 48_000, reasoning: 28_000 },
      cost: 6.18,
      messageCount: 104,
    },
    {
      agent: 'reviewer',
      clients: 'claude, capy',
      tokens: { input: 96_000, output: 31_000, cacheRead: 410_000, cacheWrite: 22_000, reasoning: 18_000 },
      cost: 3.92,
      messageCount: 72,
    },
  ];

  const sources: SourceHealthRow[] = [
    ...HARNESS_DEFS.map((h) => ({
      client: h.id,
      label: h.label,
      sessionsPath: h.sessionsPath,
      sessionsPathExists: h.sessionsPathExists,
      messageCount: harnessMsgCounts.get(h.id) ?? 0,
      headlessSupported: h.headlessSupported,
      headlessMessageCount: h.headlessSupported ? Math.round((harnessMsgCounts.get(h.id) ?? 0) * 0.25) : 0,
      status: h.status,
      credentialed: h.credentialed,
      apiBacked: h.apiBacked,
      lastSyncedAt: ANCHOR_TIMESTAMP_MS - 120_000,
      warning: h.warning,
    })),
    ...EXTRA_SOURCES,
  ];

  const usage: ProviderUsageView[] = [
    {
      provider: 'Anthropic',
      email: 'dev@tokscale.local',
      plan: 'Max 20x',
      metrics: [
        {
          label: '5-hour session window',
          remainingLabel: '15.4% remaining',
          remainingPercent: 15.4,
          usedPercent: 84.6,
          resetsAt: '2026-11-21T16:30:00Z',
        },
        {
          label: 'Weekly Opus / Sonnet quota',
          remainingLabel: '42.0% remaining',
          remainingPercent: 42.0,
          usedPercent: 58.0,
          resetsAt: '2026-11-25T00:00:00Z',
        },
      ],
    },
    {
      provider: 'OpenAI',
      email: 'dev@tokscale.local',
      plan: 'ChatGPT Pro',
      metrics: [
        {
          label: 'Codex cloud tasks',
          remainingLabel: '68.0% remaining',
          remainingPercent: 68.0,
          usedPercent: 32.0,
          resetsAt: '2026-11-22T00:00:00Z',
        },
      ],
    },
    {
      provider: 'Cursor',
      email: 'dev@tokscale.local',
      plan: 'Pro',
      metrics: [
        {
          label: 'Fast premium requests',
          remainingLabel: '31.0% remaining',
          remainingPercent: 31.0,
          usedPercent: 69.0,
          resetsAt: '2026-12-01T00:00:00Z',
        },
      ],
    },
  ];

  return {
    daily,
    hourly,
    monthly,
    modelsByRange: new Map(),
    allModels,
    weeks,
    sessions,
    projects,
    sessionDetails,
    agents,
    sources,
    usage,
  };
}

function sliceDailyByRange(daily: DailyViewRow[], query: DashboardQuery): DailyViewRow[] {
  if (query.range === 'today') {
    return daily.slice(0, 1);
  }
  if (query.range === '7d') {
    return daily.slice(0, 7);
  }
  if (query.range === '30d') {
    return daily.slice(0, 30);
  }
  if (query.range === 'custom') {
    const since = query.customSince;
    const until = query.customUntil;
    if (since && until && since > until) {
      return [];
    }
    return daily.filter((row) => (!since || row.date >= since) && (!until || row.date <= until));
  }
  return daily;
}

function sliceHourlyByRange(hourly: HourlyViewRow[], query: DashboardQuery): HourlyViewRow[] {
  if (query.range === 'today') {
    return hourly.filter((h) => h.datetime.startsWith(ANCHOR_DATE));
  }
  if (query.range === '7d') {
    const cutoff = shiftDate(ANCHOR_DATE, -6);
    return hourly.filter((h) => h.datetime.slice(0, 10) >= cutoff);
  }
  if (query.range === '30d') {
    const cutoff = shiftDate(ANCHOR_DATE, -29);
    return hourly.filter((h) => h.datetime.slice(0, 10) >= cutoff);
  }
  if (query.range === 'custom') {
    const since = query.customSince;
    const until = query.customUntil;
    if (since && until && since > until) {
      return [];
    }
    return hourly.filter((row) => {
      const d = row.datetime.slice(0, 10);
      return (!since || d >= since) && (!until || d <= until);
    });
  }
  return hourly;
}

function buildHarnessesFromDaily(daily: DailyViewRow[], activeClients: string[]): HarnessSeriesItem[] {
  const totals = new Map<string, { cost: number; tokens: number; sessions: number }>();
  for (const day of daily) {
    for (const sb of day.sourceBreakdown) {
      if (activeClients.length > 0 && !activeClients.includes(sb.client)) {
        continue;
      }
      const prev = totals.get(sb.client) ?? { cost: 0, tokens: 0, sessions: 0 };
      prev.cost = roundUsd(prev.cost + sb.cost);
      prev.tokens += sumTokenCounts(sb.tokens);
      prev.sessions += 1;
      totals.set(sb.client, prev);
    }
  }
  const grandCost = Math.max(
    0.0001,
    Array.from(totals.values()).reduce((sum, item) => sum + item.cost, 0),
  );
  return HARNESS_DEFS.filter((h) => totals.has(h.id))
    .map((h) => {
      const t = totals.get(h.id)!;
      return {
        id: h.id,
        label: h.label,
        color: h.color,
        monogram: h.monogram,
        cost: t.cost,
        tokens: t.tokens,
        sharePercent: Math.round((t.cost / grandCost) * 1000) / 10,
        sessions: t.sessions,
      };
    })
    .sort((a, b) => b.cost - a.cost);
}

function buildBarsFromDaily(daily: DailyViewRow[], activeClients: string[]): ChartBar[] {
  return [...daily]
    .reverse()
    .slice(-30)
    .map((day) => {
      const segments = day.sourceBreakdown
        .filter((sb) => activeClients.length === 0 || activeClients.includes(sb.client))
        .map((sb) => {
          const def = HARNESS_DEFS.find((h) => h.id === sb.client);
          return {
            harnessId: sb.client,
            label: sb.label,
            color: def?.color ?? 'var(--c-other)',
            cost: sb.cost,
            tokens: sumTokenCounts(sb.tokens),
          };
        });
      return {
        slot: day.date,
        label: day.date.slice(5),
        totalCost: roundUsd(segments.reduce((s, seg) => s + seg.cost, 0)),
        totalTokens: segments.reduce((s, seg) => s + seg.tokens, 0),
        segments,
      };
    });
}

function buildBarsForToday(hourly: HourlyViewRow[], activeClients: string[]): ChartBar[] {
  return [...hourly]
    .reverse()
    .map((row) => {
      const filteredClients = row.clients.filter(
        (c) => activeClients.length === 0 || activeClients.includes(c),
      );
      const divisor = Math.max(1, filteredClients.length);
      const rowTokens = sumTokenCounts(row.tokens);
      const segments = filteredClients.map((c) => {
        const def = HARNESS_DEFS.find((h) => h.id === c);
        return {
          harnessId: c,
          label: def?.label ?? c,
          color: def?.color ?? 'var(--c-other)',
          cost: roundUsd(row.cost / divisor),
          tokens: Math.round(rowTokens / divisor),
        };
      });
      return {
        slot: row.datetime,
        label: row.datetime.slice(11),
        totalCost: roundUsd(segments.reduce((s, seg) => s + seg.cost, 0)),
        totalTokens: segments.reduce((s, seg) => s + seg.tokens, 0),
        segments,
      };
    });
}

const DEFAULT_QUERY: DashboardQuery = {
  range: 'all',
  groupBy: 'model',
  activeClients: [],
  sort: 'cost',
};

export function mockTraySnapshot(range: RangePreset = 'today'): TraySnapshot {
  const dataset = buildMockDataset(true);
  const query: DashboardQuery = { ...DEFAULT_QUERY, range };
  const daily = sliceDailyByRange(dataset.daily, query);
  const hourly = sliceHourlyByRange(dataset.hourly, query);
  const harnesses = buildHarnessesFromDaily(daily, []);
  const bars = range === 'today' ? buildBarsForToday(hourly, []) : buildBarsFromDaily(daily, []);
  const totalCost = roundUsd(harnesses.reduce((s, h) => s + h.cost, 0));
  const totalTokens = harnesses.reduce((s, h) => s + h.tokens, 0);
  const totalSessions =
    range === 'today'
      ? Math.max(3, Math.round(harnesses.length * 1.5))
      : harnesses.reduce((s, h) => s + (h.sessions ?? 0), 0);

  const scaleFactor = range === 'today' ? 1 / 365 : range === '7d' ? 7 / 365 : range === '30d' ? 30 / 365 : 1;
  const topModels: TopModelSummary[] = dataset.allModels.slice(0, 3).map((m) => {
    const scaledCost = roundUsd(Math.max(0.02, m.cost * scaleFactor));
    const scaledTokens = Math.max(1000, Math.round(sumTokenCounts(m.tokens) * scaleFactor));
    return {
      model: m.model,
      client: m.client,
      provider: m.provider,
      cost: scaledCost,
      tokens: scaledTokens,
      sharePercent: totalCost > 0 ? Math.round((scaledCost / totalCost) * 1000) / 10 : 0,
    };
  });

  return {
    range,
    total: {
      cost: totalCost,
      tokens: totalTokens,
      sessions: totalSessions,
    },
    bars,
    harnesses,
    topModels,
    sources: dataset.sources,
    syncStatus: {
      syncing: false,
      lastSyncedAt: ANCHOR_TIMESTAMP_MS - 120_000,
      warnings: [],
    },
    cliInfo: mockCliInfo(),
  };
}

export function mockDashboardSnapshot(query: DashboardQuery = DEFAULT_QUERY): DashboardSnapshot {
  const dataset = buildMockDataset(true);
  const activeClients = query.activeClients ?? [];
  const search = query.search?.trim().toLowerCase() ?? '';

  const daily = sliceDailyByRange(dataset.daily, query).map((d) => {
    if (activeClients.length === 0) {
      return d;
    }
    const filteredBreakdown = d.sourceBreakdown.filter((sb) => activeClients.includes(sb.client));
    const tokens = filteredBreakdown.reduce((acc, sb) => addTokens(acc, sb.tokens), emptyTokens());
    const cost = roundUsd(filteredBreakdown.reduce((acc, sb) => acc + sb.cost, 0));
    return { ...d, tokens, cost, sourceBreakdown: filteredBreakdown };
  });

  const hourly = sliceHourlyByRange(dataset.hourly, query).filter(
    (h) => activeClients.length === 0 || h.clients.some((c) => activeClients.includes(c)),
  );

  const harnesses = buildHarnessesFromDaily(daily, activeClients);
  const chart = query.range === 'today' ? buildBarsForToday(hourly, activeClients) : buildBarsFromDaily(daily, activeClients);

  const rangeFactor =
    query.range === 'today' ? 1 / 365 : query.range === '7d' ? 7 / 365 : query.range === '30d' ? 30 / 365 : 1;

  let models = dataset.allModels
    .filter((m) => activeClients.length === 0 || activeClients.includes(m.client))
    .map((m) => {
      if (query.range === 'all') {
        return m;
      }
      return {
        ...m,
        cost: roundUsd(Math.max(0.01, m.cost * rangeFactor)),
        tokens: {
          input: Math.round(m.tokens.input * rangeFactor),
          output: Math.round(m.tokens.output * rangeFactor),
          cacheRead: Math.round(m.tokens.cacheRead * rangeFactor),
          cacheWrite: Math.round(m.tokens.cacheWrite * rangeFactor),
          reasoning: Math.round(m.tokens.reasoning * rangeFactor),
        },
        sessionCount: Math.max(1, Math.round(m.sessionCount * rangeFactor)),
        messageCount: Math.max(1, Math.round(m.messageCount * rangeFactor)),
      };
    });

  if (search) {
    models = models.filter(
      (m) =>
        m.model.toLowerCase().includes(search) ||
        m.client.toLowerCase().includes(search) ||
        m.provider.toLowerCase().includes(search) ||
        (m.workspaceLabel?.toLowerCase().includes(search) ?? false),
    );
  }

  if (query.sort === 'tokens') {
    models.sort((a, b) => sumTokenCounts(b.tokens) - sumTokenCounts(a.tokens));
  } else {
    models.sort((a, b) => b.cost - a.cost);
  }

  const totalCost = roundUsd(daily.reduce((s, d) => s + d.cost, 0));
  const totalTokens = daily.reduce((s, d) => s + sumTokenCounts(d.tokens), 0);
  const activeDays = daily.filter((d) => d.cost > 0).length;
  const costPerMillion = totalTokens > 0 ? roundUsd((totalCost / totalTokens) * 1_000_000) : 0;

  let bestDay: { date: string; cost: number } | undefined;
  for (const d of dataset.daily) {
    if (!bestDay || d.cost > bestDay.cost) {
      bestDay = { date: d.date, cost: d.cost };
    }
  }
  const graphActiveDays = dataset.daily.filter((d) => d.cost > 0).length;
  const graphTotalCost = dataset.daily.reduce((s, d) => s + d.cost, 0);

  return {
    query: { ...query },
    kpis: {
      totalCost,
      totalTokens,
      sessions: Math.max(activeDays, Math.round(dataset.sessions.length * Math.max(0.15, rangeFactor))),
      costPerMillion,
      activeDays,
    },
    chart,
    harnesses,
    models,
    daily,
    hourly,
    monthly: dataset.monthly,
    graph: { weeks: dataset.weeks },
    stats: {
      activeDays: graphActiveDays,
      currentStreak: 26,
      longestStreak: 42,
      bestDay,
      avgCostPerActiveDay: graphActiveDays > 0 ? roundUsd(graphTotalCost / graphActiveDays) : 0,
    },
    usage: dataset.usage,
    agents: dataset.agents,
    sources: {
      headlessRoots: ['~/.claude/projects', '~/.local/share/opencode/storage', '~/.codex/sessions'],
      note: 'Scanning local session logs and credentialed API providers.',
      clients: dataset.sources,
    },
    syncStatus: {
      syncing: false,
      lastSyncedAt: ANCHOR_TIMESTAMP_MS - 120_000,
      warnings:
        query.range === 'custom' && query.customSince && query.customUntil && query.customSince > query.customUntil
          ? [{ source: 'query', message: 'customSince must be on or before customUntil' }]
          : [],
    },
  };
}

export function mockSessionsPayload(query?: SessionsQuery | DashboardQuery): SessionsPayload {
  const enrichTitles = query && 'enrichTitles' in query ? query.enrichTitles : true;
  const dashQuery = query && 'query' in query ? query.query : query;
  const dataset = buildMockDataset(enrichTitles);

  const activeClients = dashQuery?.activeClients ?? [];
  const search = dashQuery?.search?.trim().toLowerCase() ?? '';

  const sessions = dataset.sessions.filter((s) => {
    if (activeClients.length > 0 && !activeClients.includes(s.client)) {
      return false;
    }
    if (search) {
      const hay = `${s.sessionId} ${s.client} ${s.workspaceLabel} ${s.title ?? ''} ${s.modelsUsed.join(' ')}`.toLowerCase();
      if (!hay.includes(search)) {
        return false;
      }
    }
    return true;
  });

  const projects = dataset.projects.filter((p) => {
    if (!search) {
      return true;
    }
    return p.workspaceLabel.toLowerCase().includes(search) || p.models.some((m) => m.toLowerCase().includes(search));
  });

  return { sessions, projects };
}

export function mockSessionDetail(client: string, sessionId: string): SessionDetail {
  const dataset = buildMockDataset(true);
  const found = dataset.sessionDetails.get(`${client}::${sessionId}`);
  if (found) {
    return found;
  }
  const fallbackSession = dataset.sessions.find((s) => s.client === client) ?? dataset.sessions[0];
  return dataset.sessionDetails.get(`${fallbackSession.client}::${fallbackSession.sessionId}`)!;
}

export function mockPreferences(): CompanionPreferences {
  return {
    menubarDisplay: 'cost',
    theme: 'system',
    launchAtLogin: false,
    notificationsEnabled: false,
    quotaAlertPercent: 85,
    sessionTitleEnrichment: true,
  };
}

export function mockCliInfo(): CliResolutionInfo {
  return {
    resolvedVia: 'probe',
    binaryPath: '/Users/dev/.bun/bin/tokscale',
    version: '4.23.0',
  };
}

export function mockTokscaleSettings(): import('./ipc').TokscaleSettingsState {
  return {
    path: '/Users/dev/.config/tokscale/settings.json',
    exists: true,
    configDir: '/Users/dev/.config/tokscale',
    appPreferencesPath: '/Users/dev/Library/Application Support/io.github.munim.tokscale-ui/preferences.json',
    bucketTimezone: 'Europe/Berlin',
    defaultClients: [],
    extraScanPaths: {
      codex: ['/Users/dev/work/monorepo/.codex'],
    },
    opencodeDbPaths: [],
    localMachineLabel: 'macbook-pro',
    includeUnusedModels: false,
    modelAliases: {
      'big-pickle': 'claude-opus-4-6',
    },
    nativeTimeoutMs: 300_000,
    lightWriteCache: false,
    autoRefreshEnabled: true,
    autoRefreshMs: 30_000,
    minutelyTabEnabled: false,
    colorPalette: 'dusk',
    tuiLanguage: 'en',
  };
}

export function mockCustomPricing(): import('./ipc').CustomPricingState {
  return {
    path: '/Users/dev/.config/tokscale/custom-pricing.json',
    exists: true,
    valid: true,
    validationMessage: 'Validated by tokscale CLI',
    entries: [
      {
        scope: 'provider',
        providerId: 'fireworks',
        modelId: 'accounts/fireworks/routers/kimi-k2p6-turbo',
        inputPerMillion: 2.0,
        outputPerMillion: 8.0,
        cacheReadPerMillion: 0.3,
      },
      {
        scope: 'model',
        modelId: 'local/qwen-coder-32b',
        inputPerMillion: 0,
        outputPerMillion: 0,
        cacheReadPerMillion: 0,
        cacheWritePerMillion: 0,
        notes: 'Local inference',
      },
    ],
  };
}

export function mockProvidersStatus(): import('./ipc').ProvidersStatusPayload {
  return {
    providers: [
      {
        id: 'cursor',
        label: 'Cursor',
        connected: true,
        supportsMultiAccount: true,
        summary: '2 saved · active: work',
        lastSyncedAt: '2026-11-22T09:50:00Z',
        accounts: [
          { name: 'work', active: true, email: 'work@tokscale.local', plan: 'Pro' },
          { name: 'personal', active: false, email: 'me@tokscale.local', plan: 'Pro' },
        ],
        diagnostics: [],
        variants: [],
      },
      {
        id: 'codex',
        label: 'Codex',
        connected: true,
        supportsMultiAccount: true,
        summary: '1 saved · active: default',
        accounts: [{ name: 'default', active: true, plan: 'Plus' }],
        diagnostics: [],
        variants: [],
      },
      {
        id: 'trae',
        label: 'Trae IDE',
        connected: false,
        supportsMultiAccount: false,
        summary: 'Not connected',
        accounts: [],
        diagnostics: [],
        variants: [],
      },
      {
        id: 'warp',
        label: 'Warp / Oz',
        connected: false,
        supportsMultiAccount: false,
        summary: 'Not connected',
        accounts: [],
        diagnostics: ['missing credentials; run `tokscale warp login`'],
        variants: [],
      },
      {
        id: 'capy',
        label: 'Capy',
        connected: false,
        supportsMultiAccount: false,
        summary: 'Not connected',
        accounts: [],
        diagnostics: ['missing API key; run `tokscale capy login`'],
        variants: [],
      },
      {
        id: 'antigravity',
        label: 'Antigravity',
        connected: false,
        supportsMultiAccount: false,
        summary: 'No running language servers detected',
        accounts: [],
        diagnostics: [],
        variants: [],
      },
      {
        id: 'amp',
        label: 'Amp',
        connected: false,
        supportsMultiAccount: false,
        summary: 'Exports threads via local amp CLI',
        accounts: [],
        diagnostics: [],
        variants: [],
      },
      {
        id: 'hindsight',
        label: 'Hindsight',
        connected: false,
        supportsMultiAccount: false,
        summary: 'Syncs LLM request logs from Hindsight API',
        accounts: [],
        diagnostics: [],
        variants: [],
      },
    ],
  };
}

export function mockSocialStatus(): import('./ipc').SocialStatusPayload {
  return {
    loggedIn: false,
    detail: 'Not logged in',
    autosubmit: {
      enabled: false,
      intervalMinutes: 1440,
      clients: [],
      today: false,
      yesterday: false,
      week: false,
      month: false,
    },
  };
}

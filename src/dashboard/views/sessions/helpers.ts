import type {
  AgentPersonaRow,
  ProjectRow,
  SessionRow,
  SourceHealthRow,
} from '$lib/contracts/ipc';

const API_BACKED_CLIENTS = new Set(['cursor', 'trae', 'warp', 'capy', 'antigravity']);

export function formatSessionCode(
  session: Pick<SessionRow, 'workspaceLabel' | 'shortId' | 'sessionId'>,
): string {
  const workspace = session.workspaceLabel?.trim() || '(no workspace)';
  const shortId = session.shortId?.trim() || session.sessionId.slice(-6);
  return `${workspace} · #${shortId}`;
}

export function formatSessionTitle(
  session: Pick<SessionRow, 'title' | 'displayLabel' | 'workspaceLabel' | 'shortId' | 'sessionId'>,
): string {
  const title = session.title?.trim();
  if (title) {
    return title;
  }
  const display = session.displayLabel?.trim();
  if (display) {
    return display;
  }
  return formatSessionCode(session);
}

function pad2(n: number): string {
  return String(n).padStart(2, '0');
}

export function formatLocalDateTime(value?: string | null): string {
  if (!value) {
    return '—';
  }
  const trimmed = value.trim();
  if (!trimmed) {
    return '—';
  }

  // Already formatted as local YYYY-MM-DD HH:MM
  if (/^\d{4}-\d{2}-\d{2}\s+\d{2}:\d{2}$/.test(trimmed)) {
    return trimmed.replace(/\s+/, ' ');
  }

  // Date-only YYYY-MM-DD: avoid UTC shift
  if (/^\d{4}-\d{2}-\d{2}$/.test(trimmed)) {
    return `${trimmed} 00:00`;
  }

  // Naive YYYY-MM-DDTHH:MM(:SS)? without timezone offset: treat as local
  const naiveMatch = /^(\d{4})-(\d{2})-(\d{2})[ T](\d{2}):(\d{2})(?::\d{2}(?:\.\d+)?)?$/.exec(
    trimmed,
  );
  if (naiveMatch) {
    return `${naiveMatch[1]}-${naiveMatch[2]}-${naiveMatch[3]} ${naiveMatch[4]}:${naiveMatch[5]}`;
  }

  const parsed = new Date(trimmed);
  if (Number.isNaN(parsed.getTime())) {
    return trimmed;
  }

  const year = parsed.getFullYear();
  const month = pad2(parsed.getMonth() + 1);
  const day = pad2(parsed.getDate());
  const hours = pad2(parsed.getHours());
  const minutes = pad2(parsed.getMinutes());
  return `${year}-${month}-${day} ${hours}:${minutes}`;
}

export function truncateMiddle(value: string, maxLength = 42): string {
  if (!value || value.length <= maxLength) {
    return value;
  }
  if (maxLength <= 3) {
    return value.slice(0, maxLength);
  }
  const keep = maxLength - 1;
  const head = Math.ceil(keep * 0.45);
  const tail = keep - head;
  return `${value.slice(0, head)}…${value.slice(value.length - tail)}`;
}

export function isApiBackedClient(
  row: Pick<SourceHealthRow, 'client' | 'apiBacked'>,
): boolean {
  return row.apiBacked || API_BACKED_CLIENTS.has(row.client.trim().toLowerCase());
}

export type SessionSortKey =
  | 'session'
  | 'title'
  | 'client'
  | 'models'
  | 'messages'
  | 'duration'
  | 'input'
  | 'cacheRead'
  | 'cost';

export function sortSessionRows(
  rows: SessionRow[],
  sortKey: SessionSortKey,
  sortDir: 'asc' | 'desc',
): SessionRow[] {
  const factor = sortDir === 'asc' ? 1 : -1;
  return [...rows].sort((a, b) => {
    let cmp = 0;
    switch (sortKey) {
      case 'session':
        cmp = formatSessionCode(a).localeCompare(formatSessionCode(b));
        break;
      case 'title':
        cmp = formatSessionTitle(a).localeCompare(formatSessionTitle(b));
        break;
      case 'client':
        cmp = a.client.localeCompare(b.client);
        break;
      case 'models':
        cmp = a.modelsUsed.length - b.modelsUsed.length;
        break;
      case 'messages':
        cmp = a.messageCount - b.messageCount;
        break;
      case 'duration':
        cmp = a.durationMinutes - b.durationMinutes;
        break;
      case 'input':
        cmp = a.inputTokens - b.inputTokens;
        break;
      case 'cacheRead':
        cmp = a.cacheReadTokens - b.cacheReadTokens;
        break;
      case 'cost':
      default:
        cmp = a.totalCost - b.totalCost;
        break;
    }
    if (cmp !== 0) {
      return cmp * factor;
    }
    return a.sessionId.localeCompare(b.sessionId);
  });
}

export type ProjectSortKey =
  | 'workspaceLabel'
  | 'sessionCount'
  | 'totalTokens'
  | 'totalCost'
  | 'models'
  | 'lastActive';

export function sortProjectRows(
  rows: ProjectRow[],
  sortKey: ProjectSortKey,
  sortDir: 'asc' | 'desc',
): ProjectRow[] {
  const factor = sortDir === 'asc' ? 1 : -1;
  return [...rows].sort((a, b) => {
    let cmp = 0;
    switch (sortKey) {
      case 'workspaceLabel':
        cmp = (a.workspaceLabel || '(no workspace)').localeCompare(
          b.workspaceLabel || '(no workspace)',
        );
        break;
      case 'sessionCount':
        cmp = a.sessionCount - b.sessionCount;
        break;
      case 'totalTokens':
        cmp = a.totalTokens - b.totalTokens;
        break;
      case 'models':
        cmp = a.models.length - b.models.length;
        break;
      case 'lastActive':
        cmp = (a.lastActive ?? '').localeCompare(b.lastActive ?? '');
        break;
      case 'totalCost':
      default:
        cmp = a.totalCost - b.totalCost;
        break;
    }
    if (cmp !== 0) {
      return cmp * factor;
    }
    return a.workspaceLabel.localeCompare(b.workspaceLabel);
  });
}

export type AgentSortKey = 'agent' | 'clients' | 'messageCount' | 'input' | 'output' | 'cost';

export function sortAgentRows(
  rows: AgentPersonaRow[],
  sortKey: AgentSortKey,
  sortDir: 'asc' | 'desc',
): AgentPersonaRow[] {
  const factor = sortDir === 'asc' ? 1 : -1;
  return [...rows].sort((a, b) => {
    let cmp = 0;
    switch (sortKey) {
      case 'agent':
        cmp = a.agent.localeCompare(b.agent);
        break;
      case 'clients':
        cmp = a.clients.localeCompare(b.clients);
        break;
      case 'messageCount':
        cmp = a.messageCount - b.messageCount;
        break;
      case 'input':
        cmp = a.tokens.input - b.tokens.input;
        break;
      case 'output':
        cmp = a.tokens.output - b.tokens.output;
        break;
      case 'cost':
      default:
        cmp = a.cost - b.cost;
        break;
    }
    if (cmp !== 0) {
      return cmp * factor;
    }
    return a.agent.localeCompare(b.agent);
  });
}

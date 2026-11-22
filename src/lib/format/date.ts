function pad2(n: number): string {
  return String(n).padStart(2, '0');
}

export function localDateKey(d: Date): string {
  const year = d.getFullYear();
  const month = pad2(d.getMonth() + 1);
  const day = pad2(d.getDate());
  return `${year}-${month}-${day}`;
}

export function addDays(key: string, delta: number): string {
  const match = /^(\d{4})-(\d{2})-(\d{2})$/.exec(key.trim());
  if (!match) {
    throw new Error(`Invalid YYYY-MM-DD date key: ${key}`);
  }
  const year = Number(match[1]);
  const month = Number(match[2]);
  const day = Number(match[3]);
  // Use local noon to avoid DST midnight shifts when stepping calendar days.
  const dt = new Date(year, month - 1, day + delta, 12, 0, 0, 0);
  return localDateKey(dt);
}

export function formatRelative(timestampMs: number, now: Date = new Date()): string {
  if (!Number.isFinite(timestampMs) || timestampMs <= 0) {
    return 'just now';
  }
  const normalizedMs = timestampMs < 1e11 ? timestampMs * 1000 : timestampMs;
  const diffMs = now.getTime() - normalizedMs;
  if (diffMs < 45_000) {
    return 'just now';
  }
  const minutes = Math.floor(diffMs / 60_000);
  if (minutes < 1) {
    return 'just now';
  }
  if (minutes < 60) {
    return `${minutes}m ago`;
  }
  const hours = Math.floor(minutes / 60);
  if (hours < 24) {
    return `${hours}h ago`;
  }
  const days = Math.floor(hours / 24);
  return `${days}d ago`;
}

export const formatRelativeTime = formatRelative;

export function formatDuration(minutes: number): string {
  if (!Number.isFinite(minutes) || minutes <= 0) {
    return '0m';
  }
  if (minutes < 1) {
    return '<1m';
  }
  const rounded = Math.round(minutes);
  if (rounded < 60) {
    return `${rounded}m`;
  }
  const hours = Math.floor(rounded / 60);
  const rem = rounded % 60;
  return rem > 0 ? `${hours}h ${rem}m` : `${hours}h`;
}

const WEEKDAY_NAMES = ['Sun', 'Mon', 'Tue', 'Wed', 'Thu', 'Fri', 'Sat'] as const;

export function formatWeekdayShort(key: string): string {
  const match = /^(\d{4})-(\d{2})-(\d{2})$/.exec(key.trim());
  if (!match) {
    return '';
  }
  const dt = new Date(Number(match[1]), Number(match[2]) - 1, Number(match[3]), 12, 0, 0, 0);
  return WEEKDAY_NAMES[dt.getDay()] ?? '';
}

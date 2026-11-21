export function formatCountdown(resetsAt: string, now: Date = new Date()): string {
  const targetMs = Date.parse(resetsAt);
  if (!Number.isFinite(targetMs)) {
    return 'resets now';
  }
  const diffMs = targetMs - now.getTime();
  if (diffMs <= 0) {
    return 'resets now';
  }
  const totalMinutes = Math.floor(diffMs / 60_000);
  if (totalMinutes < 1) {
    return 'resets in <1m';
  }
  if (totalMinutes < 60) {
    return `resets in ${totalMinutes}m`;
  }
  const totalHours = Math.floor(totalMinutes / 60);
  const minutes = totalMinutes % 60;
  if (totalHours < 24) {
    return minutes > 0 ? `resets in ${totalHours}h ${minutes}m` : `resets in ${totalHours}h`;
  }
  const days = Math.floor(totalHours / 24);
  const hours = totalHours % 24;
  return hours > 0 ? `resets in ${days}d ${hours}h` : `resets in ${days}d`;
}

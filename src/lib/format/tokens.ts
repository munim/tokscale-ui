import type { TokenCounts } from '$lib/contracts/ipc';

const integerFormatter = new Intl.NumberFormat('en-US');

export function formatTokens(n: number): string {
  if (!Number.isFinite(n)) {
    return '0';
  }
  const abs = Math.abs(n);
  const sign = n < 0 ? '-' : '';
  if (abs < 1_000) {
    return `${sign}${Math.round(abs)}`;
  }
  if (abs < 1_000_000) {
    return `${sign}${(abs / 1_000).toFixed(1)}K`;
  }
  if (abs < 1_000_000_000) {
    return `${sign}${(abs / 1_000_000).toFixed(1)}M`;
  }
  return `${sign}${(abs / 1_000_000_000).toFixed(1)}B`;
}

export function formatTokensFull(n: number): string {
  if (!Number.isFinite(n)) {
    return '0';
  }
  return integerFormatter.format(Math.round(n));
}

export function sumTokenCounts(counts: TokenCounts): number {
  return (
    counts.input +
    counts.output +
    counts.cacheRead +
    counts.cacheWrite +
    counts.reasoning
  );
}

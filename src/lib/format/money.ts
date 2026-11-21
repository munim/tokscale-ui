const usdFormatter = new Intl.NumberFormat('en-US', {
  minimumFractionDigits: 2,
  maximumFractionDigits: 2,
});

export function formatUsd(n: number): string {
  if (!Number.isFinite(n) || n === 0) {
    return '$0.00';
  }
  const abs = Math.abs(n);
  if (abs > 0 && abs < 0.01) {
    return n < 0 ? '-<$0.01' : '<$0.01';
  }
  const formatted = usdFormatter.format(abs);
  return n < 0 ? `-$${formatted}` : `$${formatted}`;
}

export function formatPercent(n: number, decimals = 1): string {
  if (!Number.isFinite(n)) {
    return `${(0).toFixed(decimals)}%`;
  }
  return `${n.toFixed(decimals)}%`;
}

export function formatCacheMultiple(cachedTokens: number, inputTokens: number): string {
  if (!Number.isFinite(cachedTokens) || !Number.isFinite(inputTokens) || inputTokens <= 0) {
    return '—';
  }
  const ratio = cachedTokens / inputTokens;
  return `${ratio.toFixed(1)}×`;
}

import { describe, expect, it } from 'vitest';
import {
  addDays,
  formatCacheMultiple,
  formatCountdown,
  formatDuration,
  formatPercent,
  formatRelative,
  formatRelativeTime,
  formatTokens,
  formatTokensFull,
  formatUsd,
  localDateKey,
} from './index';

describe('formatters', () => {
  it('formats USD currency values', () => {
    expect(formatUsd(0)).toBe('$0.00');
    expect(formatUsd(48.12)).toBe('$48.12');
    expect(formatUsd(1234.5)).toBe('$1,234.50');
    expect(formatUsd(0.004)).toBe('<$0.01');
    expect(formatUsd(0.01)).toBe('$0.01');
    expect(formatUsd(-12.3)).toBe('-$12.30');
  });

  it('formats compact and full token counts', () => {
    expect(formatTokens(0)).toBe('0');
    expect(formatTokens(999)).toBe('999');
    expect(formatTokens(1_250)).toBe('1.3K');
    expect(formatTokens(61_400_000)).toBe('61.4M');
    expect(formatTokens(2_100_000_000)).toBe('2.1B');
    expect(formatTokensFull(61_400_000)).toBe('61,400,000');
  });

  it('formats percentages and cache multiples', () => {
    expect(formatPercent(84.6)).toBe('84.6%');
    expect(formatPercent(0)).toBe('0.0%');
    expect(formatCacheMultiple(3_400, 1_000)).toBe('3.4×');
    expect(formatCacheMultiple(500, 0)).toBe('—');
  });

  it('formats durations and relative times', () => {
    expect(formatDuration(0)).toBe('0m');
    expect(formatDuration(0.4)).toBe('<1m');
    expect(formatDuration(45)).toBe('45m');
    expect(formatDuration(120)).toBe('2h');
    expect(formatDuration(135)).toBe('2h 15m');

    const now = new Date('2026-11-21T12:00:00Z');
    expect(formatRelative(now.getTime() - 10_000, now)).toBe('just now');
    expect(formatRelative(now.getTime() - 120_000, now)).toBe('2m ago');
    expect(formatRelativeTime(now.getTime() - 3 * 3_600_000, now)).toBe('3h ago');
    expect(formatRelative(now.getTime() - 2 * 86_400_000, now)).toBe('2d ago');

    // Unix seconds timestamps from Tauri backend / tui-data-cache.json
    const nowSecs = Math.floor(now.getTime() / 1000);
    expect(formatRelative(nowSecs - 10, now)).toBe('just now');
    expect(formatRelative(nowSecs - 120, now)).toBe('2m ago');
    expect(formatRelative(nowSecs - 3 * 3_600, now)).toBe('3h ago');
    expect(formatRelative(nowSecs - 2 * 86_400, now)).toBe('2d ago');

    // Non-positive or invalid timestamps
    expect(formatRelative(0, now)).toBe('just now');
    expect(formatRelative(-100, now)).toBe('just now');
    expect(formatRelative(Number.NaN, now)).toBe('just now');
  });

  it('formats countdowns accurately', () => {
    const now = new Date('2026-11-21T12:00:00Z');
    expect(formatCountdown('2026-11-23T16:00:00Z', now)).toBe('resets in 2d 4h');
    expect(formatCountdown('2026-11-21T12:35:00Z', now)).toBe('resets in 35m');
    expect(formatCountdown('2026-11-21T12:00:00Z', now)).toBe('resets now');
    expect(formatCountdown('2026-11-21T11:50:00Z', now)).toBe('resets now');
  });

  it('formats local date keys and shifts calendar days', () => {
    const dt = new Date(2026, 10, 21, 23, 55, 0);
    expect(localDateKey(dt)).toBe('2026-11-21');
    expect(addDays('2026-11-21', -6)).toBe('2026-11-15');
    expect(addDays('2026-11-30', 1)).toBe('2026-12-01');
    expect(addDays('2026-01-01', -1)).toBe('2025-12-31');
  });
});

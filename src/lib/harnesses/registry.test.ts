import '@testing-library/jest-dom/vitest';
import { render, screen } from '@testing-library/svelte';
import { describe, expect, it } from 'vitest';
import HarnessBadge from './HarnessBadge.svelte';
import { harnessColorVar, resolveHarness } from './registry';

describe('harness registry', () => {
  it('matches the exact cross-layer FNV-1a 64-bit known-answer vector for futureclient', () => {
    const meta = resolveHarness('futureclient');
    expect(meta.hue).toBe(77);
    expect(meta.colorLight).toBe('oklch(62% 0.12 77)');
    expect(meta.colorDark).toBe('oklch(74% 0.12 77)');
    expect(meta.monogram).toBe('FU');
    expect(meta.curated).toBe(false);
    expect(harnessColorVar('futureclient')).toBe('oklch(62% 0.12 77)');
    expect(harnessColorVar('futureclient', 'dark')).toBe('oklch(74% 0.12 77)');
  });

  it('resolves curated harnesses with CSS token variables', () => {
    expect(resolveHarness('claude')).toMatchObject({
      id: 'claude',
      label: 'Claude Code',
      colorLight: 'oklch(68% 0.15 50)',
      colorDark: 'oklch(76% 0.14 50)',
      monogram: 'CL',
      curated: true,
    });
    expect(harnessColorVar('claude')).toBe('var(--c-claude)');
    expect(harnessColorVar('codex')).toBe('var(--c-codex)');
    expect(harnessColorVar('opencode')).toBe('var(--c-opencode)');
    expect(harnessColorVar('gemini')).toBe('var(--c-gemini)');
    expect(harnessColorVar('other')).toBe('var(--c-other)');
  });

  it('resolves known non-curated harnesses with hashed colors and icons', () => {
    for (const [id, label] of [
      ['cursor', 'Cursor'],
      ['trae', 'Trae'],
      ['warp', 'Warp'],
      ['capy', 'Capy'],
      ['antigravity', 'Antigravity'],
    ] as const) {
      const meta = resolveHarness(id);
      expect(meta.label).toBe(label);
      expect(meta.curated).toBe(false);
      expect(meta.hasIcon).toBe(true);
      expect(meta.colorLight).toMatch(/^oklch\(62% 0\.12 \d+\)$/);
    }
  });

  it('renders HarnessBadge with SVG icon for curated/known and monogram circle for unknown', () => {
    const { rerender } = render(HarnessBadge, { props: { id: 'claude', size: 24 } });
    const claudeBadge = screen.getByRole('img', { name: 'Claude Code' });
    expect(claudeBadge.querySelector('svg')).not.toBeNull();

    rerender({ id: 'futureclient', size: 16 });
    const futureBadge = screen.getByRole('img', { name: 'futureclient' });
    expect(futureBadge.querySelector('svg')).toBeNull();
    expect(futureBadge).toHaveTextContent('FU');
  });
});

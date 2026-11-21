import { hasHarnessIcon } from './icons';

export interface HarnessMeta {
  id: string;
  label: string;
  hue: number;
  colorLight: string;
  colorDark: string;
  monogram: string;
  curated: boolean;
  hasIcon: boolean;
  cssVar?: string;
}

const FNV_OFFSET_BASIS_64 = 0xcbf29ce484222325n;
const FNV_PRIME_64 = 0x100000001b3n;
const MASK_64 = 0xffffffffffffffffn;
const utf8Encoder = new TextEncoder();

export function fnv1a64(input: string): bigint {
  const bytes = utf8Encoder.encode(input);
  let hash = FNV_OFFSET_BASIS_64;
  for (const byte of bytes) {
    hash ^= BigInt(byte);
    hash = (hash * FNV_PRIME_64) & MASK_64;
  }
  return hash;
}

export function hashToHue(id: string): number {
  return Number(fnv1a64(id) % 360n);
}

export function makeMonogram(label: string): string {
  const alnum = label.replace(/[^a-zA-Z0-9]/g, '').toUpperCase();
  if (alnum.length >= 2) {
    return alnum.slice(0, 2);
  }
  if (alnum.length === 1) {
    return `${alnum}?`;
  }
  return '??';
}

interface CuratedDef {
  label: string;
  hue: number;
  colorLight: string;
  colorDark: string;
  cssVar: string;
}

const CURATED_HARNESSES: Record<string, CuratedDef> = {
  claude: {
    label: 'Claude Code',
    hue: 50,
    colorLight: 'oklch(68% 0.15 50)',
    colorDark: 'oklch(76% 0.14 50)',
    cssVar: '--c-claude',
  },
  codex: {
    label: 'Codex',
    hue: 250,
    colorLight: 'oklch(60% 0.13 250)',
    colorDark: 'oklch(70% 0.13 250)',
    cssVar: '--c-codex',
  },
  opencode: {
    label: 'OpenCode',
    hue: 305,
    colorLight: 'oklch(60% 0.12 305)',
    colorDark: 'oklch(72% 0.13 305)',
    cssVar: '--c-opencode',
  },
  gemini: {
    label: 'Gemini CLI',
    hue: 85,
    colorLight: 'oklch(78% 0.13 85)',
    colorDark: 'oklch(84% 0.12 85)',
    cssVar: '--c-gemini',
  },
  other: {
    label: 'Other',
    hue: 240,
    colorLight: 'oklch(74% 0.02 240)',
    colorDark: 'oklch(70% 0.02 240)',
    cssVar: '--c-other',
  },
};

const KNOWN_LABELS: Record<string, string> = {
  cursor: 'Cursor',
  trae: 'Trae',
  warp: 'Warp',
  capy: 'Capy',
  antigravity: 'Antigravity',
};

export function resolveHarness(id: string): HarnessMeta {
  const normalized = id.trim().toLowerCase();
  const curated = CURATED_HARNESSES[normalized];
  if (curated) {
    return {
      id: normalized,
      label: curated.label,
      hue: curated.hue,
      colorLight: curated.colorLight,
      colorDark: curated.colorDark,
      monogram: makeMonogram(curated.label),
      curated: true,
      hasIcon: hasHarnessIcon(normalized),
      cssVar: curated.cssVar,
    };
  }

  const label = KNOWN_LABELS[normalized] ?? (id.trim() || 'Other');
  const hue = hashToHue(normalized || id);
  return {
    id: normalized || 'other',
    label,
    hue,
    colorLight: `oklch(62% 0.12 ${hue})`,
    colorDark: `oklch(74% 0.12 ${hue})`,
    monogram: makeMonogram(label),
    curated: false,
    hasIcon: hasHarnessIcon(normalized),
  };
}

export function harnessColorVar(id: string, resolvedTheme: 'light' | 'dark' = 'light'): string {
  const meta = resolveHarness(id);
  if (meta.curated && meta.cssVar) {
    return `var(${meta.cssVar})`;
  }
  return resolvedTheme === 'dark' ? meta.colorDark : meta.colorLight;
}

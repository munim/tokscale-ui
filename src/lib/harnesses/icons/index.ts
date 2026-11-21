export interface HarnessIconShape {
  paths: Array<{
    d: string;
    fill?: 'currentColor' | 'none';
    stroke?: 'currentColor' | 'none';
    strokeWidth?: number;
    strokeLinecap?: 'round' | 'square' | 'butt';
    strokeLinejoin?: 'round' | 'miter' | 'bevel';
  }>;
  rects?: Array<{
    x: number;
    y: number;
    width: number;
    height: number;
    rx?: number;
    fill?: 'currentColor' | 'none';
    stroke?: 'currentColor' | 'none';
    strokeWidth?: number;
  }>;
  circles?: Array<{
    cx: number;
    cy: number;
    r: number;
    fill?: 'currentColor' | 'none';
  }>;
}

export const HARNESS_ICONS: Record<string, HarnessIconShape> = {
  claude: {
    paths: [
      {
        d: 'M8 2.2v11.6M2.2 8h11.6M3.9 3.9l8.2 8.2M12.1 3.9l-8.2 8.2',
        fill: 'none',
        stroke: 'currentColor',
        strokeWidth: 1.6,
        strokeLinecap: 'round',
      },
    ],
  },
  codex: {
    rects: [
      {
        x: 2.2,
        y: 2.8,
        width: 11.6,
        height: 10.4,
        rx: 2.2,
        fill: 'none',
        stroke: 'currentColor',
        strokeWidth: 1.5,
      },
    ],
    paths: [
      {
        d: 'M5 6.4l2.1 1.6L5 9.6M8.3 9.8h2.6',
        fill: 'none',
        stroke: 'currentColor',
        strokeWidth: 1.5,
        strokeLinecap: 'round',
        strokeLinejoin: 'round',
      },
    ],
  },
  opencode: {
    paths: [
      {
        d: 'M3.2 4.2h9.6v7.6H3.2z',
        fill: 'none',
        stroke: 'currentColor',
        strokeWidth: 1.5,
        strokeLinejoin: 'round',
      },
      {
        d: 'M6.1 6.7h3.8v2.6H6.1z',
        fill: 'currentColor',
      },
    ],
  },
  gemini: {
    paths: [
      {
        d: 'M8 1.8C8 5.2 10.8 8 14.2 8 10.8 8 8 10.8 8 14.2 8 10.8 5.2 8 1.8 8 5.2 8 8 5.2 8 1.8Z',
        fill: 'currentColor',
      },
    ],
  },
  cursor: {
    paths: [
      {
        d: 'M8 2.1L13.2 5.1v5.8L8 13.9l-5.2-3V5.1L8 2.1Z',
        fill: 'none',
        stroke: 'currentColor',
        strokeWidth: 1.5,
        strokeLinejoin: 'round',
      },
      {
        d: 'M8 8l5.2-3M8 8v5.9M8 8L2.8 5.1',
        fill: 'none',
        stroke: 'currentColor',
        strokeWidth: 1.3,
        strokeLinejoin: 'round',
      },
    ],
  },
  trae: {
    paths: [
      {
        d: 'M3 4.2h10M8 4.2v8.2M5.4 12.4h5.2',
        fill: 'none',
        stroke: 'currentColor',
        strokeWidth: 1.6,
        strokeLinecap: 'round',
      },
    ],
  },
  warp: {
    paths: [
      {
        d: 'M2.5 5.2l2.3 6.2L8 5.8l3.2 5.6 2.3-6.2',
        fill: 'none',
        stroke: 'currentColor',
        strokeWidth: 1.6,
        strokeLinecap: 'round',
        strokeLinejoin: 'round',
      },
    ],
  },
  capy: {
    paths: [
      {
        d: 'M11.6 4.8A4.6 4.6 0 1 0 11.6 11.2',
        fill: 'none',
        stroke: 'currentColor',
        strokeWidth: 1.7,
        strokeLinecap: 'round',
      },
    ],
    circles: [
      {
        cx: 10.2,
        cy: 8,
        r: 1.4,
        fill: 'currentColor',
      },
    ],
  },
  antigravity: {
    paths: [
      {
        d: 'M8 2.4L13 13H10.7L8 7.1 5.3 13H3L8 2.4Z',
        fill: 'currentColor',
      },
    ],
    circles: [
      {
        cx: 8,
        cy: 11.2,
        r: 1.3,
        fill: 'currentColor',
      },
    ],
  },
};

export function hasHarnessIcon(id: string): boolean {
  return Object.prototype.hasOwnProperty.call(HARNESS_ICONS, id.trim().toLowerCase());
}

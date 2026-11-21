import type { Snippet } from 'svelte';

export interface SegmentOption {
  value: string;
  label: string;
  disabled?: boolean;
}

export interface StackedBarSegment {
  id: string;
  value: number;
  color: string;
  label: string;
}

export interface StackedBarItem {
  label: string;
  slot?: string;
  segments: StackedBarSegment[];
}

export interface ColumnDef<Row = unknown> {
  key: string;
  label: string;
  align?: 'start' | 'end';
  width?: string;
  sortable?: boolean;
  numeric?: boolean;
  render?: (row: Row) => string | number;
  cell?: Snippet<[Row]>;
  title?: (row: Row) => string | undefined;
}

export interface QuotaMetricItem {
  label: string;
  usedPercent?: number;
  remainingPercent?: number;
  remainingLabel?: string;
  resetsAt?: string;
}

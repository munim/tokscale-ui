import '@testing-library/jest-dom/vitest';
import { fireEvent, render, screen } from '@testing-library/svelte';
import { describe, expect, it, vi } from 'vitest';
import DataTable from './DataTable.svelte';
import type { ColumnDef } from './types';

interface TestRow {
  id: string;
  group: string;
  name: string;
  cost: number;
}

describe('DataTable', () => {
  const columns: ColumnDef<TestRow>[] = [
    { key: 'name', label: 'Name', sortable: true, render: (r) => r.name },
    { key: 'cost', label: 'Cost', align: 'end', sortable: true, render: (r) => r.cost.toFixed(2) },
  ];

  it('virtualizes 1,400 rows so fewer than 80 DOM rows exist at once', async () => {
    const rows: TestRow[] = Array.from({ length: 1400 }, (_, i) => ({
      id: `row-${i}`,
      group: 'g1',
      name: `Item ${i}`,
      cost: i * 0.25,
    }));

    const { container } = render(DataTable<TestRow>, {
      props: {
        columns,
        rows,
        rowKey: (r) => r.id,
        ariaLabel: 'Hourly table',
      },
    });

    const initialTrs = container.querySelectorAll('tbody tr');
    expect(initialTrs.length).toBeLessThan(60);
    expect(container.querySelectorAll('tbody tr[data-row-key]').length).toBeGreaterThan(10);
    expect(screen.getByText('Item 0')).toBeInTheDocument();
    expect(screen.queryByText('Item 1200')).not.toBeInTheDocument();

    const wrap = container.querySelector('.ui-table-wrap') as HTMLDivElement;
    wrap.scrollTop = 32 * 500;
    await fireEvent.scroll(wrap);

    const scrolledTrs = container.querySelectorAll('tbody tr');
    expect(scrolledTrs.length).toBeLessThan(60);
    expect(screen.getByText('Item 500')).toBeInTheDocument();
    expect(screen.queryByText('Item 0')).not.toBeInTheDocument();
  });

  it('supports sorting and group headers', async () => {
    const onsort = vi.fn();
    const rows: TestRow[] = [
      { id: '1', group: 'Alpha', name: 'Row A', cost: 1.5 },
      { id: '2', group: 'Beta', name: 'Row B', cost: 2.5 },
    ];

    const { container } = render(DataTable<TestRow>, {
      props: {
        columns,
        rows,
        sortKey: 'cost',
        sortDir: 'desc',
        onsort,
        groupBy: (r) => r.group,
        rowKey: (r) => r.id,
      },
    });

    const groupCells = container.querySelectorAll('tr.ui-table-group td');
    expect(groupCells).toHaveLength(2);
    expect(groupCells[0]).toHaveAttribute('colspan', '2');
    expect(groupCells[0]).toHaveTextContent('Alpha');

    const costSortBtn = screen.getByRole('button', { name: /Cost/ });
    await fireEvent.click(costSortBtn);
    expect(onsort).toHaveBeenCalledWith('cost');
  });
});

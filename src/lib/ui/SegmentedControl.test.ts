import '@testing-library/jest-dom/vitest';
import { fireEvent, render, screen } from '@testing-library/svelte';
import { describe, expect, it, vi } from 'vitest';
import SegmentedControl from './SegmentedControl.svelte';

describe('SegmentedControl', () => {
  const options = [
    { value: 'today', label: 'Today' },
    { value: '7d', label: '7D' },
    { value: '30d', label: '30D' },
    { value: 'all', label: 'All' },
  ];

  it('uses radiogroup semantics and roving tabindex', () => {
    const onchange = vi.fn();
    render(SegmentedControl, {
      props: {
        options,
        value: '7d',
        onchange,
        ariaLabel: 'Date range',
      },
    });

    const group = screen.getByRole('radiogroup', { name: 'Date range' });
    expect(group).toBeInTheDocument();

    const radios = screen.getAllByRole('radio');
    expect(radios).toHaveLength(4);
    expect(radios[0]).toHaveAttribute('tabindex', '-1');
    expect(radios[1]).toHaveAttribute('tabindex', '0');
    expect(radios[1]).toHaveAttribute('aria-checked', 'true');
    expect(radios[2]).toHaveAttribute('tabindex', '-1');
  });

  it('moves selection and focus with arrow keys', async () => {
    let currentValue = 'today';
    const onchange = vi.fn((next: string) => {
      currentValue = next;
    });
    const { rerender } = render(SegmentedControl, {
      props: {
        options,
        value: currentValue,
        onchange,
        ariaLabel: 'Date range',
      },
    });

    const radios = screen.getAllByRole('radio');
    radios[0].focus();

    await fireEvent.keyDown(radios[0], { key: 'ArrowRight' });
    expect(onchange).toHaveBeenCalledWith('7d');
    expect(document.activeElement).toBe(radios[1]);
    await rerender({ options, value: currentValue, onchange, ariaLabel: 'Date range' });
    expect(radios[1]).toHaveAttribute('tabindex', '0');

    await fireEvent.keyDown(radios[1], { key: 'ArrowLeft' });
    expect(onchange).toHaveBeenCalledWith('today');
    expect(document.activeElement).toBe(radios[0]);
    await rerender({ options, value: currentValue, onchange, ariaLabel: 'Date range' });
    expect(radios[0]).toHaveAttribute('tabindex', '0');

    await fireEvent.keyDown(radios[0], { key: 'ArrowLeft' });
    expect(onchange).toHaveBeenCalledWith('all');
    expect(document.activeElement).toBe(radios[3]);
  });
});

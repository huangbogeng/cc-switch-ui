import { render, screen } from '@testing-library/react';
import { describe, expect, it } from 'vitest';
import type { ProxyUsageTrend } from '@/api';
import UsageHeatmap from './UsageHeatmap';

function day(day: string, count: number, tokens = 0): ProxyUsageTrend {
  return { day, request_count: count, total_input_tokens: tokens, total_output_tokens: 0 };
}

describe('UsageHeatmap', () => {
  it('shows the empty state when there is no trend data', () => {
    render(<UsageHeatmap trend={[]} loading={false} />);
    expect(screen.getByText(/No activity yet/i)).toBeTruthy();
  });

  it('fills gap days and totals requests across the range', () => {
    const { container } = render(
      <UsageHeatmap
        trend={[day('2026-01-05', 3), day('2026-01-07', 1)]}
        loading={false}
      />,
    );

    // 01-05, 01-06 (gap, zero), 01-07 → three day cells carrying tooltips.
    const cells = container.querySelectorAll('[title]');
    expect(cells.length).toBe(3);

    // Header aggregates: 4 requests total, 2 active days.
    expect(screen.getByText('4')).toBeTruthy();
    expect(screen.getByText('2')).toBeTruthy();
  });

  it('marks the busiest day at full intensity', () => {
    const { container } = render(
      <UsageHeatmap
        trend={[day('2026-01-05', 10), day('2026-01-06', 0)]}
        loading={false}
      />,
    );
    const full = container.querySelector('.bg-primary[title]');
    expect(full?.getAttribute('title')).toContain('10 requests');
  });
});

import { describe, expect, it } from 'vitest';
import { dateInputToTimestamp, resolveUsageRange, toDateInputValue } from './usageRange';

describe('usage range helpers', () => {
  it('resolves a custom range without replacing the supplied values', () => {
    expect(resolveUsageRange({
      preset: 'custom',
      customStartDate: 100,
      customEndDate: 200,
    }, 999_000)).toEqual({ startDate: 100, endDate: 200 });
  });

  it('round-trips a local date and supports end-of-day boundaries', () => {
    const start = dateInputToTimestamp('2026-08-01');
    const end = dateInputToTimestamp('2026-08-01', true);

    expect(start).not.toBeNull();
    expect(end).not.toBeNull();
    expect(toDateInputValue(start!)).toBe('2026-08-01');
    expect(end! - start!).toBe(86_399);
  });

  it('rejects empty date values', () => {
    expect(dateInputToTimestamp('')).toBeNull();
  });
});

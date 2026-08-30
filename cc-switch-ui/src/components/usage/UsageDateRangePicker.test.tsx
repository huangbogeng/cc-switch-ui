import { fireEvent, render, screen } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import UsageDateRangePicker from './UsageDateRangePicker';

describe('UsageDateRangePicker', () => {
  it('submits explicit custom start and end dates', () => {
    const onChange = vi.fn();
    render(<UsageDateRangePicker selection={{ preset: '30d' }} onChange={onChange} />);

    fireEvent.click(screen.getByRole('button', { name: '30d' }));
    fireEvent.change(screen.getByLabelText('Start'), { target: { value: '2026-08-01' } });
    fireEvent.change(screen.getByLabelText('End'), { target: { value: '2026-08-03' } });
    fireEvent.click(screen.getByRole('button', { name: 'Apply custom range' }));

    expect(onChange).toHaveBeenCalledWith(expect.objectContaining({
      preset: 'custom',
      customStartDate: expect.any(Number),
      customEndDate: expect.any(Number),
    }));
  });

  it('rejects a reversed custom range', () => {
    const onChange = vi.fn();
    render(<UsageDateRangePicker selection={{ preset: '30d' }} onChange={onChange} />);

    fireEvent.click(screen.getByRole('button', { name: '30d' }));
    fireEvent.change(screen.getByLabelText('Start'), { target: { value: '2026-08-03' } });
    fireEvent.change(screen.getByLabelText('End'), { target: { value: '2026-08-01' } });
    fireEvent.click(screen.getByRole('button', { name: 'Apply custom range' }));

    expect(screen.getByRole('alert').textContent).toContain('Start date must be before');
    expect(onChange).not.toHaveBeenCalled();
  });
});

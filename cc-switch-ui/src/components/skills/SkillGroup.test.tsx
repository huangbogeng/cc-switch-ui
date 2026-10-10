import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { expect, it, vi } from 'vitest';
import { SkillGroup } from './SkillGroup';
import type { Skill } from '@/api';

it('expands a large collection in batches and exposes accessible collapsed state', async () => {
  const skills: Skill[] = Array.from({ length: 500 }, (_, index) => ({ id: `skill-${index}`, name: `Demo ${index}`, directory: `demo-${index}`, appType: 'claude_code', enabled: true, installedAt: 0 }));
  const onEdit = vi.fn();
  render(<SkillGroup collection="Samples" skills={skills} isSearching={false} onToggle={vi.fn()} onDelete={vi.fn()} onEdit={onEdit} />);
  const group = screen.getByRole('button', { name: /Samples/ });
  expect(group.getAttribute('aria-expanded')).toBe('false');
  expect(screen.queryAllByRole('button', { name: /^Edit Demo/ })).toHaveLength(0);
  await userEvent.click(group);
  expect(group.getAttribute('aria-expanded')).toBe('true');
  expect(screen.getAllByRole('button', { name: /^Edit Demo/ })).toHaveLength(30);
  await userEvent.click(screen.getByRole('button', { name: /Show next 30/ }));
  expect(screen.getAllByRole('button', { name: /^Edit Demo/ })).toHaveLength(60);
  await userEvent.click(screen.getByRole('button', { name: 'Edit Demo 59' }));
  expect(onEdit).toHaveBeenCalledWith(skills[59]);
  await userEvent.click(group);
  expect(screen.queryAllByRole('button', { name: /^Edit Demo/ })).toHaveLength(0);
});

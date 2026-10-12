import { render, screen, fireEvent } from '@testing-library/react';
import { expect, it, vi } from 'vitest';
import { ModelPickerDialog } from './ModelPickerDialog';
const configured = { model:'manual-model',haikuModel:'haiku-old',sonnetModel:'sonnet-old',opusModel:'opus-old' };
it('preserves unlisted current values, cancels without applying and applies only the selected role', async () => {
  const apply = vi.fn(); const close = vi.fn();
  render(<ModelPickerDialog models={[{id:'new-model',ownedBy:'Provider'},{id:'new-model'}]} configured={configured} loading={false} error="" onRefresh={vi.fn()} onApply={apply} onClose={close} />);
  expect(screen.getAllByRole('radio')).toHaveLength(2);
  fireEvent.change(screen.getByLabelText('Use for'), {target:{value:'sonnetModel'}});
  expect(screen.getByLabelText('Model ID')).toHaveProperty('value','sonnet-old');
  fireEvent.change(screen.getByLabelText('Search models'), {target:{value:'new-model'}});
  fireEvent.click(screen.getByRole('radio', {name:/new-model/}));
  fireEvent.click(screen.getByRole('button', {name:'Cancel'})); expect(apply).not.toHaveBeenCalled(); expect(close).toHaveBeenCalledOnce();
  fireEvent.click(screen.getByRole('button', {name:'Apply model'})); expect(apply).toHaveBeenCalledWith('sonnetModel','new-model');
});
it('bounds rendered choices and allows manual IDs when search finds nothing', () => {
  render(<ModelPickerDialog models={Array.from({length:300},(_,i)=>({id:`model-${i}`}))} configured={configured} loading={false} error="Refresh failed" onRefresh={vi.fn()} onApply={vi.fn()} onClose={vi.fn()} />);
  expect(screen.getAllByRole('radio')).toHaveLength(100); expect(screen.getByText(/previous model list/)).toBeTruthy();
  fireEvent.change(screen.getByLabelText('Search models'), {target:{value:'missing'}});
  expect(screen.getByText(/No matching models/)).toBeTruthy();
  fireEvent.change(screen.getByLabelText('Model ID'), {target:{value:'custom'}});
  expect(screen.getByRole('button', {name:'Apply model'})).toHaveProperty('disabled',false);
});

import { useMemo, useState } from 'react';
import { Search } from 'lucide-react';
import type { Provider } from '@/api';
import { Input } from '@/components/ui/input';
import { Button } from '@/components/ui/button';
import { providerAuthLabel, providerBaseUrl } from '@/lib/provider';
import { ProviderCard } from './ProviderCard';
import { Card, CardContent, CardHeader } from '@/components/ui/card';

interface Props {
  providers: Provider[];
  currentProviderId: string | null;
  busy: boolean;
  onSwitch: (id: string) => void;
  onEdit: (provider: Provider) => void;
  onDelete: (id: string) => void;
}

export function ProviderList({ providers, currentProviderId, busy, onSwitch, onEdit, onDelete }: Props) {
  const [search, setSearch] = useState('');
  const filtered = useMemo(() => {
    const query = search.trim().toLocaleLowerCase();
    return providers.filter((provider) => [provider.name, provider.id, providerBaseUrl(provider), providerAuthLabel(provider), provider.notes ?? ''].join(' ').toLocaleLowerCase().includes(query));
  }, [providers, search]);
  return (
    <section aria-label="Provider configurations" className="min-w-0">
      <Card>
        <CardHeader className="flex flex-col gap-3 sm:flex-row sm:items-center sm:justify-between">
          <div>
            <h2 className="text-[15px] font-semibold leading-6">Your providers <span className="ml-1 font-normal text-muted-foreground">{providers.length}</span></h2>
            <p className="mt-1.5 text-xs leading-5 text-muted-foreground">Select a provider for Direct Config.</p>
          </div>
          <div className="relative w-full sm:w-56">
            <Search aria-hidden="true" className="pointer-events-none absolute left-3 top-3 h-4 w-4 text-muted-foreground" />
            <Input type="search" aria-label="Search providers" placeholder="Search providers" value={search} onChange={(event) => setSearch(event.target.value)} className="pl-9" />
          </div>
        </CardHeader>
        <CardContent className="p-0 sm:px-0">
        <div className="divide-y divide-white/[0.07]">
          {filtered.map((provider) => <ProviderCard embedded key={provider.id} provider={provider} active={provider.id === currentProviderId} busy={busy} onSwitch={onSwitch} onEdit={onEdit} onDelete={onDelete} />)}
          {filtered.length === 0 && <div role="status" className="rounded-xl border border-dashed border-white/10 px-5 py-10 text-center">
            <p className="text-sm text-muted-foreground">No providers match your search.</p>
            <Button variant="ghost" size="sm" className="mt-3" onClick={() => setSearch('')}>Clear search</Button>
          </div>}
        </div>
        </CardContent>
      </Card>
    </section>
  );
}

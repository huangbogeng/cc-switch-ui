import { Server, Zap } from 'lucide-react';
import type { Provider } from '@/api';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Card, CardContent, CardHeader, CardTitle, CardDescription } from '@/components/ui/card';
import { Select } from '@/components/ui/select';
import { Label } from '@/components/ui/label';

interface LocalRoutePanelProps {
  providers: Provider[];
  currentProviderId: string | null;
  proxyRunning: boolean;
  recoveryRequired?: boolean;
  statusUnavailable?: boolean;
  proxyTargetId: string | null;
  busy: boolean;
  onTargetChange: (providerId: string) => void;
  onStartProxy: (providerId: string) => void;
  onStopProxy: () => void;
}

export function LocalRoutePanel({
  providers,
  currentProviderId,
  proxyRunning,
  recoveryRequired = false,
  statusUnavailable = false,
  proxyTargetId,
  busy,
  onTargetChange,
  onStartProxy,
  onStopProxy,
}: LocalRoutePanelProps) {
  const currentProvider = providers.find((provider) => provider.id === currentProviderId) ?? null;
  const targetProvider = providers.find((provider) => provider.id === proxyTargetId) ?? null;
  const targetMatchesSelection =
    !proxyRunning || !currentProvider || !targetProvider || currentProvider.id === targetProvider.id;

  return (
    <Card>
      <CardHeader>
        <div className="flex items-center justify-between gap-2">
          <CardTitle className="flex items-center gap-2"><Server className="h-4 w-4 text-muted-foreground" />Local Route</CardTitle>
          <Badge variant="outline" className={recoveryRequired ? 'border-amber-500/20 text-amber-300' : proxyRunning ? 'border-emerald-500/20 text-emerald-400' : 'text-muted-foreground'}>
            {statusUnavailable ? 'Unavailable' : recoveryRequired ? 'Recovery needed' : proxyRunning ? 'Running' : 'Stopped'}
          </Badge>
        </div>
        <CardDescription>Independent from Direct Config.</CardDescription>
      </CardHeader>
      <CardContent className="space-y-5">
        <div className="space-y-1 text-sm">
          <div className="text-xs text-muted-foreground">Direct Config</div>
          <div className="break-words font-medium">{currentProvider?.name ?? 'None'}</div>
        </div>
        <div className="space-y-2">
          <Label htmlFor="route-target">Route Target</Label>
          <Select id="route-target" value={proxyTargetId ?? ''} onChange={(event) => onTargetChange(event.target.value)} disabled={busy || statusUnavailable}>
            <option value="" disabled>Select a provider</option>
            {providers.map((provider) => <option key={provider.id} value={provider.id}>{provider.name}</option>)}
          </Select>
          <p className="text-xs leading-5 text-muted-foreground">This target is independent from the direct Provider selection. Stopping the route restores Direct Config.</p>
        </div>
        {recoveryRequired && <p role="alert" className="text-xs leading-5 text-amber-300">The previous route was interrupted. Restore Direct Config before starting again.</p>}
        {proxyRunning || recoveryRequired ? (
          <Button variant={recoveryRequired ? 'default' : 'outline'} className="w-full" onClick={onStopProxy} disabled={busy || statusUnavailable}>
            {busy ? 'Updating...' : recoveryRequired ? 'Restore Direct Config' : 'Stop Route'}
          </Button>
        ) : (
          <Button className="w-full" onClick={() => proxyTargetId && onStartProxy(proxyTargetId)} disabled={!proxyTargetId || busy || statusUnavailable}>
            <Zap className="h-4 w-4" />{busy ? 'Starting...' : 'Start Route'}
          </Button>
        )}
        {proxyRunning && !targetMatchesSelection && currentProvider && targetProvider && (
          <p role="status" className="border-t border-white/[0.07] pt-4 text-xs leading-5 text-muted-foreground">
            Route is still forwarding to <span className="font-medium text-foreground">{targetProvider.name}</span>. Use Route Target above to change the running route.
          </p>
        )}
      </CardContent>
    </Card>
  );
}

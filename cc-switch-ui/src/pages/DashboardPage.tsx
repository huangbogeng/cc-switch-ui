import { useCallback, useEffect, useState } from 'react';
import {
  getProxyStatus, startProxy, stopProxy,
  getCopilotOAuthStatus, startCopilotOAuth, pollCopilotOAuth,
  getCopilotUsage,
  listProviders, switchProvider, getCurrentProviderId,
  type Provider,
} from '../api';
import type { CopilotAccount, CopilotUsageResponse } from '../api';
import { PageHeader } from '@/components/PageHeader';
import {
  CopilotOAuthModal,
  CurrentProviderCard,
  OAuthStatusCard,
  ProviderGrid,
  ProxyCard,
  UsageCard,
} from '@/components/dashboard/DashboardPanels';

export default function DashboardPage() {
  // Current provider state
  const [currentProviderId, setCurrentProviderId] = useState<string | null>(null);
  const [providers, setProviders] = useState<Record<string, Provider>>({});
  const [loadingProviders, setLoadingProviders] = useState(true);

  // Copilot OAuth state
  const [copilotStatus, setCopilotStatus] = useState<{
    authenticated: boolean;
    accounts: CopilotAccount[];
    default_account_id: string | null;
  } | null>(null);
  const [copilotPending, setCopilotPending] = useState(false);
  const [copilotDeviceCode, setCopilotDeviceCode] = useState('');
  const [copilotUserCode, setCopilotUserCode] = useState('');
  const [copilotVerificationUri, setCopilotVerificationUri] = useState('');

  // Usage state
  const [usage, setUsage] = useState<CopilotUsageResponse | null>(null);

  // Proxy state
  const [proxyStatus, setProxyStatus] = useState<{
    running: boolean;
    listen_addr: string | null;
    upstream_url: string;
  } | null>(null);

  const loadUsage = useCallback(async () => {
    try {
      const data = await getCopilotUsage();
      setUsage(data);
    } catch (e) {
      console.error('Usage error:', e);
    }
  }, []);

  const loadCopilotStatus = useCallback(async () => {
    try {
      const status = await getCopilotOAuthStatus();
      setCopilotStatus(status);
      if (status.authenticated) {
        loadUsage();
      }
    } catch (e) {
      console.error('Copilot status error:', e);
    }
  }, [loadUsage]);

  const loadProxyStatus = useCallback(async () => {
    try {
      const status = await getProxyStatus();
      setProxyStatus(status);
    } catch (e) {
      console.error('Proxy status error:', e);
    }
  }, []);

  const loadProviders = useCallback(async () => {
    try {
      const data = await listProviders();
      setProviders(data.providers);
      const current = await getCurrentProviderId().catch(() => ({ current_provider_id: null }));
      setCurrentProviderId(current.current_provider_id);
    } catch (e) {
      console.error('Providers error:', e);
    } finally {
      setLoadingProviders(false);
    }
  }, []);

  const loadAll = useCallback(async () => {
    await Promise.all([
      loadCopilotStatus(),
      loadProxyStatus(),
      loadProviders(),
    ]);
  }, [loadCopilotStatus, loadProviders, loadProxyStatus]);

  useEffect(() => {
    Promise.resolve().then(loadAll);
    const interval = setInterval(loadAll, 5000);
    return () => clearInterval(interval);
  }, [loadAll]);

  // Handlers
  const handleStartCopilotOAuth = async () => {
    try {
      const data = await startCopilotOAuth();
      setCopilotDeviceCode(data.device_code);
      setCopilotUserCode(data.user_code);
      setCopilotVerificationUri(data.verification_uri);
      setCopilotPending(true);
    } catch (e) {
      console.error('Start Copilot OAuth error:', e);
    }
  };

  const handlePollCopilotOAuth = async () => {
    if (!copilotDeviceCode) return;
    try {
      const data = await pollCopilotOAuth(copilotDeviceCode);
      if (data.success) {
        setCopilotPending(false);
        loadCopilotStatus();
      }
    } catch (e) {
      console.error('Poll Copilot OAuth error:', e);
    }
  };

  const handleSwitchProvider = async (id: string) => {
    try {
      await switchProvider(id);
      setCurrentProviderId(id);
    } catch (e) {
      console.error('Switch provider error:', e);
    }
  };

  const handleToggleProxy = async () => {
    try {
      if (proxyStatus?.running) {
        await stopProxy();
      } else {
        await startProxy();
      }
      loadProxyStatus();
    } catch (e) {
      console.error('Toggle proxy error:', e);
    }
  };

  const currentProvider = currentProviderId ? providers[currentProviderId] : null;
  const providerList = Object.values(providers);

  return (
    <div>
      <PageHeader
        title="Dashboard"
        description="Monitor active routing, OAuth state, local proxy health, and Copilot usage."
      />
      <div className="space-y-6">
        <div className="grid grid-cols-1 gap-4 xl:grid-cols-2">
          <CurrentProviderCard loading={loadingProviders} provider={currentProvider} />
          <OAuthStatusCard
            status={copilotStatus}
            pending={copilotPending}
            onConnect={handleStartCopilotOAuth}
          />
          <ProxyCard status={proxyStatus} onToggle={handleToggleProxy} />
          {usage && <UsageCard usage={usage} />}
        </div>

        <ProviderGrid
          providers={providerList}
          currentProviderId={currentProviderId}
          onSwitch={handleSwitchProvider}
        />
      </div>

      <CopilotOAuthModal
        open={copilotPending}
        verificationUri={copilotVerificationUri}
        userCode={copilotUserCode}
        onAuthorized={handlePollCopilotOAuth}
      />
    </div>
  );
}

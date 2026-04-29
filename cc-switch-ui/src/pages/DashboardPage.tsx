import { useCallback, useEffect, useState } from 'react';
import {
  getCodexOAuthStatus, startCodexOAuth, pollCodexOAuth,
  removeCodexAccount, setDefaultCodexAccount,
  getProxyStatus, startProxy, stopProxy, setProxyTarget,
  getCopilotOAuthStatus, startCopilotOAuth, pollCopilotOAuth,
  getCopilotUsage,
  listProviders, switchProvider, getCurrentProviderId,
  type Provider,
} from '../api';
import type { CodexOAuthStatus, CopilotAccount, CopilotUsageResponse } from '../api';
import { PageHeader } from '@/components/PageHeader';
import {
  CodexOAuthStatusCard,
  CurrentProviderCard,
  DeviceOAuthModal,
  OAuthStatusCard,
  ProviderGrid,
  ProxyCard,
  UsageCard,
} from '@/components/dashboard/DashboardPanels';
import { providerAuthMode, sortProviders } from '@/lib/provider';

export default function DashboardPage() {
  // Current provider state
  const [currentProviderId, setCurrentProviderId] = useState<string | null>(null);
  const [providers, setProviders] = useState<Record<string, Provider>>({});
  const [loadingProviders, setLoadingProviders] = useState(true);

  // Codex OAuth + Proxy state
  const [codexStatus, setCodexStatus] = useState<CodexOAuthStatus | null>(null);
  const [codexPending, setCodexPending] = useState(false);
  const [codexDeviceCode, setCodexDeviceCode] = useState('');
  const [codexUserCode, setCodexUserCode] = useState('');
  const [codexVerificationUri, setCodexVerificationUri] = useState('');

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
    active_target_provider_id: string | null;
    active_target_provider_name: string | null;
  } | null>(null);
  const [proxyError, setProxyError] = useState('');

  const loadUsage = useCallback(async () => {
    try {
      const data = await getCopilotUsage();
      setUsage(data);
    } catch (e) {
      console.error('Usage error:', e);
    }
  }, []);

  const loadCodexStatus = useCallback(async () => {
    try {
      const status = await getCodexOAuthStatus();
      setCodexStatus(status);
    } catch (e) {
      console.error('Codex status error:', e);
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
      loadCodexStatus(),
      loadCopilotStatus(),
      loadProxyStatus(),
      loadProviders(),
    ]);
  }, [loadCodexStatus, loadCopilotStatus, loadProviders, loadProxyStatus]);

  useEffect(() => {
    Promise.resolve().then(loadAll);
    const interval = setInterval(loadAll, 5000);
    return () => clearInterval(interval);
  }, [loadAll]);

  // Handlers
  const handleStartCodexOAuth = async () => {
    try {
      const data = await startCodexOAuth();
      setCodexDeviceCode(data.device_code);
      setCodexUserCode(data.user_code);
      setCodexVerificationUri(data.verification_uri);
      setCodexPending(true);
    } catch (e) {
      console.error('Start Codex OAuth error:', e);
    }
  };

  const handlePollCodexOAuth = async () => {
    if (!codexDeviceCode) return;
    try {
      const data = await pollCodexOAuth(codexDeviceCode);
      if (data.success) {
        setCodexPending(false);
        loadCodexStatus();
      }
    } catch (e) {
      console.error('Poll Codex OAuth error:', e);
    }
  };

  const handleSetDefaultCodexAccount = async (accountId: string) => {
    try {
      await setDefaultCodexAccount(accountId);
      loadCodexStatus();
    } catch (e) {
      console.error('Set default Codex account error:', e);
    }
  };

  const handleRemoveCodexAccount = async (accountId: string) => {
    if (!confirm('Remove this ChatGPT account?')) return;
    try {
      await removeCodexAccount(accountId);
      loadCodexStatus();
    } catch (e) {
      console.error('Remove Codex account error:', e);
    }
  };

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
      if (providers[id] && providerAuthMode(providers[id]) === 'oauth_proxy') {
        await setProxyTarget(id);
        loadProxyStatus();
      }
    } catch (e) {
      console.error('Switch provider error:', e);
    }
  };

  const handleProxyTargetChange = async (id: string) => {
    if (!id) return;
    try {
      setProxyError('');
      const result = await setProxyTarget(id);
      if (!result.success) {
        throw new Error(result.error || 'Failed to set proxy target');
      }
      loadProxyStatus();
    } catch (e) {
      setProxyError(e instanceof Error ? e.message : 'Failed to set proxy target');
      console.error('Set proxy target error:', e);
    }
  };

  const handleToggleProxy = async () => {
    try {
      setProxyError('');
      if (proxyStatus?.running) {
        const result = await stopProxy();
        if (!result.success) throw new Error(result.error || 'Failed to stop proxy');
      } else {
        const result = await startProxy();
        if (!result.success) throw new Error(result.error || 'Failed to start proxy');
      }
      loadProxyStatus();
    } catch (e) {
      setProxyError(e instanceof Error ? e.message : 'Proxy operation failed');
      console.error('Toggle proxy error:', e);
    }
  };

  const currentProvider = currentProviderId ? providers[currentProviderId] : null;
  const providerList = sortProviders(providers);
  const proxyTargetProviders = providerList.filter((provider) => providerAuthMode(provider) === 'oauth_proxy');

  return (
    <div>
      <PageHeader
        title="Dashboard"
        description="Monitor active routing, account OAuth state, local proxy health, and Copilot usage."
      />
      <div className="space-y-6">
        <div className="grid grid-cols-1 gap-4 xl:grid-cols-2">
          <CurrentProviderCard loading={loadingProviders} provider={currentProvider} />
          <CodexOAuthStatusCard
            status={codexStatus}
            pending={codexPending}
            onConnect={handleStartCodexOAuth}
            onSetDefault={handleSetDefaultCodexAccount}
            onRemove={handleRemoveCodexAccount}
          />
          <OAuthStatusCard
            title="Copilot OAuth"
            providerName="GitHub Copilot"
            status={copilotStatus}
            pending={copilotPending}
            onConnect={handleStartCopilotOAuth}
          />
          <ProxyCard
            status={proxyStatus}
            targetProviders={proxyTargetProviders}
            error={proxyError}
            onToggle={handleToggleProxy}
            onTargetChange={handleProxyTargetChange}
          />
          {usage && <UsageCard usage={usage} />}
        </div>

        <ProviderGrid
          providers={providerList}
          currentProviderId={currentProviderId}
          onSwitch={handleSwitchProvider}
        />
      </div>

      <DeviceOAuthModal
        open={codexPending}
        title="Connect ChatGPT Codex"
        verificationUri={codexVerificationUri}
        userCode={codexUserCode}
        onAuthorized={handlePollCodexOAuth}
      />

      <DeviceOAuthModal
        open={copilotPending}
        title="Connect GitHub Copilot"
        verificationUri={copilotVerificationUri}
        userCode={copilotUserCode}
        onAuthorized={handlePollCopilotOAuth}
      />
    </div>
  );
}

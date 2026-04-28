import { useEffect, useState } from 'react';
import {
  getOAuthStatus, startOAuth, pollOAuth,
  getProxyStatus, startProxy, stopProxy,
  getCopilotOAuthStatus, startCopilotOAuth, pollCopilotOAuth,
  getCopilotUsage,
} from '../api';
import type { CopilotAccount, CopilotUsageResponse } from '../api';

export default function DashboardPage() {
  // Codex OAuth state
  const [oauthStatus, setOauthStatus] = useState<{ authenticated: boolean; accounts: { id: string; login: string; is_default: boolean }[] } | null>(null);
  const [oauthPending, setOauthPending] = useState(false);
  const [oauthError, setOauthError] = useState('');
  const [deviceCode, setDeviceCode] = useState('');
  const [userCode, setUserCode] = useState('');
  const [verificationUri, setVerificationUri] = useState('');

  // Copilot OAuth state
  const [copilotStatus, setCopilotStatus] = useState<{ authenticated: boolean; accounts: CopilotAccount[]; default_account_id: string | null } | null>(null);
  const [copilotPending, setCopilotPending] = useState(false);
  const [copilotError, setCopilotError] = useState('');
  const [copilotDeviceCode, setCopilotDeviceCode] = useState('');
  const [copilotUserCode, setCopilotUserCode] = useState('');
  const [copilotVerificationUri, setCopilotVerificationUri] = useState('');

  // Usage state
  const [usage, setUsage] = useState<CopilotUsageResponse | null>(null);
  const [usageError, setUsageError] = useState('');

  // Proxy state
  const [proxyStatus, setProxyStatus] = useState<{ running: boolean; listen_addr: string | null; upstream_url: string } | null>(null);
  const [proxyError, setProxyError] = useState('');

  // Load OAuth status
  const loadOAuthStatus = async () => {
    try {
      const status = await getOAuthStatus();
      setOauthStatus(status);
    } catch (e) {
      console.error('OAuth status error:', e);
    }
  };

  // Load Copilot OAuth status
  const loadCopilotOAuthStatus = async () => {
    try {
      const status = await getCopilotOAuthStatus();
      setCopilotStatus(status);
      if (status.authenticated) {
        loadCopilotUsage();
      }
    } catch (e) {
      console.error('Copilot OAuth status error:', e);
    }
  };

  // Load Copilot usage
  const loadCopilotUsage = async () => {
    try {
      const data = await getCopilotUsage();
      setUsage(data);
      setUsageError('');
    } catch (e) {
      setUsageError(e instanceof Error ? e.message : 'Failed to load usage');
    }
  };

  // Load proxy status
  const loadProxyStatus = async () => {
    try {
      const status = await getProxyStatus();
      setProxyStatus(status);
    } catch (e) {
      console.error('Proxy status error:', e);
    }
  };

  useEffect(() => {
    loadOAuthStatus();
    loadCopilotOAuthStatus();
    loadProxyStatus();
    const interval = setInterval(() => {
      loadOAuthStatus();
      loadCopilotOAuthStatus();
      loadProxyStatus();
    }, 5000);
    return () => clearInterval(interval);
  }, []);

  // Codex OAuth handlers
  const handleStartOAuth = async () => {
    setOauthError('');
    try {
      const data = await startOAuth();
      setDeviceCode(data.device_code);
      setUserCode(data.user_code);
      setVerificationUri(data.verification_uri);
      setOauthPending(true);
    } catch (e) {
      setOauthError(e instanceof Error ? e.message : 'Failed to start OAuth');
    }
  };

  const handlePollOAuth = async () => {
    if (!deviceCode) return;
    try {
      const data = await pollOAuth(deviceCode);
      if (data.success) {
        setOauthPending(false);
        loadOAuthStatus();
      } else if (data.pending) {
        setOauthError('Authorization not complete. Please complete in browser and try again.');
      }
    } catch (e) {
      setOauthError(e instanceof Error ? e.message : 'Poll failed');
    }
  };

  // Copilot OAuth handlers
  const handleStartCopilotOAuth = async () => {
    setCopilotError('');
    try {
      const data = await startCopilotOAuth();
      setCopilotDeviceCode(data.device_code);
      setCopilotUserCode(data.user_code);
      setCopilotVerificationUri(data.verification_uri);
      setCopilotPending(true);
    } catch (e) {
      setCopilotError(e instanceof Error ? e.message : 'Failed to start OAuth');
    }
  };

  const handlePollCopilotOAuth = async () => {
    if (!copilotDeviceCode) return;
    try {
      const data = await pollCopilotOAuth(copilotDeviceCode);
      if (data.success) {
        setCopilotPending(false);
        loadCopilotOAuthStatus();
      } else if (data.error && !data.error.includes('pending')) {
        setCopilotError(data.error);
      }
    } catch (e) {
      setCopilotError(e instanceof Error ? e.message : 'Poll failed');
    }
  };

  // Proxy handlers
  const handleStartProxy = async () => {
    setProxyError('');
    try {
      const data = await startProxy();
      if (!data.success) {
        setProxyError(data.error || 'Failed to start');
      }
      loadProxyStatus();
    } catch (e) {
      setProxyError(e instanceof Error ? e.message : 'Failed to start proxy');
    }
  };

  const handleStopProxy = async () => {
    setProxyError('');
    try {
      const data = await stopProxy();
      if (!data.success) {
        setProxyError(data.error || 'Failed to stop');
      }
      loadProxyStatus();
    } catch (e) {
      setProxyError(e instanceof Error ? e.message : 'Failed to stop proxy');
    }
  };

  // Render quota bar
  const renderQuotaBar = (remaining: number, total: number, unlimited: boolean) => {
    if (unlimited) {
      return <span style={styles.unlimited}>Unlimited</span>;
    }
    const percent = total > 0 ? (remaining / total) * 100 : 0;
    const color = percent < 10 ? '#e74c3c' : percent < 30 ? '#f39c12' : '#2ecc71';
    return (
      <div style={styles.quotaBarContainer}>
        <div style={{ ...styles.quotaBarFill, width: `${percent}%`, background: color }} />
        <span style={styles.quotaText}>{remaining.toLocaleString()} / {total.toLocaleString()}</span>
      </div>
    );
  };

  return (
    <div style={styles.container}>
      <h1 style={styles.header}>CC Switch Dashboard</h1>

      {/* Copilot OAuth Card */}
      <div style={styles.card}>
        <h2 style={styles.cardTitle}>GitHub Copilot</h2>
        {copilotError && <div style={styles.error}>{copilotError}</div>}

        {!copilotPending ? (
          copilotStatus?.authenticated ? (
            <div>
              <span style={styles.badgeSuccess}>Authenticated</span>
              <div style={styles.accountList}>
                {copilotStatus.accounts.map((a) => (
                  <div key={a.id} style={styles.account}>
                    {a.avatar_url && <img src={a.avatar_url} alt="" style={styles.avatar} />}
                    <span>{a.login}</span>
                    {a.id === copilotStatus.default_account_id && <span style={styles.defaultBadge}>default</span>}
                  </div>
                ))}
              </div>

              {/* Usage Display */}
              {usage && (
                <div style={styles.usageSection}>
                  <h3 style={styles.usageTitle}>Usage (Resets: {usage.quota_reset_date})</h3>
                  <div style={styles.planBadge}>{usage.copilot_plan}</div>
                  <div style={styles.quotaGrid}>
                    <div style={styles.quotaItem}>
                      <span style={styles.quotaLabel}>Chat</span>
                      {renderQuotaBar(usage.quota_snapshots.chat.remaining, usage.quota_snapshots.chat.entitlement, usage.quota_snapshots.chat.unlimited)}
                    </div>
                    <div style={styles.quotaItem}>
                      <span style={styles.quotaLabel}>Completions</span>
                      {renderQuotaBar(usage.quota_snapshots.completions.remaining, usage.quota_snapshots.completions.entitlement, usage.quota_snapshots.completions.unlimited)}
                    </div>
                  </div>
                </div>
              )}
              {usageError && <div style={styles.usageError}>{usageError}</div>}

              <button style={styles.buttonSecondary} onClick={handleStartCopilotOAuth}>Re-authenticate</button>
              <button style={styles.buttonSecondary} onClick={loadCopilotUsage}>Refresh Usage</button>
            </div>
          ) : (
            <div>
              <p style={styles.info}>Not authenticated. Start OAuth to use Copilot as provider.</p>
              <button style={styles.buttonPrimary} onClick={handleStartCopilotOAuth}>Start Copilot OAuth</button>
            </div>
          )
        ) : (
          <div style={styles.oauthBox}>
            <p>Visit URL and enter code:</p>
            <a href={copilotVerificationUri} target="_blank" rel="noopener noreferrer" style={styles.oauthUrl}>
              {copilotVerificationUri}
            </a>
            <div style={styles.oauthCode}>{copilotUserCode}</div>
            <button style={styles.buttonSuccess} onClick={handlePollCopilotOAuth}>I've Authorized</button>
          </div>
        )}
      </div>

      {/* Codex OAuth Card */}
      <div style={styles.card}>
        <h2 style={styles.cardTitle}>OpenAI Codex OAuth</h2>
        {oauthError && <div style={styles.error}>{oauthError}</div>}

        {!oauthPending ? (
          oauthStatus?.authenticated ? (
            <div>
              <span style={styles.badgeSuccess}>Authenticated</span>
              <div style={styles.accountList}>
                {oauthStatus.accounts.map((a) => (
                  <div key={a.id} style={styles.account}>
                    <span>{a.login}</span>
                    {a.is_default && <span style={styles.defaultBadge}>default</span>}
                  </div>
                ))}
              </div>
              <button style={styles.buttonSecondary} onClick={handleStartOAuth}>Re-authenticate</button>
            </div>
          ) : (
            <div>
              <p style={styles.info}>Not authenticated. Start OAuth to use Codex/ChatGPT as provider.</p>
              <button style={styles.buttonPrimary} onClick={handleStartOAuth}>Start OAuth Login</button>
            </div>
          )
        ) : (
          <div style={styles.oauthBox}>
            <p>Visit URL and enter code:</p>
            <a href={verificationUri} target="_blank" rel="noopener noreferrer" style={styles.oauthUrl}>
              {verificationUri}
            </a>
            <div style={styles.oauthCode}>{userCode}</div>
            <button style={styles.buttonSuccess} onClick={handlePollOAuth}>I've Authorized</button>
          </div>
        )}
      </div>

      {/* Proxy Card */}
      <div style={styles.card}>
        <h2 style={styles.cardTitle}>Proxy Server</h2>
        {proxyError && <div style={styles.error}>{proxyError}</div>}

        {proxyStatus?.running ? (
          <div>
            <span style={styles.badgeSuccess}>Running</span>
            <p style={styles.info}>Proxy URL: <code>{proxyStatus.listen_addr}/v1/chat/completions</code></p>
            <button style={{ ...styles.button, background: '#e74c3c', color: '#fff' }} onClick={handleStopProxy}>Stop Proxy</button>
          </div>
        ) : (
          <div>
            <span style={styles.badgePending}>Stopped</span>
            <p style={styles.info}>Start the proxy to route Claude Code requests through your OpenAI account.</p>
            <button style={styles.buttonPrimary} onClick={handleStartProxy}>Start Proxy</button>
          </div>
        )}
      </div>
    </div>
  );
}

const styles: Record<string, React.CSSProperties> = {
  container: { padding: '40px', maxWidth: '800px', margin: '0 auto', background: '#1a1a2e', minHeight: '100vh', color: '#eee', fontFamily: '-apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif' },
  header: { color: '#00d4ff', marginBottom: '30px' },
  card: { background: '#16213e', borderRadius: '8px', padding: '24px', marginBottom: '20px', border: '1px solid #0f3460' },
  cardTitle: { color: '#00d4ff', marginBottom: '15px', fontSize: '1.2em' },
  badgeSuccess: { display: 'inline-block', padding: '4px 12px', borderRadius: '20px', fontSize: '0.85em', background: '#2ecc71', color: '#fff' },
  badgePending: { display: 'inline-block', padding: '4px 12px', borderRadius: '20px', fontSize: '0.85em', background: '#f39c12', color: '#1a1a2e' },
  defaultBadge: { fontSize: '0.75em', color: '#2ecc71', marginLeft: '8px' },
  account: { display: 'flex', alignItems: 'center', gap: '10px', padding: '12px', background: '#0f3460', borderRadius: '6px', marginTop: '10px' },
  avatar: { width: '24px', height: '24px', borderRadius: '50%' },
  accountList: { marginTop: '15px' },
  info: { color: '#888', marginBottom: '15px', lineHeight: 1.6 },
  error: { color: '#e74c3c', padding: '12px', background: 'rgba(231, 76, 60, 0.1)', borderRadius: '6px', marginBottom: '15px' },
  oauthBox: { textAlign: 'center' as const },
  oauthUrl: { color: '#00d4ff', textDecoration: 'none', fontSize: '1.1em', display: 'block', margin: '10px 0' },
  oauthCode: { fontSize: '2.5em', letterSpacing: '4px', background: '#16213e', padding: '20px', borderRadius: '6px', margin: '15px 0', fontFamily: 'monospace', color: '#00d4ff' },
  button: { padding: '10px 20px', border: 'none', borderRadius: '6px', cursor: 'pointer', fontWeight: '500', marginRight: '10px' },
  buttonPrimary: { padding: '10px 20px', background: '#00d4ff', border: 'none', borderRadius: '6px', cursor: 'pointer', fontWeight: '500', color: '#1a1a2e' },
  buttonSuccess: { padding: '10px 20px', background: '#2ecc71', border: 'none', borderRadius: '6px', cursor: 'pointer', fontWeight: '500', color: '#fff' },
  buttonSecondary: { padding: '10px 20px', background: 'transparent', border: '1px solid #0f3460', borderRadius: '6px', cursor: 'pointer', color: '#888', marginTop: '15px', marginRight: '10px' },
  usageSection: { marginTop: '20px', paddingTop: '20px', borderTop: '1px solid #0f3460' },
  usageTitle: { color: '#888', fontSize: '0.9em', marginBottom: '10px' },
  planBadge: { display: 'inline-block', padding: '2px 8px', borderRadius: '4px', fontSize: '0.8em', background: '#0f3460', color: '#00d4ff', marginBottom: '15px' },
  quotaGrid: { display: 'grid', gap: '12px' },
  quotaItem: { display: 'flex', flexDirection: 'column', gap: '4px' },
  quotaLabel: { color: '#888', fontSize: '0.85em' },
  quotaBarContainer: { position: 'relative', height: '20px', background: '#0f3460', borderRadius: '4px', overflow: 'hidden' },
  quotaBarFill: { position: 'absolute', top: 0, left: 0, height: '100%', transition: 'width 0.3s' },
  quotaText: { position: 'absolute', top: '50%', left: '50%', transform: 'translate(-50%, -50%)', fontSize: '0.75em', color: '#fff', textShadow: '0 1px 2px rgba(0,0,0,0.5)' },
  usageError: { color: '#f39c12', fontSize: '0.85em', marginTop: '10px' },
  unlimited: { color: '#2ecc71', fontSize: '0.9em' },
};

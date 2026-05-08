import { useState, useEffect, useCallback, useRef } from 'react';
import {
  getCodexOAuthStatus, startCodexOAuth, pollCodexOAuth,
  removeCodexAccount, setDefaultCodexAccount,
  getCopilotOAuthStatus, startCopilotOAuth, pollCopilotOAuth,
  type CodexAccount,
} from '@/api';
import { PageHeader } from '@/components/PageHeader';
import { Button } from '@/components/ui/button';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Check, CheckCircle2, Circle, Loader2, Server, Trash2 } from 'lucide-react';

interface CopilotStatus {
  authenticated: boolean;
  accounts: { id: string; login: string; avatar_url: string | null; github_domain: string }[];
  default_account_id: string | null;
}

interface CodexStatus {
  authenticated: boolean;
  accounts: CodexAccount[];
}

export default function OAuthPage() {
  // Codex OAuth state
  const [codexStatus, setCodexStatus] = useState<CodexStatus | null>(null);
  const [codexPending, setCodexPending] = useState(false);
  const [codexDeviceCode, setCodexDeviceCode] = useState('');
  const [codexUserCode, setCodexUserCode] = useState('');
  const [codexVerificationUri, setCodexVerificationUri] = useState('');

  // Copilot OAuth state
  const [copilotStatus, setCopilotStatus] = useState<CopilotStatus | null>(null);
  const [copilotPending, setCopilotPending] = useState(false);
  const [copilotDeviceCode, setCopilotDeviceCode] = useState('');
  const [copilotUserCode, setCopilotUserCode] = useState('');
  const [copilotVerificationUri, setCopilotVerificationUri] = useState('');

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
    } catch (e) {
      console.error('Copilot status error:', e);
    }
  }, []);

  const loadAll = useCallback(async () => {
    await Promise.all([
      loadCodexStatus(),
      loadCopilotStatus(),
    ]);
  }, [loadCodexStatus, loadCopilotStatus]);

  useEffect(() => {
    Promise.resolve().then(loadAll);
  }, [loadAll]);

  // Codex OAuth handlers
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

  const closeCodexOAuthModal = () => {
    setCodexPending(false);
    setCodexDeviceCode('');
    setCodexUserCode('');
    setCodexVerificationUri('');
  };

  const handlePollCodexOAuth = async (): Promise<boolean> => {
    if (!codexDeviceCode) return false;
    try {
      const data = await pollCodexOAuth(codexDeviceCode);
      if (data.success) {
        closeCodexOAuthModal();
        loadCodexStatus();
        return true;
      }
    } catch (e) {
      console.error('Poll Codex OAuth error:', e);
    }
    return false;
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

  // Copilot OAuth handlers
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

  const closeCopilotOAuthModal = () => {
    setCopilotPending(false);
    setCopilotDeviceCode('');
    setCopilotUserCode('');
    setCopilotVerificationUri('');
  };

  const handlePollCopilotOAuth = async (): Promise<boolean> => {
    if (!copilotDeviceCode) return false;
    try {
      const data = await pollCopilotOAuth(copilotDeviceCode);
      if (data.success) {
        closeCopilotOAuthModal();
        loadCopilotStatus();
        return true;
      }
    } catch (e) {
      console.error('Poll Copilot OAuth error:', e);
    }
    return false;
  };

  return (
    <div>
      <PageHeader
        title="OAuth"
        description="Manage account connections used by OAuth-backed routes."
      />

      <div className="space-y-6">
        {/* Codex OAuth Section */}
        <Card className="border-white/5 bg-card/40 hover:bg-card/60 transition-colors duration-300">
          <CardHeader className="border-b border-white/5 bg-black/10 pb-4">
            <CardTitle className="flex items-center gap-2.5 text-sm font-semibold text-muted-foreground tracking-tight">
              <div className="rounded-md bg-white/5 p-1.5 shadow-inner">
                <Server className="h-4 w-4 text-primary" />
              </div>
              ChatGPT Codex OAuth
            </CardTitle>
          </CardHeader>
          <CardContent className="space-y-4 pt-5">
            <div className="grid grid-cols-[minmax(0,1fr)_auto] items-center gap-4 rounded-2xl bg-black/20 border border-white/5 p-4 shadow-inner">
              <div className="grid min-w-0 grid-cols-[40px_minmax(0,1fr)] items-center gap-4">
                {codexStatus?.authenticated ? (
                  <div className="flex h-10 w-10 items-center justify-center rounded-xl bg-emerald-500/20 border border-emerald-500/20 shadow-[0_0_15px_rgba(16,185,129,0.15)]">
                    <CheckCircle2 className="h-5 w-5 text-emerald-400 drop-shadow-sm" />
                  </div>
                ) : (
                  <div className="flex h-10 w-10 items-center justify-center rounded-xl bg-white/5 border border-white/5">
                    <Circle className="h-5 w-5 text-muted-foreground/50" />
                  </div>
                )}
                <div className="min-w-0">
                  <div className="truncate text-[15px] font-bold tracking-tight leading-5">ChatGPT Codex</div>
                  <div className="mt-1 truncate text-[11px] font-medium uppercase tracking-wider text-muted-foreground/80">
                    {codexStatus?.authenticated
                      ? codexStatus.accounts.find((a) => a.is_default)?.login || codexStatus.accounts[0]?.login || 'Connected'
                      : 'OAuth route provider only'}
                  </div>
                </div>
              </div>
              <Button size="sm" variant={codexStatus?.authenticated ? 'secondary' : 'default'} onClick={handleStartCodexOAuth} disabled={codexPending} className="rounded-xl px-4">
                {codexStatus?.authenticated ? 'Reconnect' : 'Connect'}
              </Button>
            </div>

            {codexStatus?.accounts && codexStatus.accounts.length > 0 && (
              <div className="space-y-2.5 pt-2">
                <div className="text-[11px] font-bold uppercase tracking-wider text-muted-foreground/60 px-1">Linked Accounts</div>
                {codexStatus.accounts.map((account) => (
                  <div key={account.id} className="grid grid-cols-[minmax(0,1fr)_auto_auto] items-center gap-3 rounded-xl bg-white/[0.02] border border-white/5 px-4 py-3 transition-colors hover:bg-white/[0.04]">
                    <div className="min-w-0">
                      <div className="truncate text-[13px] font-semibold leading-5 text-foreground">{account.login}</div>
                      <div className="truncate font-mono text-[10px] leading-4 text-muted-foreground/60">{account.id}</div>
                    </div>
                    <Button
                      size="sm"
                      variant={account.is_default ? 'default' : 'secondary'}
                      onClick={() => handleSetDefaultCodexAccount(account.id)}
                      disabled={account.is_default}
                      className={`h-8 rounded-lg text-xs font-medium px-3 ${account.is_default ? 'bg-primary/20 text-primary hover:bg-primary/30 cursor-default opacity-100' : ''}`}
                    >
                      {account.is_default ? 'Default' : 'Set Default'}
                    </Button>
                    <Button size="sm" variant="ghost" onClick={() => handleRemoveCodexAccount(account.id)} className="h-8 w-8 p-0 rounded-lg text-destructive/70 hover:text-destructive hover:bg-destructive/10">
                      <Trash2 className="h-3.5 w-3.5" />
                    </Button>
                  </div>
                ))}
              </div>
            )}
          </CardContent>
        </Card>

        {/* Copilot OAuth Section */}
        <Card className="border-white/5 bg-card/40 hover:bg-card/60 transition-colors duration-300">
          <CardHeader className="border-b border-white/5 bg-black/10 pb-4">
            <CardTitle className="flex items-center gap-2.5 text-sm font-semibold text-muted-foreground tracking-tight">
              <div className="rounded-md bg-white/5 p-1.5 shadow-inner">
                <Server className="h-4 w-4 text-primary" />
              </div>
              GitHub Copilot OAuth
            </CardTitle>
          </CardHeader>
          <CardContent className="space-y-4 pt-5">
            <div className="grid grid-cols-[minmax(0,1fr)_auto] items-center gap-4 rounded-2xl bg-black/20 border border-white/5 p-4 shadow-inner">
              <div className="grid min-w-0 grid-cols-[40px_minmax(0,1fr)] items-center gap-4">
                {copilotStatus?.authenticated ? (
                  <div className="flex h-10 w-10 items-center justify-center rounded-xl bg-emerald-500/20 border border-emerald-500/20 shadow-[0_0_15px_rgba(16,185,129,0.15)]">
                    <CheckCircle2 className="h-5 w-5 text-emerald-400 drop-shadow-sm" />
                  </div>
                ) : (
                  <div className="flex h-10 w-10 items-center justify-center rounded-xl bg-white/5 border border-white/5">
                    <Circle className="h-5 w-5 text-muted-foreground/50" />
                  </div>
                )}
                <div className="min-w-0">
                  <div className="truncate text-[15px] font-bold tracking-tight leading-5">GitHub Copilot</div>
                  <div className="mt-1 truncate text-[11px] font-medium uppercase tracking-wider text-muted-foreground/80">
                    {copilotStatus?.authenticated
                      ? copilotStatus.accounts.find((a) => a.id === copilotStatus.default_account_id)?.login || 'Connected'
                      : 'Not connected'}
                  </div>
                </div>
              </div>
              <Button size="sm" variant={copilotStatus?.authenticated ? 'secondary' : 'default'} onClick={handleStartCopilotOAuth} disabled={copilotPending} className="rounded-xl px-4">
                {copilotStatus?.authenticated ? 'Reconnect' : 'Connect'}
              </Button>
            </div>

            {copilotStatus?.accounts && copilotStatus.accounts.length > 0 && (
              <div className="space-y-2.5 pt-2">
                <div className="text-[11px] font-bold uppercase tracking-wider text-muted-foreground/60 px-1">Linked Accounts</div>
                {copilotStatus.accounts.map((account) => (
                  <div key={account.id} className="grid grid-cols-[minmax(0,1fr)_auto_auto] items-center gap-3 rounded-xl bg-white/[0.02] border border-white/5 px-4 py-3 transition-colors hover:bg-white/[0.04]">
                    <div className="min-w-0">
                      <div className="truncate text-[13px] font-semibold leading-5 text-foreground">{account.login}</div>
                      <div className="truncate font-mono text-[10px] leading-4 text-muted-foreground/60">{account.github_domain}</div>
                    </div>
                  </div>
                ))}
              </div>
            )}
          </CardContent>
        </Card>

      </div>

      {/* Device OAuth Modals */}
      {codexPending && (
        <DeviceOAuthModal
          title="Connect ChatGPT Codex"
          verificationUri={codexVerificationUri}
          userCode={codexUserCode}
          onAuthorized={handlePollCodexOAuth}
          onClose={closeCodexOAuthModal}
        />
      )}

      {copilotPending && (
        <DeviceOAuthModal
          title="Connect GitHub Copilot"
          verificationUri={copilotVerificationUri}
          userCode={copilotUserCode}
          onAuthorized={handlePollCopilotOAuth}
          onClose={closeCopilotOAuthModal}
        />
      )}
    </div>
  );
}

function DeviceOAuthModal({
  title,
  verificationUri,
  userCode,
  onAuthorized,
  onClose,
}: {
  title: string;
  verificationUri: string;
  userCode: string;
  onAuthorized: () => boolean | Promise<boolean>;
  onClose: () => void;
}) {
  const [checking, setChecking] = useState(false);
  const [pollError, setPollError] = useState('');
  const checkingRef = useRef(false);
  const onAuthorizedRef = useRef(onAuthorized);

  useEffect(() => {
    onAuthorizedRef.current = onAuthorized;
  }, [onAuthorized]);

  const checkAuthorization = useCallback(async () => {
    if (checkingRef.current) return;
    checkingRef.current = true;
    setChecking(true);
    setPollError('');
    try {
      const authorized = await onAuthorizedRef.current();
      if (authorized) {
        return;
      }
    } catch (e) {
      setPollError(e instanceof Error ? e.message : 'Authorization check failed');
    } finally {
      checkingRef.current = false;
      setChecking(false);
    }
  }, []);

  useEffect(() => {
    const timer = window.setInterval(() => {
      void checkAuthorization();
    }, 3000);
    const initialTimer = window.setTimeout(() => {
      void checkAuthorization();
    }, 0);
    return () => {
      window.clearInterval(timer);
      window.clearTimeout(initialTimer);
    };
  }, [checkAuthorization]);

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/55 px-4 backdrop-blur-sm">
      <div className="w-full max-w-md rounded-3xl border border-white/10 bg-card/95 p-6 shadow-2xl shadow-black/30">
        <div className="mb-4 flex items-center justify-between">
          <h2 className="text-xl font-semibold">{title}</h2>
          <button onClick={onClose} className="text-muted-foreground hover:text-foreground">
            ×
          </button>
        </div>
        <p className="mb-4 text-sm text-muted-foreground">Visit the URL and enter the code to authorize</p>
        <a
          href={verificationUri}
          target="_blank"
          rel="noopener noreferrer"
          className="mb-4 flex min-w-0 items-center gap-2 text-primary hover:underline"
        >
          {verificationUri}
        </a>
        <div className="mb-6 text-center">
          <div className="rounded-2xl bg-secondary/50 py-6 font-mono text-5xl font-bold tracking-widest text-primary">
            {userCode}
          </div>
        </div>
        {pollError && <p className="mb-3 text-sm text-destructive">{pollError}</p>}
        <Button onClick={() => void checkAuthorization()} className="w-full" disabled={checking}>
          {checking ? (
            <>
              <Loader2 className="mr-2 h-4 w-4 animate-spin" />
              Checking authorization
            </>
          ) : (
            <>
              <Check className="mr-2 h-4 w-4" />
              Check now
            </>
          )}
        </Button>
      </div>
    </div>
  );
}

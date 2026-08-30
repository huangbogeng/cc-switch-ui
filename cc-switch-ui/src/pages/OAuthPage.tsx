import { useState, useEffect, useCallback, useRef } from 'react';
import {
  getCodexOAuthStatus, startCodexOAuth, pollCodexOAuth,
  removeCodexAccount, setDefaultCodexAccount,
  getCopilotOAuthStatus, startCopilotOAuth, pollCopilotOAuth,
  removeCopilotAccount, setDefaultCopilotAccount,
  type CodexAccount,
} from '@/api';
import { cacheGet, cacheSet } from '@/lib/fetchCache';
import { PageHeader } from '@/components/PageHeader';
import { Button } from '@/components/ui/button';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Check, CheckCircle2, Circle, Loader2, Server, Trash2 } from 'lucide-react';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { ErrorAlert } from '@/components/ErrorAlert';
import { errorMessage } from '@/lib/errors';
import { useDialog } from '@/lib/useDialog';

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
  const cachedCodex = cacheGet<CodexStatus>('oauth-codex');
  const cachedCopilot = cacheGet<CopilotStatus>('oauth-copilot');

  // Codex OAuth state
  const [codexStatus, setCodexStatus] = useState<CodexStatus | null>(cachedCodex ?? null);
  const [codexPending, setCodexPending] = useState(false);
  const [codexDeviceCode, setCodexDeviceCode] = useState('');
  const [codexUserCode, setCodexUserCode] = useState('');
  const [codexVerificationUri, setCodexVerificationUri] = useState('');
  const [codexExpiresAt, setCodexExpiresAt] = useState(0);

  // Copilot OAuth state
  const [copilotStatus, setCopilotStatus] = useState<CopilotStatus | null>(cachedCopilot ?? null);
  const [copilotPending, setCopilotPending] = useState(false);
  const [copilotDeviceCode, setCopilotDeviceCode] = useState('');
  const [copilotUserCode, setCopilotUserCode] = useState('');
  const [copilotVerificationUri, setCopilotVerificationUri] = useState('');
  const [copilotExpiresAt, setCopilotExpiresAt] = useState(0);
  const [githubDomain, setGithubDomain] = useState('github.com');
  const [error, setError] = useState('');
  const [pendingAction, setPendingAction] = useState<string | null>(null);

  const loadCodexStatus = useCallback(async (signal?: AbortSignal) => {
    try {
      const status = await getCodexOAuthStatus({ signal });
      setCodexStatus(status);
      cacheSet('oauth-codex', status);
    } catch (e) {
      if (e instanceof DOMException && e.name === 'AbortError') return;
      setError(errorMessage(e, 'Failed to load ChatGPT accounts'));
    }
  }, []);

  const loadCopilotStatus = useCallback(async (signal?: AbortSignal) => {
    try {
      const status = await getCopilotOAuthStatus({ signal });
      setCopilotStatus(status);
      cacheSet('oauth-copilot', status);
    } catch (e) {
      if (e instanceof DOMException && e.name === 'AbortError') return;
      setError(errorMessage(e, 'Failed to load Copilot accounts'));
    }
  }, []);

  const loadAll = useCallback(async (signal?: AbortSignal) => {
    setError('');
    await Promise.all([
      loadCodexStatus(signal),
      loadCopilotStatus(signal),
    ]);
  }, [loadCodexStatus, loadCopilotStatus]);

  useEffect(() => {
    const ctrl = new AbortController();
    Promise.resolve().then(() => loadAll(ctrl.signal));
    return () => ctrl.abort();
  }, [loadAll]);

  // Codex OAuth handlers
  const handleStartCodexOAuth = async () => {
    try {
      setPendingAction('codex-connect');
      setError('');
      const data = await startCodexOAuth();
      setCodexDeviceCode(data.device_code);
      setCodexUserCode(data.user_code);
      setCodexVerificationUri(data.verification_uri);
      setCodexExpiresAt(Date.now() + data.expires_in * 1000);
      setCodexPending(true);
    } catch (e) {
      setError(errorMessage(e, 'Failed to start ChatGPT authorization'));
    } finally {
      setPendingAction(null);
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
    const data = await pollCodexOAuth(codexDeviceCode);
    if (data.success) {
      closeCodexOAuthModal();
      await loadCodexStatus();
      return true;
    }
    if (data.error && !data.pending) throw new Error(data.error);
    return false;
  };

  const handleSetDefaultCodexAccount = async (accountId: string) => {
    try {
      setPendingAction(`codex-default:${accountId}`);
      setError('');
      await setDefaultCodexAccount(accountId);
      await loadCodexStatus();
    } catch (e) {
      setError(errorMessage(e, 'Failed to set the default ChatGPT account'));
    } finally {
      setPendingAction(null);
    }
  };

  const handleRemoveCodexAccount = async (accountId: string) => {
    if (!confirm('Remove this ChatGPT account?')) return;
    try {
      setPendingAction(`codex-remove:${accountId}`);
      setError('');
      await removeCodexAccount(accountId);
      await loadCodexStatus();
    } catch (e) {
      setError(errorMessage(e, 'Failed to remove the ChatGPT account'));
    } finally {
      setPendingAction(null);
    }
  };

  // Copilot OAuth handlers
  const handleStartCopilotOAuth = async () => {
    try {
      setPendingAction('copilot-connect');
      setError('');
      const domain = githubDomain.trim() || 'github.com';
      const data = await startCopilotOAuth(domain);
      setCopilotDeviceCode(data.device_code);
      setCopilotUserCode(data.user_code);
      setCopilotVerificationUri(data.verification_uri);
      setCopilotExpiresAt(Date.now() + data.expires_in * 1000);
      setCopilotPending(true);
    } catch (e) {
      setError(errorMessage(e, 'Failed to start Copilot authorization'));
    } finally {
      setPendingAction(null);
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
    const data = await pollCopilotOAuth(copilotDeviceCode, githubDomain.trim() || 'github.com');
    if (data.success) {
      closeCopilotOAuthModal();
      await loadCopilotStatus();
      return true;
    }
    if (data.error) throw new Error(data.error);
    return false;
  };

  const handleSetDefaultCopilotAccount = async (accountId: string) => {
    try {
      setPendingAction(`copilot-default:${accountId}`);
      setError('');
      await setDefaultCopilotAccount(accountId);
      await loadCopilotStatus();
    } catch (e) {
      setError(errorMessage(e, 'Failed to set the default Copilot account'));
    } finally {
      setPendingAction(null);
    }
  };

  const handleRemoveCopilotAccount = async (accountId: string) => {
    if (!confirm('Remove this GitHub Copilot account?')) return;
    try {
      setPendingAction(`copilot-remove:${accountId}`);
      setError('');
      await removeCopilotAccount(accountId);
      await loadCopilotStatus();
    } catch (e) {
      setError(errorMessage(e, 'Failed to remove the Copilot account'));
    } finally {
      setPendingAction(null);
    }
  };

  return (
    <div>
      <PageHeader
        title="OAuth"
        description="Manage account connections used by OAuth-backed routes."
      />

      {error && <ErrorAlert message={error} className="mb-6" />}

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
              <Button size="sm" variant={codexStatus?.authenticated ? 'secondary' : 'default'} onClick={handleStartCodexOAuth} disabled={codexPending || pendingAction !== null} className="rounded-xl px-4">
                {pendingAction === 'codex-connect' ? 'Connecting...' : codexStatus?.authenticated ? 'Reconnect' : 'Connect'}
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
                      disabled={account.is_default || pendingAction !== null}
                      className={`h-8 rounded-lg text-xs font-medium px-3 ${account.is_default ? 'bg-primary/20 text-primary hover:bg-primary/30 cursor-default opacity-100' : ''}`}
                    >
                      {account.is_default ? 'Default' : 'Set Default'}
                    </Button>
                    <Button aria-label={`Remove ${account.login}`} size="sm" variant="ghost" disabled={pendingAction !== null} onClick={() => handleRemoveCodexAccount(account.id)} className="h-8 w-8 p-0 rounded-lg text-destructive/70 hover:text-destructive hover:bg-destructive/10">
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
            <div className="space-y-2">
              <Label htmlFor="github-domain">GitHub domain</Label>
              <Input
                id="github-domain"
                value={githubDomain}
                onChange={(event) => setGithubDomain(event.target.value)}
                placeholder="github.com or your GHES host"
                disabled={copilotPending || pendingAction !== null}
              />
            </div>
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
              <Button size="sm" variant={copilotStatus?.authenticated ? 'secondary' : 'default'} onClick={handleStartCopilotOAuth} disabled={copilotPending || pendingAction !== null} className="rounded-xl px-4">
                {pendingAction === 'copilot-connect' ? 'Connecting...' : copilotStatus?.authenticated ? 'Reconnect' : 'Connect'}
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
                    <Button
                      size="sm"
                      variant={account.id === copilotStatus.default_account_id ? 'default' : 'secondary'}
                      disabled={account.id === copilotStatus.default_account_id || pendingAction !== null}
                      onClick={() => handleSetDefaultCopilotAccount(account.id)}
                    >
                      {account.id === copilotStatus.default_account_id ? 'Default' : 'Set Default'}
                    </Button>
                    <Button
                      aria-label={`Remove ${account.login}`}
                      size="icon"
                      variant="ghost"
                      disabled={pendingAction !== null}
                      onClick={() => handleRemoveCopilotAccount(account.id)}
                    >
                      <Trash2 className="h-3.5 w-3.5 text-destructive" />
                    </Button>
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
          expiresAt={codexExpiresAt}
          onClose={closeCodexOAuthModal}
        />
      )}

      {copilotPending && (
        <DeviceOAuthModal
          title="Connect GitHub Copilot"
          verificationUri={copilotVerificationUri}
          userCode={copilotUserCode}
          onAuthorized={handlePollCopilotOAuth}
          expiresAt={copilotExpiresAt}
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
  expiresAt,
  onClose,
}: {
  title: string;
  verificationUri: string;
  userCode: string;
  onAuthorized: () => boolean | Promise<boolean>;
  expiresAt: number;
  onClose: () => void;
}) {
  const [checking, setChecking] = useState(false);
  const [pollError, setPollError] = useState('');
  const checkingRef = useRef(false);
  const onAuthorizedRef = useRef(onAuthorized);
  const dialogRef = useDialog(true, onClose);

  useEffect(() => {
    onAuthorizedRef.current = onAuthorized;
  }, [onAuthorized]);

  const checkAuthorization = useCallback(async () => {
    if (checkingRef.current) return;
    checkingRef.current = true;
    setChecking(true);
    setPollError('');
    try {
      if (Date.now() >= expiresAt) {
        throw new Error('This device code has expired. Close this dialog and start again.');
      }
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
  }, [expiresAt]);

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
      <div ref={dialogRef} tabIndex={-1} role="dialog" aria-modal="true" aria-labelledby="oauth-dialog-title" className="w-full max-w-md rounded-3xl border border-white/10 bg-card/95 p-6 shadow-2xl shadow-black/30">
        <div className="mb-4 flex items-center justify-between">
          <h2 id="oauth-dialog-title" className="text-xl font-semibold">{title}</h2>
          <button aria-label="Close authorization dialog" onClick={onClose} className="text-muted-foreground hover:text-foreground">
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

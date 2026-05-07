const API_BASE = '/api';

let authToken = localStorage.getItem('ccswitch_token') || '';

export function setAuthToken(token: string) {
  authToken = token;
  localStorage.setItem('ccswitch_token', token);
}

export function clearAuthToken() {
  authToken = '';
  localStorage.removeItem('ccswitch_token');
}

async function api<T>(path: string, options: RequestInit = {}): Promise<T> {
  const headers: Record<string, string> = {
    'Content-Type': 'application/json',
    ...(options.headers as Record<string, string> || {}),
  };
  if (authToken) {
    headers['Authorization'] = `Bearer ${authToken}`;
  }

  const res = await fetch(`${API_BASE}${path}`, { ...options, headers });

  if (res.status === 401) {
    clearAuthToken();
    window.location.reload();
    throw new Error('Unauthorized');
  }

  if (!res.ok) {
    const body = await res.json().catch(() => ({}));
    throw new Error((body as { error?: string }).error || `HTTP ${res.status}`);
  }

  return res.json() as Promise<T>;
}

// Auth
export async function login(token: string) {
  return api<{ success: boolean; message: string }>('/auth/login', {
    method: 'POST',
    body: JSON.stringify({ token }),
  });
}

// Codex OAuth
export interface CodexAccount {
  id: string;
  login: string;
  is_default: boolean;
}

export interface CodexOAuthStatus {
  authenticated: boolean;
  accounts: CodexAccount[];
}

export async function getCodexOAuthStatus() {
  return api<CodexOAuthStatus>('/codex/oauth/status');
}

export async function startCodexOAuth() {
  return api<{
    device_code: string;
    user_code: string;
    verification_uri: string;
    expires_in: number;
    interval: number;
  }>('/codex/oauth/start', { method: 'POST' });
}

export async function pollCodexOAuth(device_code: string) {
  return api<{
    success?: boolean;
    pending?: boolean;
    account?: CodexAccount;
    error?: string;
  }>('/codex/oauth/poll', {
    method: 'POST',
    body: JSON.stringify({ device_code }),
  });
}

export async function removeCodexAccount(account_id: string) {
  return api<{ success: boolean }>('/codex/oauth/remove', {
    method: 'POST',
    body: JSON.stringify({ account_id }),
  });
}

export async function setDefaultCodexAccount(account_id: string) {
  return api<{ success: boolean }>('/codex/oauth/set-default', {
    method: 'POST',
    body: JSON.stringify({ account_id }),
  });
}

// Copilot OAuth
export interface CopilotAccount {
  id: string;
  login: string;
  avatar_url: string | null;
  github_domain: string;
}

export interface CopilotUsageResponse {
  copilot_plan: string;
  quota_reset_date: string;
  quota_snapshots: {
    chat: { entitlement: number; remaining: number; percent_remaining: number; unlimited: boolean };
    completions: { entitlement: number; remaining: number; percent_remaining: number; unlimited: boolean };
    premium_interactions: { entitlement: number; remaining: number; percent_remaining: number; unlimited: boolean };
  };
  endpoints?: { api: string; telemetry?: string };
}

export async function getCopilotOAuthStatus() {
  return api<{
    authenticated: boolean;
    accounts: CopilotAccount[];
    default_account_id: string | null;
  }>('/copilot/oauth/status');
}

export async function startCopilotOAuth(github_domain?: string) {
  return api<{
    device_code: string;
    user_code: string;
    verification_uri: string;
    expires_in: number;
  }>('/copilot/oauth/start', {
    method: 'POST',
    body: JSON.stringify({ github_domain }),
  });
}

export async function pollCopilotOAuth(device_code: string, github_domain?: string) {
  return api<{
    success: boolean;
    account?: CopilotAccount;
    error?: string;
  }>('/copilot/oauth/poll', {
    method: 'POST',
    body: JSON.stringify({ device_code, github_domain }),
  });
}

export async function removeCopilotAccount(account_id: string) {
  return api<{ success: boolean }>('/copilot/oauth/remove', {
    method: 'POST',
    body: JSON.stringify({ account_id }),
  });
}

export async function setDefaultCopilotAccount(account_id: string) {
  return api<{ success: boolean }>('/copilot/oauth/set-default', {
    method: 'POST',
    body: JSON.stringify({ account_id }),
  });
}

export async function getCopilotUsage() {
  return api<CopilotUsageResponse>('/copilot/usage');
}

// Proxy
export async function getProxyStatus() {
  return api<{
    running: boolean;
    listen_addr: string | null;
    upstream_url: string;
    http_proxy_url: string | null;
    active_target_provider_id: string | null;
    active_target_provider_name: string | null;
  }>('/proxy/status');
}

export async function startProxy() {
  return api<{ success: boolean; listen_addr?: string; error?: string }>(
    '/proxy/start',
    { method: 'POST' }
  );
}

export async function stopProxy() {
  return api<{ success: boolean; message?: string; error?: string }>(
    '/proxy/stop',
    { method: 'POST' }
  );
}

export async function getProxyTarget() {
  return api<{
    provider_id: string | null;
    provider: Provider | null;
  }>('/proxy/target');
}

export async function setProxyTarget(provider_id: string) {
  return api<{ success: boolean; error?: string }>('/proxy/target', {
    method: 'POST',
    body: JSON.stringify({ provider_id }),
  });
}

// Providers
export interface Provider {
  id: string;
  name: string;
  settingsConfig: unknown;
  websiteUrl?: string;
  category?: string;
  createdAt?: number;
  sortIndex?: number;
  notes?: string;
  icon?: string;
  iconColor?: string;
  meta: unknown;
  inFailoverQueue: boolean;
}

export async function listProviders() {
  return api<{ providers: Record<string, Provider> }>('/providers');
}

export async function getProvider(id: string) {
  return api<{ provider: Provider }>(`/providers/${id}`);
}

export async function getCurrentProviderId() {
  return api<{ current_provider_id: string | null }>('/providers/current');
}

export async function saveProvider(provider: Provider) {
  return api<{ success: boolean }>('/providers', {
    method: 'POST',
    body: JSON.stringify(provider),
  });
}

export async function updateProvider(provider: Provider) {
  return api<{ success: boolean }>(`/providers/${provider.id}`, {
    method: 'PUT',
    body: JSON.stringify(provider),
  });
}

export async function deleteProvider(id: string) {
  return api<{ success: boolean }>(`/providers/${id}`, {
    method: 'DELETE',
  });
}

export async function switchProvider(id: string) {
  return api<{ success: boolean }>(`/providers/${id}/switch`, {
    method: 'POST',
  });
}

// Proxy Settings
export interface ProxyConfig {
  enabled: boolean;
  proxy_type: string;
  host: string;
  port: number;
}

export async function getProxyConfig() {
  return api<ProxyConfig>('/settings/proxy');
}

export async function setProxyConfig(config: ProxyConfig) {
  return api<{ success: boolean }>('/settings/proxy', {
    method: 'PUT',
    body: JSON.stringify(config),
  });
}

export async function deleteProxyConfig() {
  return api<{ success: boolean }>('/settings/proxy', {
    method: 'DELETE',
  });
}

export async function getProxyPort() {
  return api<{ port: number }>('/settings/proxy-port');
}

export async function setProxyPort(port: number) {
  return api<{ success: boolean }>('/settings/proxy-port', {
    method: 'PUT',
    body: JSON.stringify({ port }),
  });
}

// Usage
export interface ProxyUsageSummary {
  provider_id: string;
  model: string;
  total_input_tokens: number;
  total_output_tokens: number;
  request_count: number;
}

export interface ProxyUsageSummaryResponse {
  summary: ProxyUsageSummary[];
  total_input_tokens: number;
  total_output_tokens: number;
  total_requests: number;
}

export interface ProxyUsageTrend {
  day: string;
  total_input_tokens: number;
  total_output_tokens: number;
  request_count: number;
}

export interface ProxyUsageTrendResponse {
  trend: ProxyUsageTrend[];
}

export interface ProxyUsageProvidersResponse {
  providers: ProxyUsageSummary[];
}

export async function getProxyUsageSummary() {
  return api<ProxyUsageSummaryResponse>('/usage/summary');
}

export async function getProxyUsageTrend() {
  return api<ProxyUsageTrendResponse>('/usage/trend');
}

export async function getProxyUsageProviders() {
  return api<ProxyUsageProvidersResponse>('/usage/providers');
}

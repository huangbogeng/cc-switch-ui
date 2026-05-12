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

export async function getCodexOAuthStatus(options?: RequestInit) {
  return api<CodexOAuthStatus>('/codex/oauth/status', options);
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

export async function getCopilotOAuthStatus(options?: RequestInit) {
  return api<{
    authenticated: boolean;
    accounts: CopilotAccount[];
    default_account_id: string | null;
  }>('/copilot/oauth/status', options);
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

export async function getCopilotUsage(signal?: AbortSignal) {
  return api<CopilotUsageResponse>('/copilot/usage', { signal });
}

// Proxy
export async function getProxyStatus(options?: RequestInit) {
  return api<{
    running: boolean;
    listen_addr: string | null;
    upstream_url: string;
    http_proxy_url: string | null;
    active_target_provider_id: string | null;
    active_target_provider_name: string | null;
  }>('/proxy/status', options);
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

export async function listProviders(options?: RequestInit) {
  return api<{ providers: Record<string, Provider> }>('/providers', options);
}

export async function getProvider(id: string) {
  return api<{ provider: Provider }>(`/providers/${id}`);
}

export async function getCurrentProviderId(options?: RequestInit) {
  return api<{ current_provider_id: string | null }>('/providers/current', options);
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

// Outbound network proxy settings
export interface ProxyConfig {
  enabled: boolean;
  proxy_type: string;
  host: string;
  port: number;
}

export async function getProxyConfig(options?: RequestInit) {
  return api<ProxyConfig>('/settings/proxy', options);
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

export async function getProxyPort(options?: RequestInit) {
  return api<{ port: number }>('/settings/proxy-port', options);
}

export async function setProxyPort(port: number) {
  return api<{ success: boolean }>('/settings/proxy-port', {
    method: 'PUT',
    body: JSON.stringify({ port }),
  });
}

// Usage
export interface ProxyUsageTrend {
  day: string;
  total_input_tokens: number;
  total_output_tokens: number;
  request_count: number;
}

export interface UsageSourceItem {
  app_type: string;
  request_count: number;
}

export interface ProxyUsageSummaryResponse {
  totals: {
    input_tokens: number;
    output_tokens: number;
    request_count: number;
  };
  providers: {
    provider_id: string;
    input_tokens: number;
    output_tokens: number;
    request_count: number;
  }[];
  models: {
    model: string;
    input_tokens: number;
    output_tokens: number;
    request_count: number;
  }[];
  trend: ProxyUsageTrend[];
  sources: UsageSourceItem[];
}

export async function getProxyUsageSummary(signal?: AbortSignal) {
  return api<ProxyUsageSummaryResponse>('/usage/summary', { signal });
}

// Model pricing
export interface ModelPricingItem {
  modelId: string;
  displayName: string;
  inputCostPerMillion: string;
  outputCostPerMillion: string;
  cacheReadCostPerMillion: string;
  cacheCreationCostPerMillion: string;
}

export async function getModelPricing(signal?: AbortSignal) {
  return api<ModelPricingItem[]>('/usage/pricing', { signal });
}

export async function upsertModelPricing(pricing: ModelPricingItem) {
  return api<{ success: boolean }>('/usage/pricing', {
    method: 'PUT',
    body: JSON.stringify(pricing),
  });
}

export async function deleteModelPricing(modelId: string) {
  return api<{ success: boolean }>(`/usage/pricing/${encodeURIComponent(modelId)}`, {
    method: 'DELETE',
  });
}

// Provider & Model stats
export interface ProviderStatsItem {
  provider_id: string;
  request_count: number;
  total_input_tokens: number;
  total_output_tokens: number;
  success_count: number;
  fail_count: number;
}

export interface ModelStatsItem {
  model: string;
  request_count: number;
  total_input_tokens: number;
  total_output_tokens: number;
}

export interface RequestLogDetail {
  id: number;
  app_type: string;
  provider_id: string;
  request_path: string;
  request_model: string | null;
  status_code: number | null;
  success: boolean;
  error_message: string | null;
  input_tokens: number | null;
  output_tokens: number | null;
  created_at: number;
}

export interface PaginatedLogs {
  data: RequestLogDetail[];
  total: number;
  page: number;
  page_size: number;
}

export interface LogsQueryParams {
  page?: number;
  page_size?: number;
  app_type?: string;
  provider_id?: string;
  model?: string;
  status_code?: number;
  start_date?: number;
  end_date?: number;
}

export async function getProviderStats(start_date?: number, end_date?: number, signal?: AbortSignal) {
  const params = new URLSearchParams();
  if (start_date !== undefined) params.set('start_date', String(start_date));
  if (end_date !== undefined) params.set('end_date', String(end_date));
  const qs = params.toString();
  return api<{ providers: ProviderStatsItem[] }>(`/usage/provider-stats${qs ? '?' + qs : ''}`, { signal });
}

export async function getModelStats(start_date?: number, end_date?: number, signal?: AbortSignal) {
  const params = new URLSearchParams();
  if (start_date !== undefined) params.set('start_date', String(start_date));
  if (end_date !== undefined) params.set('end_date', String(end_date));
  const qs = params.toString();
  return api<{ models: ModelStatsItem[] }>(`/usage/model-stats${qs ? '?' + qs : ''}`, { signal });
}

export async function getRequestLogs(params: LogsQueryParams, signal?: AbortSignal) {
  const qs = new URLSearchParams();
  if (params.page !== undefined) qs.set('page', String(params.page));
  if (params.page_size !== undefined) qs.set('page_size', String(params.page_size));
  if (params.app_type) qs.set('app_type', params.app_type);
  if (params.provider_id) qs.set('provider_id', params.provider_id);
  if (params.model) qs.set('model', params.model);
  if (params.status_code !== undefined) qs.set('status_code', String(params.status_code));
  if (params.start_date !== undefined) qs.set('start_date', String(params.start_date));
  if (params.end_date !== undefined) qs.set('end_date', String(params.end_date));
  return api<PaginatedLogs>(`/usage/request-logs?${qs.toString()}`, { signal });
}

export async function getRequestLogDetail(id: number, signal?: AbortSignal) {
  return api<RequestLogDetail | null>(`/usage/request-logs/${id}`, { signal });
}

// Session usage sync
export interface SessionSyncResult {
  imported: number;
  skipped: number;
  filesScanned: number;
  errors: string[];
}

export interface DataSourceSummary {
  dataSource: string;
  requestCount: number;
  totalCostUsd: string;
}

export async function syncSessionUsage() {
  return api<SessionSyncResult>('/usage/sync-session', {
    method: 'POST',
  });
}

export async function getDataSourceBreakdown(signal?: AbortSignal) {
  return api<DataSourceSummary[]>('/usage/sources', { signal });
}

// MCP Servers
export interface McpServer {
  id: string;
  name: string;
  serverSpec: unknown;
  appType: string;
  enabled: boolean;
}

export async function listMcpServers(options?: RequestInit) {
  return api<{ servers: McpServer[] }>('/mcp/servers', options);
}

export async function saveMcpServer(server: McpServer) {
  return api<{ success: boolean }>('/mcp/servers', {
    method: 'POST',
    body: JSON.stringify(server),
  });
}

export async function deleteMcpServer(id: string) {
  return api<{ success: boolean }>(`/mcp/servers/${encodeURIComponent(id)}`, {
    method: 'DELETE',
  });
}

export async function syncMcpServers() {
  return api<{ success: boolean }>('/mcp/servers/sync', { method: 'POST' });
}

export async function importMcpServers() {
  return api<{ success: boolean; imported: number }>('/mcp/servers/import', { method: 'POST' });
}

export async function toggleMcpServer(id: string) {
  return api<{ success: boolean; enabled: boolean }>(`/mcp/servers/${encodeURIComponent(id)}/toggle`, { method: 'POST' });
}

// Skills
export interface Skill {
  id: string;
  name: string;
  description?: string;
  directory: string;
  appType: string;
  enabled: boolean;
  collection?: string;
  installedAt: number;
  repoOwner?: string;
  repoName?: string;
  repoBranch?: string;
  readmeUrl?: string;
}

export async function listSkills(options?: RequestInit) {
  return api<{ skills: Skill[] }>('/skills', options);
}

export async function saveSkill(skill: Skill) {
  return api<{ success: boolean }>('/skills', {
    method: 'POST',
    body: JSON.stringify(skill),
  });
}

export async function deleteSkill(id: string) {
  return api<{ success: boolean }>(`/skills/${encodeURIComponent(id)}`, {
    method: 'DELETE',
  });
}

export async function syncSkills() {
  return api<{ success: boolean }>('/skills/sync', { method: 'POST' });
}

export async function importSkills() {
  return api<{ success: boolean; imported: number }>('/skills/import', { method: 'POST' });
}

export async function toggleSkill(id: string) {
  return api<{ success: boolean; enabled: boolean }>(`/skills/${encodeURIComponent(id)}/toggle`, { method: 'POST' });
}

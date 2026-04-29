import type { FormEvent, ReactNode } from 'react';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Card, CardContent } from '@/components/ui/card';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Textarea } from '@/components/ui/textarea';
import type { ProviderPreset } from '@/config/providerPresets';
import { PresetSelector } from './PresetSelector';
import type { ApiFormat, ApiKeyField, ProviderFormData } from './providerForm';
import type { CodexAccount } from '@/api';

interface ProviderFormDialogProps {
  open: boolean;
  editingId: string | null;
  selectedPreset: ProviderPreset | null;
  formData: ProviderFormData;
  saving: boolean;
  error?: string;
  codexAccounts?: CodexAccount[];
  onChange: (formData: ProviderFormData) => void;
  onPresetSelect: (preset: ProviderPreset) => void;
  onCancel: () => void;
  onSubmit: (event: FormEvent) => void;
}

export function ProviderFormDialog({
  open,
  editingId,
  selectedPreset,
  formData,
  saving,
  error,
  codexAccounts = [],
  onChange,
  onPresetSelect,
  onCancel,
  onSubmit,
}: ProviderFormDialogProps) {
  if (!open) return null;

  const usesOAuthProxy = formData.authMode === 'oauth_proxy';
  const showApiKey = !usesOAuthProxy;
  const showEndpoint = !usesOAuthProxy || !!formData.baseUrl;

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/55 px-4 py-6 backdrop-blur-sm">
      <Card className="max-h-[90vh] w-full max-w-2xl overflow-y-auto">
        <CardContent className="p-6">
          <div className="mb-5">
            <h2 className="text-xl font-semibold leading-7 text-foreground">
              {editingId ? 'Edit Provider' : selectedPreset ? `Add ${selectedPreset.name}` : 'Add Custom Provider'}
            </h2>
            <p className="mt-1 text-sm leading-5 text-muted-foreground">
              Keep routing credentials local to this admin service.
            </p>
          </div>

          {!editingId && !selectedPreset && (
            <div className="mb-6 space-y-4">
              <PresetSelector onSelect={onPresetSelect} />
              <div className="grid grid-cols-[minmax(0,1fr)_auto_minmax(0,1fr)] items-center gap-3 text-xs leading-4 text-muted-foreground">
                <div className="h-px flex-1 bg-border" />
                or enter manually
                <div className="h-px flex-1 bg-border" />
              </div>
            </div>
          )}

          {selectedPreset && (
            <div className="mb-5 rounded-2xl border border-white/10 bg-white/[0.04] p-4">
              <div className="flex flex-wrap items-center gap-2">
                <Badge style={{ background: selectedPreset.iconColor }} className="border-transparent text-white">
                  {selectedPreset.name}
                </Badge>
                <Badge variant="outline">{usesOAuthProxy ? 'OAuth Proxy' : 'API Key'}</Badge>
              </div>
              {selectedPreset.description && (
                <p className="mt-2 text-sm leading-5 text-muted-foreground">{selectedPreset.description}</p>
              )}
              {usesOAuthProxy && (
                <p className="mt-3 rounded-2xl border border-primary/20 bg-primary/10 px-3 py-2 text-sm leading-5 text-primary">
                  This provider uses a ChatGPT account through the local Codex proxy. Connect Codex OAuth on the Dashboard; no API key is required here.
                </p>
              )}
            </div>
          )}

          <form onSubmit={onSubmit} className="space-y-5" noValidate>
            {error && (
              <div className="rounded-2xl border border-destructive/30 bg-destructive/10 px-3 py-2 text-sm leading-5 text-destructive">
                {error}
              </div>
            )}

            <FormSection title="Basic">
              <div className="grid gap-4 sm:grid-cols-2">
                <div className="space-y-2">
                  <Label htmlFor="provider-id">Provider ID</Label>
                  <Input
                    id="provider-id"
                    value={formData.id}
                    onChange={(e) => onChange({ ...formData, id: e.target.value })}
                    disabled={!!editingId}
                  />
                </div>
                <div className="space-y-2">
                  <Label htmlFor="provider-name">Name</Label>
                  <Input
                    id="provider-name"
                    value={formData.name}
                    onChange={(e) => onChange({ ...formData, name: e.target.value })}
                  />
                </div>
              </div>

              {!selectedPreset && (
                <div className="space-y-2">
                  <Label htmlFor="website-url">Website URL</Label>
                  <Input
                    id="website-url"
                    placeholder="Optional provider website"
                    value={formData.websiteUrl}
                    onChange={(e) => onChange({ ...formData, websiteUrl: e.target.value })}
                  />
                </div>
              )}
            </FormSection>

            <FormSection title="Authentication">
              {showApiKey ? (
                <div className="grid gap-4 sm:grid-cols-[minmax(0,1fr)_180px]">
                  <div className="space-y-2">
                    <Label htmlFor="api-key">API Key</Label>
                    <Input
                      id="api-key"
                      type="password"
                      value={formData.apiKey}
                      onChange={(e) => onChange({ ...formData, apiKey: e.target.value })}
                    />
                  </div>
                  <div className="space-y-2">
                    <Label htmlFor="api-key-field">Auth Field</Label>
                    <Select
                      id="api-key-field"
                      value={formData.apiKeyField}
                      onChange={(value) => onChange({ ...formData, apiKeyField: value as ApiKeyField })}
                    >
                      <option value="ANTHROPIC_AUTH_TOKEN">AUTH_TOKEN</option>
                      <option value="ANTHROPIC_API_KEY">API_KEY</option>
                    </Select>
                  </div>
                </div>
              ) : (
                <div className="space-y-3">
                  <div className="rounded-2xl border border-primary/20 bg-primary/10 px-3 py-2 text-sm leading-5 text-primary">
                    Uses managed ChatGPT OAuth credentials. This provider is still saved as a switchable route.
                  </div>
                  <div className="space-y-2">
                    <Label htmlFor="codex-account">ChatGPT Account</Label>
                    <Select
                      id="codex-account"
                      value={formData.codexAccountId}
                      onChange={(value) => onChange({ ...formData, codexAccountId: value })}
                    >
                      <option value="">Default account</option>
                      {codexAccounts.map((account) => (
                        <option key={account.id} value={account.id}>
                          {account.login}{account.is_default ? ' (default)' : ''}
                        </option>
                      ))}
                    </Select>
                  </div>
                </div>
              )}
            </FormSection>

            {showEndpoint && (
              <FormSection title="Endpoint">
                <div className="space-y-2">
                  <Label htmlFor="base-url">Base URL</Label>
                  <Input
                    id="base-url"
                    placeholder="https://api.example.com/anthropic"
                    value={formData.baseUrl}
                    onChange={(e) => onChange({ ...formData, baseUrl: e.target.value })}
                  />
                </div>
                <label className="flex items-center gap-2 text-sm leading-5 text-muted-foreground">
                  <input
                    type="checkbox"
                    checked={formData.isFullUrl}
                    onChange={(e) => onChange({ ...formData, isFullUrl: e.target.checked })}
                    className="h-4 w-4 rounded border-input accent-primary"
                  />
                  Treat Base URL as a full request URL
                </label>
              </FormSection>
            )}

            <FormSection title="Models">
              <div className="grid gap-4 sm:grid-cols-2">
                <ModelInput
                  id="model-main"
                  label="Main"
                  value={formData.model}
                  onChange={(value) => onChange({ ...formData, model: value })}
                />
                <ModelInput
                  id="model-haiku"
                  label="Haiku"
                  value={formData.haikuModel}
                  onChange={(value) => onChange({ ...formData, haikuModel: value })}
                />
                <ModelInput
                  id="model-sonnet"
                  label="Sonnet"
                  value={formData.sonnetModel}
                  onChange={(value) => onChange({ ...formData, sonnetModel: value })}
                />
                <ModelInput
                  id="model-opus"
                  label="Opus"
                  value={formData.opusModel}
                  onChange={(value) => onChange({ ...formData, opusModel: value })}
                />
              </div>
              <Button
                type="button"
                size="sm"
                variant="outline"
                onClick={() => {
                  const value = formData.model || formData.sonnetModel || formData.opusModel || formData.haikuModel;
                  onChange({
                    ...formData,
                    model: value,
                    haikuModel: value,
                    sonnetModel: value,
                    opusModel: value,
                  });
                }}
              >
                Apply one model to all
              </Button>
            </FormSection>

            <FormSection title="Advanced">
              <div className="grid gap-4 sm:grid-cols-2">
                <div className="space-y-2">
                  <Label htmlFor="api-format">API Format</Label>
                  <Select
                    id="api-format"
                    value={formData.apiFormat}
                    onChange={(value) => onChange({ ...formData, apiFormat: value as ApiFormat })}
                  >
                    <option value="anthropic">Anthropic Messages</option>
                    <option value="openai_chat">OpenAI Chat</option>
                    <option value="openai_responses">OpenAI Responses</option>
                    <option value="gemini_native">Gemini Native</option>
                  </Select>
                </div>
                <div className="space-y-2">
                  <Label htmlFor="api-timeout">API Timeout MS</Label>
                  <Input
                    id="api-timeout"
                    inputMode="numeric"
                    value={formData.apiTimeoutMs}
                    onChange={(e) => onChange({ ...formData, apiTimeoutMs: e.target.value })}
                    placeholder="3000000"
                  />
                </div>
              </div>

              <div className="space-y-2">
                <Label htmlFor="prompt-cache-key">Prompt Cache Key</Label>
                <Input
                  id="prompt-cache-key"
                  value={formData.promptCacheKey}
                  onChange={(e) => onChange({ ...formData, promptCacheKey: e.target.value })}
                  placeholder="Optional, for OpenAI Responses-compatible providers"
                />
              </div>

              {usesOAuthProxy && (
                <div className="space-y-2">
                  <Label htmlFor="codex-http-proxy">Codex HTTP Proxy URL</Label>
                  <Input
                    id="codex-http-proxy"
                    value={formData.codexHttpProxy}
                    onChange={(e) => onChange({ ...formData, codexHttpProxy: e.target.value })}
                    placeholder="http://127.0.0.1:7890 or socks5://127.0.0.1:7890"
                  />
                  <p className="text-xs leading-4 text-muted-foreground">
                    Applies to Codex upstream forwarding for this provider.
                  </p>
                </div>
              )}

              <label className="flex items-center gap-2 text-sm leading-5 text-muted-foreground">
                <input
                  type="checkbox"
                  checked={formData.disableNonessentialTraffic}
                  onChange={(e) => onChange({ ...formData, disableNonessentialTraffic: e.target.checked })}
                  className="h-4 w-4 rounded border-input accent-primary"
                />
                Disable Claude Code nonessential traffic
              </label>
            </FormSection>

            <div className="space-y-2">
              <Label htmlFor="notes">Notes</Label>
              <Textarea
                id="notes"
                value={formData.notes}
                onChange={(e) => onChange({ ...formData, notes: e.target.value })}
                placeholder="Optional"
              />
            </div>

            <div className="flex justify-end gap-2 pt-2">
              <Button type="button" variant="ghost" onClick={onCancel}>
                Cancel
              </Button>
              <Button type="submit" disabled={saving}>
                {saving ? 'Saving...' : 'Save'}
              </Button>
            </div>
          </form>
        </CardContent>
      </Card>
    </div>
  );
}

function FormSection({ title, children }: { title: string; children: ReactNode }) {
  return (
    <section className="space-y-3 rounded-2xl border border-white/10 bg-white/[0.025] p-4">
      <h3 className="text-sm font-medium leading-5 text-muted-foreground">{title}</h3>
      {children}
    </section>
  );
}

function ModelInput({
  id,
  label,
  value,
  onChange,
}: {
  id: string;
  label: string;
  value: string;
  onChange: (value: string) => void;
}) {
  return (
    <div className="space-y-2">
      <Label htmlFor={id}>{label}</Label>
      <Input id={id} value={value} onChange={(e) => onChange(e.target.value)} />
    </div>
  );
}

function Select({
  id,
  value,
  onChange,
  children,
}: {
  id: string;
  value: string;
  onChange: (value: string) => void;
  children: ReactNode;
}) {
  return (
    <select
      id={id}
      value={value}
      onChange={(e) => onChange(e.target.value)}
      className="h-10 w-full rounded-xl border border-input bg-white/[0.04] px-3 text-sm leading-5 text-foreground shadow-inner shadow-black/10 outline-none transition focus:border-primary/70 focus:ring-4 focus:ring-primary/15"
    >
      {children}
    </select>
  );
}

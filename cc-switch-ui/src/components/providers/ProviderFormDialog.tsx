import { useState, type FormEvent, type ReactNode } from 'react';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Card, CardContent } from '@/components/ui/card';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Select } from '@/components/ui/select';
import { Textarea } from '@/components/ui/textarea';
import type { ProviderPreset } from '@/config/providerPresets';
import { PresetSelector } from './PresetSelector';
import type { ApiFormat, ApiKeyField, ProviderFormData } from './providerForm';
import type { CodexAccount } from '@/api';

interface ProviderFormDialogProps {
  open: boolean;
  editingId: string | null;
  selectedPreset: ProviderPreset | null;
  initialFormData: ProviderFormData;
  saving: boolean;
  error?: string;
  codexAccounts?: CodexAccount[];
  onPresetSelect: (preset: ProviderPreset) => void;
  onCancel: () => void;
  onSubmit: (formData: ProviderFormData) => void;
}

export function ProviderFormDialog({
  open,
  editingId,
  selectedPreset,
  initialFormData,
  saving,
  error,
  codexAccounts = [],
  onPresetSelect,
  onCancel,
  onSubmit,
}: ProviderFormDialogProps) {
  const [formData, setFormData] = useState<ProviderFormData>(initialFormData);

  if (!open) return null;

  const usesOAuthProxy = formData.authMode === 'oauth_proxy';
  const showApiKey = !usesOAuthProxy;
  const showEndpoint = !usesOAuthProxy || !!formData.baseUrl;

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/65 px-4 py-6">
      <Card className="w-full max-w-2xl overflow-hidden [contain:layout_paint]">
        <CardContent className="max-h-[90vh] overflow-y-auto overscroll-contain p-6 [scrollbar-gutter:stable] [will-change:scroll-position]">
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
                <Badge variant="outline">{usesOAuthProxy ? 'OAuth Account' : 'API Key'}</Badge>
              </div>
              {selectedPreset.description && (
                <p className="mt-2 text-sm leading-5 text-muted-foreground">{selectedPreset.description}</p>
              )}
              {usesOAuthProxy && (
                <p className="mt-3 rounded-2xl border border-primary/20 bg-primary/10 px-3 py-2 text-sm leading-5 text-primary">
                  This route uses a ChatGPT account through the local route endpoint. Connect Codex OAuth on the OAuth page; no API key is required here.
                </p>
              )}
            </div>
          )}

          <form
            onSubmit={(event: FormEvent) => {
              event.preventDefault();
              onSubmit(formData);
            }}
            className="space-y-5"
            noValidate
          >
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
                    onChange={(e) => setFormData({ ...formData, id: e.target.value })}
                    disabled={!!editingId}
                  />
                </div>
                <div className="space-y-2">
                  <Label htmlFor="provider-name">Name</Label>
                  <Input
                    id="provider-name"
                    value={formData.name}
                    onChange={(e) => setFormData({ ...formData, name: e.target.value })}
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
                    onChange={(e) => setFormData({ ...formData, websiteUrl: e.target.value })}
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
                      onChange={(e) => setFormData({ ...formData, apiKey: e.target.value })}
                    />
                  </div>
                  <div className="space-y-2">
                    <Label htmlFor="api-key-field">Auth Field</Label>
                    <Select
                      id="api-key-field"
                      value={formData.apiKeyField}
                      onChange={(e) => setFormData({ ...formData, apiKeyField: e.target.value as ApiKeyField })}
                    >
                      <option value="ANTHROPIC_AUTH_TOKEN">AUTH_TOKEN</option>
                      <option value="ANTHROPIC_API_KEY">API_KEY</option>
                    </Select>
                  </div>
                </div>
              ) : (
                <div className="space-y-3">
                  <div className="rounded-2xl border border-primary/20 bg-primary/10 px-3 py-2 text-sm leading-5 text-primary">
                    Uses managed ChatGPT OAuth credentials. This provider is saved as a switchable route.
                  </div>
                  <div className="space-y-2">
                    <Label htmlFor="codex-account">ChatGPT Account</Label>
                    <Select
                      id="codex-account"
                      value={formData.codexAccountId}
                      onChange={(e) => setFormData({ ...formData, codexAccountId: e.target.value })}
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
                    onChange={(e) => setFormData({ ...formData, baseUrl: e.target.value })}
                  />
                </div>
                <label className="flex items-center gap-2 text-sm leading-5 text-muted-foreground">
                  <input
                    type="checkbox"
                    checked={formData.isFullUrl}
                    onChange={(e) => setFormData({ ...formData, isFullUrl: e.target.checked })}
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
                  onChange={(value) => setFormData({ ...formData, model: value })}
                />
                <ModelInput
                  id="model-haiku"
                  label="Haiku"
                  value={formData.haikuModel}
                  onChange={(value) => setFormData({ ...formData, haikuModel: value })}
                />
                <ModelInput
                  id="model-sonnet"
                  label="Sonnet"
                  value={formData.sonnetModel}
                  onChange={(value) => setFormData({ ...formData, sonnetModel: value })}
                />
                <ModelInput
                  id="model-opus"
                  label="Opus"
                  value={formData.opusModel}
                  onChange={(value) => setFormData({ ...formData, opusModel: value })}
                />
              </div>
              <Button
                type="button"
                size="sm"
                variant="outline"
                onClick={() => {
                  const value = formData.model || formData.sonnetModel || formData.opusModel || formData.haikuModel;
                  setFormData({
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

            <details className="rounded-2xl border border-white/10 bg-white/[0.025] p-4">
              <summary className="cursor-pointer select-none text-sm font-medium leading-5 text-muted-foreground">
                Advanced
              </summary>
              <div className="space-y-4 pt-4">
              <div className="grid gap-4 sm:grid-cols-2">
                <div className="space-y-2">
                  <Label htmlFor="api-format">API Format</Label>
                  <Select
                    id="api-format"
                    value={formData.apiFormat}
                    onChange={(e) => setFormData({ ...formData, apiFormat: e.target.value as ApiFormat })}
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
                    onChange={(e) => setFormData({ ...formData, apiTimeoutMs: e.target.value })}
                    placeholder="3000000"
                  />
                </div>
              </div>

              <div className="space-y-2">
                <Label htmlFor="prompt-cache-key">Prompt Cache Key</Label>
                <Input
                  id="prompt-cache-key"
                  value={formData.promptCacheKey}
                  onChange={(e) => setFormData({ ...formData, promptCacheKey: e.target.value })}
                  placeholder="Optional, for OpenAI Responses-compatible providers"
                />
              </div>

              {usesOAuthProxy && (
                <label className="flex items-center gap-2 text-sm leading-5 text-muted-foreground">
                  <input
                    type="checkbox"
                    checked={formData.codexFastMode}
                    onChange={(e) => setFormData({ ...formData, codexFastMode: e.target.checked })}
                    className="h-4 w-4 rounded border-input accent-primary"
                  />
                  Codex FAST mode
                </label>
              )}

              <label className="flex items-center gap-2 text-sm leading-5 text-muted-foreground">
                <input
                  type="checkbox"
                  checked={formData.disableNonessentialTraffic}
                  onChange={(e) =>
                    setFormData({ ...formData, disableNonessentialTraffic: e.target.checked })
                  }
                  className="h-4 w-4 rounded border-input accent-primary"
                />
                Disable Claude Code nonessential traffic
              </label>

              <div className="grid gap-4 sm:grid-cols-2">
                <div className="space-y-2">
                  <Label htmlFor="subagent-model">Subagent Model</Label>
                  <Input
                    id="subagent-model"
                    value={formData.subagentModel}
                    onChange={(e) => setFormData({ ...formData, subagentModel: e.target.value })}
                    placeholder="CLAUDE_CODE_SUBAGENT_MODEL"
                  />
                </div>
                <div className="space-y-2">
                  <Label htmlFor="effort-level">Effort Level</Label>
                  <Select
                    id="effort-level"
                    value={formData.effortLevel}
                    onChange={(e) => setFormData({ ...formData, effortLevel: e.target.value })}
                  >
                    <option value="">(default)</option>
                    <option value="low">low</option>
                    <option value="medium">medium</option>
                    <option value="max">max</option>
                  </Select>
                </div>
              </div>
              </div>
            </details>

            <div className="space-y-2">
              <Label htmlFor="notes">Notes</Label>
              <Textarea
                id="notes"
                value={formData.notes}
                onChange={(e) => setFormData({ ...formData, notes: e.target.value })}
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

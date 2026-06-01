import { useEffect, useMemo, useState, type FormEvent, type ReactNode } from 'react';
import { Download, Loader2 } from 'lucide-react';
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
import {
  detectProviderEndpointType,
  fetchProviderModels,
  type CodexAccount,
  type EndpointDetectionResult,
  type FetchedModel,
} from '@/api';

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
  const [fetchedModels, setFetchedModels] = useState<FetchedModel[]>([]);
  const [fetchingModels, setFetchingModels] = useState(false);
  const [fetchModelsError, setFetchModelsError] = useState('');
  const [detectingApiFormat, setDetectingApiFormat] = useState(false);
  const [endpointDetection, setEndpointDetection] = useState<EndpointDetectionResult | null>(null);
  const [detectEndpointError, setDetectEndpointError] = useState('');

  useEffect(() => {
    setFormData(initialFormData);
    setFetchedModels([]);
    setFetchModelsError('');
    setEndpointDetection(null);
    setDetectEndpointError('');
  }, [initialFormData, open]);

  if (!open) return null;

  const usesOAuthProxy = formData.authMode === 'oauth_proxy';
  const showApiKey = !usesOAuthProxy;
  const showEndpoint = !usesOAuthProxy || !!formData.baseUrl;
  const modelOptions = useMemo(
    () => fetchedModels.map((model) => model.id).filter((value, index, list) => list.indexOf(value) === index),
    [fetchedModels]
  );
  const canFetchModels = !!formData.baseUrl.trim() && (!showApiKey || !!formData.apiKey.trim() || isLikelyLocalBaseUrl(formData.baseUrl));
  const canDetectEndpointType = canFetchModels && !usesOAuthProxy;

  const handleFetchModels = async () => {
    setFetchingModels(true);
    setFetchModelsError('');
    try {
      const response = await fetchProviderModels({
        baseUrl: formData.baseUrl.trim(),
        apiKey: formData.apiKey.trim(),
        isFullUrl: formData.isFullUrl,
        modelsUrl: selectedPreset?.modelsUrl,
      });
      setFetchedModels(response.models);
      if (response.models.length === 0) {
        setFetchModelsError('No models returned from this endpoint.');
      }
    } catch (e) {
      setFetchedModels([]);
      setFetchModelsError(e instanceof Error ? e.message : 'Failed to fetch models');
    } finally {
      setFetchingModels(false);
    }
  };

  const handleDetectEndpointType = async () => {
    setDetectingApiFormat(true);
    setDetectEndpointError('');
    setEndpointDetection(null);
    try {
      const result = await detectProviderEndpointType({
        baseUrl: formData.baseUrl.trim(),
        apiKey: formData.apiKey.trim(),
        isFullUrl: formData.isFullUrl,
      });
      setEndpointDetection(result);
    } catch (e) {
      setDetectEndpointError(e instanceof Error ? e.message : 'Failed to detect endpoint type');
    } finally {
      setDetectingApiFormat(false);
    }
  };

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
                  options={modelOptions}
                  onChange={(value) => setFormData({ ...formData, model: value })}
                />
                <ModelInput
                  id="model-haiku"
                  label="Haiku"
                  value={formData.haikuModel}
                  options={modelOptions}
                  onChange={(value) => setFormData({ ...formData, haikuModel: value })}
                />
                <ModelInput
                  id="model-sonnet"
                  label="Sonnet"
                  value={formData.sonnetModel}
                  options={modelOptions}
                  onChange={(value) => setFormData({ ...formData, sonnetModel: value })}
                />
                <ModelInput
                  id="model-opus"
                  label="Opus"
                  value={formData.opusModel}
                  options={modelOptions}
                  onChange={(value) => setFormData({ ...formData, opusModel: value })}
                />
              </div>
              <div className="flex flex-wrap items-center gap-2">
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
              </div>
            </FormSection>

            <details className="rounded-2xl border border-white/10 bg-white/[0.025] p-4">
              <summary className="cursor-pointer select-none text-sm font-medium leading-5 text-muted-foreground">
                Diagnostics
              </summary>
              <div className="space-y-4 pt-4">
                <div className="rounded-xl border border-white/10 bg-black/20 px-3 py-3">
                  <div className="flex flex-wrap items-center gap-2">
                    <Button
                      type="button"
                      size="sm"
                      variant="outline"
                      onClick={handleDetectEndpointType}
                      disabled={detectingApiFormat || !canDetectEndpointType}
                    >
                      {detectingApiFormat ? (
                        <Loader2 className="mr-2 h-4 w-4 animate-spin" />
                      ) : (
                        <Download className="mr-2 h-4 w-4" />
                      )}
                      Detect endpoint type
                    </Button>
                    {endpointDetection?.recommendedApiFormat && (
                      <Button
                        type="button"
                        size="sm"
                        variant="ghost"
                        onClick={() =>
                          setFormData({
                            ...formData,
                            apiFormat: endpointDetection.recommendedApiFormat as ApiFormat,
                          })
                        }
                      >
                        Apply recommended {formatApiFormatLabel(endpointDetection.recommendedApiFormat)}
                      </Button>
                    )}
                  </div>
                  {detectEndpointError && (
                    <p className="mt-3 text-sm text-muted-foreground">{detectEndpointError}</p>
                  )}
                  {endpointDetection && (
                    <div className="mt-3 space-y-2 text-sm">
                      <p className="text-muted-foreground">
                        Recommended:{' '}
                        {endpointDetection.recommendedApiFormat
                          ? formatApiFormatLabel(endpointDetection.recommendedApiFormat)
                          : 'No clear match'}
                      </p>
                      <div className="space-y-1 text-xs text-muted-foreground">
                        {endpointDetection.probes.map((probe) => (
                          <div key={probe.apiFormat} className="space-y-1 rounded-lg border border-white/10 px-2 py-2">
                            <div className="flex flex-wrap items-center gap-x-2 gap-y-1">
                              <span className="font-medium text-foreground">{formatApiFormatLabel(probe.apiFormat)}</span>
                              <span>{probe.supported ? 'supported' : 'not supported'}</span>
                              <span>{probe.statusCode ? `HTTP ${probe.statusCode}` : 'no status'}</span>
                            </div>
                            {probe.error && (
                              <pre className="whitespace-pre-wrap break-words rounded-md bg-black/30 px-2 py-2 text-[11px] leading-4 text-muted-foreground">
                                {probe.error}
                              </pre>
                            )}
                          </div>
                        ))}
                      </div>
                    </div>
                  )}
                </div>

                <div className="rounded-xl border border-white/10 bg-black/20 px-3 py-3">
                  <div className="flex flex-wrap items-center gap-2">
                    <Button
                      type="button"
                      size="sm"
                      variant="outline"
                      onClick={handleFetchModels}
                      disabled={usesOAuthProxy || fetchingModels || !canFetchModels}
                    >
                      {fetchingModels ? (
                        <Loader2 className="mr-2 h-4 w-4 animate-spin" />
                      ) : (
                        <Download className="mr-2 h-4 w-4" />
                      )}
                      Fetch models
                    </Button>
                    {modelOptions.length > 0 && (
                      <span className="text-xs text-muted-foreground">
                        {modelOptions.length} models fetched
                      </span>
                    )}
                  </div>
                  {fetchModelsError && (
                    <p className="mt-3 text-sm text-muted-foreground">{fetchModelsError}</p>
                  )}
                </div>
              </div>
            </details>

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
  options,
  onChange,
}: {
  id: string;
  label: string;
  value: string;
  options?: string[];
  onChange: (value: string) => void;
}) {
  const listId = `${id}-models`;
  return (
    <div className="space-y-2">
      <Label htmlFor={id}>{label}</Label>
      <Input id={id} list={options?.length ? listId : undefined} value={value} onChange={(e) => onChange(e.target.value)} />
      {options?.length ? (
        <datalist id={listId}>
          {options.map((option) => (
            <option key={option} value={option} />
          ))}
        </datalist>
      ) : null}
    </div>
  );
}

function isLikelyLocalBaseUrl(baseUrl: string): boolean {
  const trimmed = baseUrl.trim();
  if (!trimmed) return false;
  return (
    trimmed.includes('://localhost') ||
    trimmed.includes('://127.0.0.1') ||
    trimmed.includes('://0.0.0.0') ||
    trimmed.startsWith('localhost:') ||
    trimmed.startsWith('127.0.0.1:') ||
    trimmed.startsWith('0.0.0.0:')
  );
}

function formatApiFormatLabel(apiFormat: string): string {
  switch (apiFormat) {
    case 'openai_chat':
      return 'OpenAI Chat';
    case 'openai_responses':
      return 'OpenAI Responses';
    case 'anthropic':
    default:
      return 'Anthropic Messages';
  }
}

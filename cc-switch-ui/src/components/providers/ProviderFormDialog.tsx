import { ModelPickerDialog } from './ModelPickerDialog';
import { ModelMappingInput } from './ModelMappingInput';
import { CodexDiagnostics } from './CodexDiagnostics';
import { ProviderLogo } from './ProviderLogo';
import { useEffect, useId, useMemo, useRef, useState, type FormEvent, type ReactNode } from 'react';
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
  fetchCodexModels,
  type CodexAccount,
  type EndpointDetectionResult,
  type FetchedModel,
  type ProviderPreview,
} from '@/api';
import { useDialog } from '@/lib/useDialog';
import { useUnsavedChanges } from '@/lib/useUnsavedChanges';

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
  onPreview?: (formData: ProviderFormData) => Promise<ProviderPreview>;
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
  onPreview,
}: ProviderFormDialogProps) {
  const [formData, setFormData] = useState<ProviderFormData>(initialFormData);
  const [previewState, setPreviewState] = useState<{ fingerprint: string; result: ProviderPreview } | null>(null);
  const preview = previewState?.fingerprint === JSON.stringify(formData) ? previewState.result : null;
  const [previewing, setPreviewing] = useState(false);
  const [previewError, setPreviewError] = useState('');
  const [pickerOpen, setPickerOpen] = useState(false);
  const [modelsFor, setModelsFor] = useState<string | null>(null);
  const modelRequest = useRef(0);
  const [fetchedModels, setFetchedModels] = useState<FetchedModel[]>([]);
  const [fetchingModels, setFetchingModels] = useState(false);
  const [fetchModelsError, setFetchModelsError] = useState('');
  const [detectingApiFormat, setDetectingApiFormat] = useState(false);
  const [endpointDetection, setEndpointDetection] = useState<EndpointDetectionResult | null>(null);
  const [detectEndpointError, setDetectEndpointError] = useState('');
  const titleId = useId();
  const canDiscard = useUnsavedChanges(open && JSON.stringify(formData) !== JSON.stringify(initialFormData), open && (saving || previewing));
  const requestCancel = () => { if (canDiscard()) onCancel(); };
  const dialogRef = useDialog(open, requestCancel, !saving && !previewing);

  useEffect(() => {
    setFormData(initialFormData);
    setFetchedModels([]);
    setFetchModelsError('');
    setEndpointDetection(null);
    setDetectEndpointError('');
  }, [initialFormData, open]);

  useEffect(() => { setPreviewState(null); setPreviewError(''); }, [formData, open]);
  const inspectChanges = async () => {
    if (!onPreview || saving || previewing) return;
    setPreviewing(true); setPreviewState(null); setPreviewError('');
    try { setPreviewState({ fingerprint: JSON.stringify(formData), result: await onPreview(formData) }); }
    catch (e) { setPreviewError(e instanceof Error ? e.message : 'Preview unavailable'); }
    finally { setPreviewing(false); }
  };

  const modelAccountId = formData.codexAccountId.trim() || codexAccounts.find(account => account.is_default)?.id || codexAccounts[0]?.id;
  const hasModelAccount = codexAccounts.some(account => account.id === modelAccountId);
  const modelsContext = JSON.stringify([formData.baseUrl, formData.apiKey, formData.isFullUrl, formData.authMode, formData.noAuth, selectedPreset?.modelsUrl, formData.codexAccountId, modelAccountId, hasModelAccount]);
  useEffect(() => {
    modelRequest.current += 1; setModelsFor(null); setFetchedModels([]); setFetchModelsError(''); setFetchingModels(false); setPickerOpen(false);
    return () => { modelRequest.current += 1; };
  }, [modelsContext, open]);
  const availableModels = useMemo(() => modelsFor === modelsContext ? fetchedModels : [], [modelsFor, modelsContext, fetchedModels]);
  const modelOptions = useMemo(
    () => availableModels.map((model) => model.id).filter((value, index, list) => list.indexOf(value) === index),
    [availableModels]
  );

  if (!open) return null;

  const usesOAuthProxy = formData.authMode === 'oauth_proxy';
  const noAuth = formData.noAuth;
  const showApiKey = !usesOAuthProxy && !noAuth;
  const showEndpoint = !noAuth && (!usesOAuthProxy || !!formData.baseUrl);
  const canFetchModels = usesOAuthProxy
    ? hasModelAccount
    : !!formData.baseUrl.trim() && (!showApiKey || !!formData.apiKey.trim() || isLikelyLocalBaseUrl(formData.baseUrl));
  const canDetectEndpointType = canFetchModels && !usesOAuthProxy;

  const handleFetchModels = async (openOnSuccess = true) => {
    const request = ++modelRequest.current;
    setFetchingModels(true); setFetchModelsError('');
    try {
      const response = usesOAuthProxy
        ? await fetchCodexModels(formData.codexAccountId.trim() || undefined)
        : await fetchProviderModels({ baseUrl: formData.baseUrl.trim(), apiKey: formData.apiKey.trim(), isFullUrl: formData.isFullUrl, modelsUrl: selectedPreset?.modelsUrl });
      if (request !== modelRequest.current) return;
      if (response.models.length === 0) { setFetchModelsError('No models returned from this endpoint.'); return; }
      setFetchedModels(response.models); setModelsFor(modelsContext);
      if (openOnSuccess) setPickerOpen(true);
    } catch (e) { if (request === modelRequest.current) setFetchModelsError(e instanceof Error ? e.message : 'Failed to fetch models'); }
    finally { if (request === modelRequest.current) setFetchingModels(false); }
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
    <div className="dialog-overlay fixed inset-0 z-50 flex items-center justify-center bg-black/60 px-4 py-6">
      <Card ref={dialogRef} tabIndex={-1} role="dialog" aria-modal="true" aria-labelledby={titleId} className="dialog-panel w-full max-w-2xl [contain:layout_paint]">
        <CardContent className="dialog-body [scrollbar-gutter:stable]">
          <div className="mb-5">
            <h2 id={titleId} className="text-xl font-semibold leading-7 text-foreground">
              {editingId ? 'Edit Provider' : selectedPreset ? `Add ${selectedPreset.name}` : 'Add Custom Provider'}
            </h2>
            <p className="mt-1 text-sm leading-5 text-muted-foreground">
              Keep routing credentials local to this admin service.
            </p>
          </div>

          {!editingId && !selectedPreset && (
            <div className="mb-6 space-y-4">
              <PresetSelector onSelect={(preset) => { if (canDiscard()) onPresetSelect(preset); }} />
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
                <ProviderLogo icon={selectedPreset.icon} />
                <span className="text-[15px] font-semibold">{selectedPreset.name}</span>
                <Badge variant="outline">{usesOAuthProxy ? 'OAuth Account' : noAuth ? 'Subscription' : 'API Key'}</Badge>
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
              if (!saving && !previewing && !pickerOpen) onSubmit({ ...formData, apiFormat: usesOAuthProxy ? 'openai_responses' : formData.apiFormat });
            }}
            className="space-y-5"
            noValidate
          >
            {error && (
              <div className="rounded-2xl border border-destructive/30 bg-destructive/10 px-3 py-2 text-sm leading-5 text-destructive">
                {error}
              </div>
            )}

            <fieldset disabled={saving || previewing || pickerOpen} className="contents">
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
              {noAuth ? (
                <div className="rounded-2xl border border-primary/20 bg-primary/10 px-3 py-2 text-sm leading-5 text-primary">
                  Uses your logged-in Claude subscription. No API key or base URL is needed —
                  switching to this provider clears any third-party takeover and restores the
                  official login.
                </div>
              ) : showApiKey ? (
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

            <FormSection title="Models" action={<Button type="button" size="sm" variant="outline" disabled={fetchingModels || !canFetchModels} onClick={() => { if (availableModels.length > 0) setPickerOpen(true); else void handleFetchModels(); }}>{fetchingModels ? <Loader2 className="mr-2 h-4 w-4 animate-spin" /> : <Download className="mr-2 h-4 w-4" />}{availableModels.length ? 'Choose model' : 'Fetch models'}</Button>}>
              {usesOAuthProxy && <p className="text-xs text-muted-foreground">{canFetchModels ? 'Fetch models uses the selected OAuth account. The local route does not need to be running.' : 'Connect an account on the OAuth page and choose a connected account here to fetch models.'}</p>}
              {fetchModelsError && !pickerOpen && <p role="alert" className="text-xs text-destructive">{fetchModelsError}</p>}
              <div className="grid min-w-0 gap-4 sm:grid-cols-2">
                <ModelMappingInput
                  id="model-main"
                  label="Main"
                  value={formData.model}
                  options={modelOptions}
                  disabled={saving || previewing || pickerOpen}
                  onChange={(value) => setFormData({ ...formData, model: value })}
                />
                <ModelMappingInput
                  id="model-haiku"
                  label="Haiku"
                  value={formData.haikuModel}
                  options={modelOptions}
                  disabled={saving || previewing || pickerOpen}
                  onChange={(value) => setFormData({ ...formData, haikuModel: value })}
                />
                <ModelMappingInput
                  id="model-sonnet"
                  label="Sonnet"
                  value={formData.sonnetModel}
                  options={modelOptions}
                  disabled={saving || previewing || pickerOpen}
                  onChange={(value) => setFormData({ ...formData, sonnetModel: value })}
                />
                <ModelMappingInput
                  id="model-opus"
                  label="Opus"
                  value={formData.opusModel}
                  options={modelOptions}
                  disabled={saving || previewing || pickerOpen}
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
                {usesOAuthProxy ? <CodexDiagnostics
                  accountId={formData.codexAccountId.trim() || undefined}
                  resolvedAccountId={modelAccountId}
                  canCheck={hasModelAccount}
                  disabled={saving || previewing || pickerOpen}
                  configured={{ model: formData.model, haikuModel: formData.haikuModel, sonnetModel: formData.sonnetModel, opusModel: formData.opusModel }}
                /> : (
                <div className="rounded-xl border border-white/[0.07] bg-white/[0.025] px-4 py-3">
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
                )}
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
                    disabled={usesOAuthProxy}
                    aria-describedby="api-format-help"
                    value={usesOAuthProxy ? 'openai_responses' : formData.apiFormat}
                    onChange={(e) => setFormData({ ...formData, apiFormat: e.target.value as ApiFormat })}
                  >
                    <option value="anthropic">Anthropic Messages</option>
                    <option value="openai_chat">OpenAI Chat</option>
                    <option value="openai_responses">OpenAI Responses</option>
                    <option value="gemini_native">Gemini Native (local route unavailable)</option>
                    {!['anthropic', 'openai_chat', 'openai_responses', 'gemini_native'].includes(formData.apiFormat) && <option value={formData.apiFormat}>{formData.apiFormat || 'Unknown protocol'} (local route unavailable)</option>}
                  </Select>
                  <p id="api-format-help" className="text-xs leading-5 text-muted-foreground">{usesOAuthProxy
                    ? 'Codex OAuth uses OpenAI Responses through the local route.'
                    : formData.apiFormat === 'gemini_native'
                      ? 'Gemini Native can be saved for configuration and model discovery. Local routing is not supported.'
                      : 'Choose the upstream endpoint format. Claude Code connects to the local route using Anthropic Messages.'}</p>
                </div>

              </div>

              <div className="space-y-2">
                <Label htmlFor="context-window">Context window (tokens)</Label>
                <Input
                  id="context-window"
                  inputMode="numeric"
                  aria-describedby="context-window-help"
                  value={formData.contextWindow}
                  onChange={(e) => setFormData({ ...formData, contextWindow: e.target.value })}
                  placeholder="Optional: confirmed gateway/model limit"
                />
                <p id="context-window-help" className="text-xs leading-5 text-muted-foreground">
                  For unrecognized custom model IDs in Claude Code. Enter the gateway's confirmed token limit;
                  use the smallest supported window if mappings differ. Blank sets no provider override.
                  Known Claude IDs and local-route aliases follow their own window rules.
                  This does not change the upstream model or its capabilities.
                </p>
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

              <p className="text-xs text-muted-foreground">
                API timeout and nonessential traffic are user settings. Edit them in
                the live Claude Code configuration; provider switching preserves them.
                {' '}<a href="#settings" className="underline text-primary">Open Settings</a>
              </p>

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

            </fieldset>
            {previewError && <p role="alert" className="text-sm text-destructive">{previewError}</p>}
            {preview && <div className="rounded-xl border border-border p-4 space-y-3">
              <h3 className="text-sm font-semibold">Save preview</h3>
              <p className="text-sm text-muted-foreground">{preview.message}</p>
              <div className="text-xs space-y-2"><p className="break-all font-mono">{preview.recordPath}</p><p>{preview.recordKeys.length ? 'Provider fields that change:' : 'No provider fields change.'}</p><ul className="break-all font-mono">{preview.recordKeys.slice(0, 60).map(key => <li key={key}>{key}</li>)}</ul>{preview.recordKeys.length > 60 && <p>{preview.recordKeys.length - 60} additional fields</p>}</div>
              {preview.livePath && <div className="text-xs space-y-2"><p className="break-all font-mono">{preview.livePath}</p><p>{preview.liveKeys.length ? 'Live fields that change:' : 'No live fields change.'}</p><ul className="break-all font-mono">{preview.liveKeys.slice(0, 60).map(key => <li key={key}>{key}</li>)}</ul></div>}
              <p className="text-xs text-muted-foreground">Credential values are hidden. User settings are preserved. Preview changes no files; Save checks the current configuration again.</p>
            </div>}
            <div className="dialog-actions">
              <Button type="button" variant="ghost" onClick={requestCancel} disabled={saving || previewing}>
                Cancel
              </Button>
              {onPreview && <Button type="button" variant="outline" disabled={saving || previewing} onClick={() => void inspectChanges()}>{previewing ? 'Checking...' : 'Preview save'}</Button>}
              <Button type="submit" disabled={saving || previewing}>
                {saving ? 'Saving...' : 'Save'}
              </Button>
            </div>
          </form>
        </CardContent>
      </Card>
      {pickerOpen && modelsFor === modelsContext && <ModelPickerDialog models={availableModels} configured={{model:formData.model,haikuModel:formData.haikuModel,sonnetModel:formData.sonnetModel,opusModel:formData.opusModel}} loading={fetchingModels} error={fetchModelsError} onRefresh={() => void handleFetchModels(false)} onClose={() => setPickerOpen(false)} onApply={(role,id) => { setFormData({ ...formData, [role]:id }); setPickerOpen(false); }} />}
    </div>
  );
}

function FormSection({ title, children, action }: { title: string; children: ReactNode; action?: ReactNode }) {
  return (
    <section className="space-y-3 rounded-2xl border border-white/10 bg-white/[0.025] p-4">
      <div className="flex items-center justify-between gap-3"><h3 className="text-sm font-medium leading-5 text-muted-foreground">{title}</h3>{action}</div>
      {children}
    </section>
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

import { ManagedImportDialog } from '@/components/ManagedImportDialog';
import { restoreRemovedItems } from '@/api';
import { useDialog } from '@/lib/useDialog';
import { ToastNotice } from '@/components/ToastNotice';
import { useConfirm } from '@/lib/useConfirm';
import { DeletionRecoveryPanel } from '@/components/DeletionRecoveryPanel';
import { OperationResults } from '@/components/OperationResults';
import type { OperationReport } from '@/api';
import { useCallback, useEffect, useId, useState } from 'react';
import { Plus, RefreshCw, Trash2, Server, Download, Pencil } from 'lucide-react';
import {
  ApiError,
  listMcpServers,
  saveMcpServer,
  deleteMcpServer,
  syncMcpServers,
  importMcpServers,
  toggleMcpServer,
  type McpServer,
} from '../api';
import { PageHeader } from '@/components/PageHeader';
import { Button } from '@/components/ui/button';
import { Card, CardContent, CardHeader, CardTitle, CardDescription } from '@/components/ui/card';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Textarea } from '@/components/ui/textarea';
import { Switch } from '@/components/ui/switch';
import { cacheGet, cacheSet } from '@/lib/fetchCache';
import { useUnsavedChanges } from '@/lib/useUnsavedChanges';

function emptyForm(): McpServer {
  return {
    id: '',
    name: '',
    serverSpec: { command: '', args: [] },
    appType: 'claude_code',
    enabled: true,
  };
}

export default function McpPage() {
  const cached = cacheGet<McpServer[]>('mcp-servers');
  const { confirmAction, confirmation } = useConfirm();
  const [importMode, setImportMode] = useState<'Import' | 'Restore' | null>(null);
  const [servers, setServers] = useState<McpServer[]>(cached ?? []);
  const [loading, setLoading] = useState(!cached);
  const [error, setError] = useState('');
  const [showForm, setShowForm] = useState(false);
  const [form, setForm] = useState(emptyForm());
  const [saving, setSaving] = useState(false);
  const [formError, setFormError] = useState('');
  const [specText, setSpecText] = useState('');
  const [editingId, setEditingId] = useState<string | null>(null);
  const [pendingAction, setPendingAction] = useState<string | null>(null);
  const [notice, setNotice] = useState('');
  const [deletionRefresh, setDeletionRefresh] = useState(0);
  const [hasPendingDeletion, setHasPendingDeletion] = useState(false);
  const [operation, setOperation] = useState<{ report: OperationReport; action: 'Import' | 'Sync' | 'Restore' } | null>(null);
  const [baseline, setBaseline] = useState('');
  const draft = JSON.stringify([form, specText]);
  const canDiscard = useUnsavedChanges(showForm && draft !== baseline, saving || pendingAction !== null);

  const formTitleId = useId();
  const closeForm = () => { if (canDiscard()) setShowForm(false); };
  const formDialogRef = useDialog(showForm, closeForm, !saving && pendingAction === null);

  const loadServers = useCallback(async (signal?: AbortSignal) => {
    try {
      const data = await listMcpServers({ signal });
      setServers(data.servers);
      cacheSet('mcp-servers', data.servers);
      return true;
    } catch (e) {
      if (e instanceof DOMException && e.name === 'AbortError') return false;
      setError(e instanceof Error ? e.message : 'Failed to load');
      return false;
    } finally {
      if (!signal?.aborted) setLoading(false);
    }
  }, []);

  useEffect(() => {
    const ctrl = new AbortController();
    loadServers(ctrl.signal);
    return () => ctrl.abort();
  }, [loadServers]);

  const handleSave = async () => {
    setSaving(true);
    setFormError('');
    try {
      let spec: unknown;
      try {
        spec = JSON.parse(specText || '{"command":"","args":[]}');
      } catch {
        setFormError('serverSpec must be valid JSON');
        return;
      }
      if (!form.id.trim()) {
        setFormError('ID is required');
        return;
      }
      if (!spec || typeof spec !== 'object' || Array.isArray(spec)) {
        setFormError('serverSpec must be a JSON object');
        return;
      }

      const server: McpServer = {
        ...form,
        serverSpec: spec,
        appType: 'claude_code',
        enabled: editingId ? form.enabled : true,
      };

      await saveMcpServer(server);
      setError('');
      setNotice('MCP server saved and synced to Claude Code.');
      setShowForm(false);
      setForm(emptyForm());
      setSpecText('');
      setEditingId(null);
      await loadServers();
    } catch (e) {
      if (e instanceof ApiError && e.saved) {
        setBaseline(draft);
        await loadServers();
      }
      setFormError(e instanceof Error ? e.message : 'Save failed');
    } finally {
      setSaving(false);
    }
  };

  const handleDelete = async (id: string) => {
    if (!await confirmAction('Remove this MCP server from management and Claude Code? Future imports will skip it until you restore it.', { title: 'Remove managed item?', confirmLabel: 'Remove', destructive: true })) return;
    try {
      setPendingAction(`delete:${id}`);
      setError('');
      setNotice(''); setOperation(null);
      await deleteMcpServer(id);
      await loadServers();
    } catch (e) {
      // A lost response or partial cleanup can follow a committed deletion.
      const message = e instanceof Error ? e.message : 'Delete failed';
      await loadServers();
      setError(message);
    } finally {
      setDeletionRefresh((value) => value + 1);
      setPendingAction(null);
    }
  };

  const handleSync = async () => {
    setPendingAction('sync');
    setNotice(''); setOperation(null); setError('');
    try {
      const report = await syncMcpServers();
      setOperation({ report, action: 'Sync' });
    } catch (e) { setError(e instanceof Error ? e.message : 'Sync failed'); }
    finally { setPendingAction(null); }
  };

  const handleImport = async (ids: string[]) => {
    setPendingAction('import');
    setNotice(''); setOperation(null); setError('');
    try {
      const report = await importMcpServers(ids);
      setOperation({ report, action: 'Import' });
      if (!await loadServers()) setError((previous) => `Import results are retained, but refreshing records failed: ${previous}`);
    } catch (e) {
      // A lost response can still follow committed imports; refresh observed records.
      const message = e instanceof Error ? e.message : 'Import failed';
      const refreshed = await loadServers();
      setError((previous) => refreshed ? message : `${message}. Refresh failed: ${previous}`);
    } finally { setPendingAction(null); }
  };

  const handleRestore = async (ids: string[]) => {
    setPendingAction('restore');
    setNotice(''); setOperation(null); setError('');
    try {
      const report = await restoreRemovedItems('mcp', ids);
      setOperation({ report, action: 'Restore' });
      if (!await loadServers()) setError(previous => `Restore results are retained, but refreshing records failed: ${previous}`);
    } catch (e) {
      await loadServers();
      throw e;
    } finally { setPendingAction(null); }
  };

  const handleToggle = async (srv: McpServer) => {
    try {
      setPendingAction(`toggle:${srv.id}`);
      setError('');
      setNotice(''); setOperation(null);
      const res = await toggleMcpServer(srv.id);
      setNotice('MCP server state saved and synced to Claude Code.');
      setServers((prev) => {
        const next = prev.map((server) =>
          server.id === srv.id ? { ...server, enabled: res.enabled } : server,
        );
        cacheSet('mcp-servers', next);
        return next;
      });
    } catch (e) {
      if (e instanceof ApiError && e.saved) await loadServers();
      setError(e instanceof Error ? e.message : 'Toggle failed');
    } finally {
      setPendingAction(null);
    }
  };

  const openCreate = () => {
    if (showForm && !canDiscard()) return;
    const nextForm = emptyForm();
    const nextSpec = JSON.stringify(nextForm.serverSpec, null, 2);
    setBaseline(JSON.stringify([nextForm, nextSpec]));
    setForm(nextForm);
    setSpecText(nextSpec);
    setFormError('');
    setEditingId(null);
    setShowForm(true);
  };

  const openEdit = (server: McpServer) => {
    if (showForm && !canDiscard()) return;
    const nextSpec = JSON.stringify(server.serverSpec, null, 2);
    setBaseline(JSON.stringify([server, nextSpec]));
    setForm(server);
    setSpecText(nextSpec);
    setFormError('');
    setEditingId(server.id);
    setShowForm(true);
  };

  if (loading) {
    return (
      <div className="space-y-6">
        <PageHeader title="MCP Servers" description="Manage MCP server configurations" />
        <div className="flex items-center justify-center py-20 text-muted-foreground">Loading...</div>
      </div>
    );
  }

  return (
    <div className="space-y-6">
      {confirmation}
      {importMode && <ManagedImportDialog kind="mcp" mode={importMode} onClose={() => setImportMode(null)} onApply={importMode === 'Import' ? handleImport : handleRestore} />}
      <PageHeader
        title="MCP Servers"
        description="Manage servers available to Claude Code."
        action={
          <div className="flex flex-wrap items-center gap-2">
            <Button variant="outline" onClick={() => setImportMode('Import')} disabled={pendingAction !== null || saving || hasPendingDeletion} className="gap-1.5">
              <Download className="h-3.5 w-3.5" /> {pendingAction === 'import' ? 'Importing...' : 'Import'}
            </Button>
            <Button variant="outline" onClick={() => setImportMode('Restore')} disabled={pendingAction !== null || saving || hasPendingDeletion}>Restore removed</Button>
            <Button variant="outline" onClick={handleSync} disabled={pendingAction !== null || saving || hasPendingDeletion} className="gap-1.5">
              <RefreshCw className="h-3.5 w-3.5" /> {pendingAction === 'sync' ? 'Syncing...' : 'Sync'}
            </Button>
            <Button onClick={openCreate} disabled={pendingAction !== null || saving || hasPendingDeletion} className="gap-1.5">
              <Plus className="h-3.5 w-3.5" /> Add
            </Button>
          </div>
        }
      />

      {error && <ToastNotice tone="error" message={error} onDismiss={() => setError('')} />}
      {notice && <ToastNotice message={notice} onDismiss={() => setNotice('')} />}
      {operation && <OperationResults report={operation.report} action={operation.action} />}
      <DeletionRecoveryPanel refreshKey={deletionRefresh} onPendingChange={setHasPendingDeletion} disabled={saving || pendingAction !== null || showForm} onBusyChange={(value) => setPendingAction(value ? 'recover' : null)} onResolved={async () => { setOperation(null); await loadServers(); }} />

      {servers.length === 0 ? (
        <Card>
          <CardContent className="flex flex-col items-center justify-center py-16 text-center">
            <Server className="mb-3 h-10 w-10 text-muted-foreground/40" />
            <p className="text-sm font-medium text-muted-foreground">No MCP servers configured</p>
            <p className="mt-1 text-xs text-muted-foreground/60">Add MCP servers to sync them to ~/.claude.json</p>
          </CardContent>
        </Card>
      ) : (
        <Card>
          <CardHeader>
            <CardTitle>Your servers <span className="ml-1 font-normal text-muted-foreground">{servers.length}</span></CardTitle>
            <CardDescription>Enabled servers are applied to Claude Code.</CardDescription>
          </CardHeader>
          <div className="divide-y divide-white/[0.07]">
          {servers.map((srv) => (
            <Card key={srv.id} className="rounded-none border-0 shadow-none transition-colors hover:bg-white/[0.025]">
              <CardContent className="flex items-start justify-between gap-3 p-5 sm:px-6">
                <div className="min-w-0 space-y-1">
                  <div className="flex flex-wrap items-center gap-2">
                    <span className="font-semibold text-sm">{srv.name || srv.id}</span>
                    <Switch checked={srv.enabled} aria-label={`${srv.enabled ? 'Disable' : 'Enable'} ${srv.name || srv.id}`} disabled={pendingAction !== null || saving || hasPendingDeletion} onCheckedChange={() => handleToggle(srv)} />
                  </div>
                  <p className="text-xs text-muted-foreground truncate font-mono">{srv.id}</p>
                </div>
                <div className="flex items-center gap-1">
                  <Button
                    aria-label={`Edit ${srv.name || srv.id}`}
                    variant="ghost"
                    size="icon"
                    onClick={() => openEdit(srv)}
                    disabled={pendingAction !== null || saving || hasPendingDeletion}
                    className="h-9 w-9 rounded-lg text-muted-foreground hover:text-foreground"
                  >
                    <Pencil className="h-4 w-4" />
                  </Button>
                  <Button
                    aria-label={`Remove ${srv.name || srv.id}`}
                    variant="ghost"
                    size="icon"
                    onClick={() => handleDelete(srv.id)}
                    disabled={pendingAction !== null || saving || hasPendingDeletion}
                    className="h-9 w-9 rounded-lg text-muted-foreground hover:text-red-400"
                  >
                    <Trash2 className="h-4 w-4" />
                  </Button>
                </div>
              </CardContent>
            </Card>
          ))}
        </div>
        </Card>
      )}

      {showForm && (
        <div className="dialog-overlay fixed inset-0 z-50 flex items-center justify-center bg-black/60 px-4 py-6">
        <Card ref={formDialogRef} tabIndex={-1} role="dialog" aria-modal="true" aria-labelledby={formTitleId} className="dialog-panel w-full max-w-lg">
          <CardHeader>
            <CardTitle id={formTitleId} className="text-xl leading-7">{editingId ? 'Edit MCP Server' : 'Add MCP Server'}</CardTitle>
            <CardDescription>Save applies enabled servers to Claude Code.</CardDescription>
          </CardHeader>
          <CardContent className="space-y-5">
            <div className="space-y-2">
              <Label htmlFor="mcp-id">ID</Label>
              <Input
                id="mcp-id"
                placeholder="e.g. context7"
                value={form.id}
                onChange={(e) => setForm({ ...form, id: e.target.value })}
                disabled={editingId !== null || saving || pendingAction !== null || hasPendingDeletion}
              />
            </div>
            <div className="space-y-2">
              <Label htmlFor="mcp-name">Name</Label>
              <Input
                id="mcp-name"
                disabled={saving || pendingAction !== null || hasPendingDeletion}
                placeholder="e.g. Context7"
                value={form.name}
                onChange={(e) => setForm({ ...form, name: e.target.value })}
              />
            </div>
            <div className="space-y-2">
              <Label htmlFor="mcp-spec">Server Spec (JSON)</Label>
              <Textarea
                id="mcp-spec"
                disabled={saving || pendingAction !== null || hasPendingDeletion}
                placeholder='{"command":"npx","args":["-y","@upstash/context7-mcp"]}'
                value={specText}
                onChange={(e) => setSpecText(e.target.value)}
                className="min-h-[120px] font-mono text-xs"
              />
            </div>
            {formError && (
              <div className="rounded-lg border border-red-500/20 bg-red-500/10 px-3 py-2 text-xs text-red-400">{formError}</div>
            )}
            <div className="dialog-actions">
              <Button variant="outline" onClick={() => { if (canDiscard()) setShowForm(false); }} disabled={saving}>Cancel</Button>
              <Button onClick={handleSave} disabled={saving || pendingAction !== null || hasPendingDeletion}>
                {saving ? 'Saving...' : 'Save'}
              </Button>
            </div>
          </CardContent>
        </Card>
        </div>
      )}
    </div>
  );
}

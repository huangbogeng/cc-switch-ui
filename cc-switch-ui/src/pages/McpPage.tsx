import { DeletionRecoveryPanel } from '@/components/DeletionRecoveryPanel';
import { OperationResults } from '@/components/OperationResults';
import type { OperationReport } from '@/api';
import { useCallback, useEffect, useState } from 'react';
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
import { Card, CardContent } from '@/components/ui/card';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Textarea } from '@/components/ui/textarea';
import { Badge } from '@/components/ui/badge';
import { cacheGet, cacheSet } from '@/lib/fetchCache';
import { ErrorAlert } from '@/components/ErrorAlert';
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
  const [operation, setOperation] = useState<{ report: OperationReport; action: 'Import' | 'Sync' } | null>(null);
  const [baseline, setBaseline] = useState('');
  const draft = JSON.stringify([form, specText]);
  const canDiscard = useUnsavedChanges(showForm && draft !== baseline, saving || pendingAction !== null);

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
    if (!confirm('Delete this managed MCP server and remove it from Claude Code?')) return;
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

  const handleImport = async () => {
    setPendingAction('import');
    setNotice(''); setOperation(null); setError('');
    try {
      const report = await importMcpServers();
      setOperation({ report, action: 'Import' });
      if (!await loadServers()) setError((previous) => `Import results are retained, but refreshing records failed: ${previous}`);
    } catch (e) {
      // A lost response can still follow committed imports; refresh observed records.
      const message = e instanceof Error ? e.message : 'Import failed';
      const refreshed = await loadServers();
      setError((previous) => refreshed ? message : `${message}. Refresh failed: ${previous}`);
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
      <PageHeader
        title="MCP Servers"
        description="Save and enable/disable apply managed servers to Claude Code. Retry Sync after an application error."
        action={
          <div className="flex items-center gap-2">
            <Button variant="outline" size="sm" onClick={handleImport} disabled={pendingAction !== null || saving || hasPendingDeletion} className="gap-1.5">
              <Download className="h-3.5 w-3.5" /> {pendingAction === 'import' ? 'Importing...' : 'Import'}
            </Button>
            <Button variant="outline" size="sm" onClick={handleSync} disabled={pendingAction !== null || saving || hasPendingDeletion} className="gap-1.5">
              <RefreshCw className="h-3.5 w-3.5" /> {pendingAction === 'sync' ? 'Syncing...' : 'Sync'}
            </Button>
            <Button size="sm" onClick={openCreate} disabled={pendingAction !== null || saving || hasPendingDeletion} className="gap-1.5">
              <Plus className="h-3.5 w-3.5" /> Add
            </Button>
          </div>
        }
      />

      {error && (
        <ErrorAlert message={error} />
      )}
      {notice && <div role="status" className="rounded-lg border border-emerald-500/20 bg-emerald-500/10 px-4 py-3 text-sm text-emerald-400">{notice}</div>}
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
        <div className="space-y-3">
          {servers.map((srv) => (
            <Card key={srv.id} className="group transition-all hover:border-primary/20">
              <CardContent className="flex items-start justify-between p-4">
                <div className="min-w-0 space-y-1">
                  <div className="flex items-center gap-2">
                    <span className="font-semibold text-sm">{srv.name || srv.id}</span>
                    <button aria-label={`${srv.enabled ? 'Disable' : 'Enable'} ${srv.name || srv.id}`} disabled={pendingAction !== null || saving || hasPendingDeletion} onClick={() => handleToggle(srv)} className="cursor-pointer">
                      <Badge variant={srv.enabled ? 'default' : 'outline'} className="text-[10px] px-1.5 py-0 hover:opacity-80">
                        {srv.enabled ? 'enabled' : 'disabled'}
                      </Badge>
                    </button>
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
                    className="h-8 w-8 text-muted-foreground hover:text-foreground"
                  >
                    <Pencil className="h-4 w-4" />
                  </Button>
                  <Button
                    aria-label={`Delete ${srv.name || srv.id}`}
                    variant="ghost"
                    size="icon"
                    onClick={() => handleDelete(srv.id)}
                    disabled={pendingAction !== null || saving || hasPendingDeletion}
                    className="h-8 w-8 text-muted-foreground hover:text-red-400"
                  >
                    <Trash2 className="h-4 w-4" />
                  </Button>
                </div>
              </CardContent>
            </Card>
          ))}
        </div>
      )}

      {showForm && (
        <Card className="border-primary/30">
          <CardContent className="space-y-4 p-4">
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
            <div className="flex items-center justify-end gap-2">
              <Button variant="ghost" size="sm" onClick={() => { if (canDiscard()) setShowForm(false); }} disabled={saving}>Cancel</Button>
              <Button size="sm" onClick={handleSave} disabled={saving || pendingAction !== null || hasPendingDeletion}>
                {saving ? 'Saving...' : 'Save'}
              </Button>
            </div>
          </CardContent>
        </Card>
      )}
    </div>
  );
}

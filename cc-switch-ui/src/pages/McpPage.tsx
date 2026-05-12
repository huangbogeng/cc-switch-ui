import { useCallback, useEffect, useState } from 'react';
import { Plus, RefreshCw, Trash2, Server, Download } from 'lucide-react';
import {
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
import { cn } from '@/lib/utils';
import { cacheGet, cacheSet } from '@/lib/fetchCache';

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

  const loadServers = useCallback(async (signal?: AbortSignal) => {
    try {
      const data = await listMcpServers({ signal });
      setServers(data.servers);
      cacheSet('mcp-servers', data.servers);
    } catch (e) {
      if (e instanceof DOMException && e.name === 'AbortError') return;
      setError(e instanceof Error ? e.message : 'Failed to load');
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
        setSaving(false);
        return;
      }

      const server: McpServer = {
        ...form,
        serverSpec: spec,
        appType: 'claude_code',
        enabled: true,
      };

      await saveMcpServer(server);
      setShowForm(false);
      setForm(emptyForm());
      setSpecText('');
      await loadServers();
    } catch (e) {
      setFormError(e instanceof Error ? e.message : 'Save failed');
    } finally {
      setSaving(false);
    }
  };

  const handleDelete = async (id: string) => {
    if (!confirm('Delete this MCP server?')) return;
    try {
      await deleteMcpServer(id);
      await loadServers();
    } catch (e) {
      setError(e instanceof Error ? e.message : 'Delete failed');
    }
  };

  const handleSync = async () => {
    try {
      await syncMcpServers();
      setError('');
    } catch (e) {
      setError(e instanceof Error ? e.message : 'Sync failed');
    }
  };

  const handleImport = async () => {
    try {
      const res = await importMcpServers();
      setError('');
      if (res.imported > 0) {
        await loadServers();
      }
    } catch (e) {
      setError(e instanceof Error ? e.message : 'Import failed');
    }
  };

  const handleToggle = async (srv: McpServer) => {
    try {
      const res = await toggleMcpServer(srv.id);
      setServers((prev) =>
        prev.map((s) => (s.id === srv.id ? { ...s, enabled: res.enabled } : s))
      );
    } catch (e) {
      setError(e instanceof Error ? e.message : 'Toggle failed');
    }
  };

  const openCreate = () => {
    setForm(emptyForm());
    setSpecText(JSON.stringify({ command: '', args: [] }, null, 2));
    setFormError('');
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
        description="MCP server configs synced to ~/.claude.json"
        action={
          <div className="flex items-center gap-2">
            <Button variant="outline" size="sm" onClick={handleImport} className="gap-1.5">
              <Download className="h-3.5 w-3.5" /> Import
            </Button>
            <Button variant="outline" size="sm" onClick={handleSync} className="gap-1.5">
              <RefreshCw className="h-3.5 w-3.5" /> Sync
            </Button>
            <Button size="sm" onClick={openCreate} className="gap-1.5">
              <Plus className="h-3.5 w-3.5" /> Add
            </Button>
          </div>
        }
      />

      {error && (
        <div className="rounded-lg border border-red-500/20 bg-red-500/10 px-4 py-3 text-sm text-red-400">{error}</div>
      )}

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
                    <button onClick={() => handleToggle(srv)} className="cursor-pointer">
                      <Badge variant={srv.enabled ? 'default' : 'outline'} className="text-[10px] px-1.5 py-0 hover:opacity-80">
                        {srv.enabled ? 'enabled' : 'disabled'}
                      </Badge>
                    </button>
                  </div>
                  <p className="text-xs text-muted-foreground truncate font-mono">{srv.id}</p>
                </div>
                <Button
                  variant="ghost"
                  size="icon"
                  onClick={() => handleDelete(srv.id)}
                  className="h-8 w-8 opacity-0 group-hover:opacity-100 transition-opacity text-muted-foreground hover:text-red-400"
                >
                  <Trash2 className="h-4 w-4" />
                </Button>
              </CardContent>
            </Card>
          ))}
        </div>
      )}

      {showForm && (
        <Card className={cn("border-primary/30")}>
          <CardContent className="space-y-4 p-4">
            <div className="space-y-2">
              <Label htmlFor="mcp-id">ID</Label>
              <Input
                id="mcp-id"
                placeholder="e.g. context7"
                value={form.id}
                onChange={(e) => setForm({ ...form, id: e.target.value })}
              />
            </div>
            <div className="space-y-2">
              <Label htmlFor="mcp-name">Name</Label>
              <Input
                id="mcp-name"
                placeholder="e.g. Context7"
                value={form.name}
                onChange={(e) => setForm({ ...form, name: e.target.value })}
              />
            </div>
            <div className="space-y-2">
              <Label htmlFor="mcp-spec">Server Spec (JSON)</Label>
              <Textarea
                id="mcp-spec"
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
              <Button variant="ghost" size="sm" onClick={() => setShowForm(false)}>Cancel</Button>
              <Button size="sm" onClick={handleSave} disabled={saving}>
                {saving ? 'Saving...' : 'Save'}
              </Button>
            </div>
          </CardContent>
        </Card>
      )}
    </div>
  );
}

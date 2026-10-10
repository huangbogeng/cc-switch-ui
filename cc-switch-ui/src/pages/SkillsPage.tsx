import { ToastNotice } from '@/components/ToastNotice';
import { useConfirm } from '@/lib/useConfirm';
import { DeletionRecoveryPanel } from '@/components/DeletionRecoveryPanel';
import { OperationResults } from '@/components/OperationResults';
import type { OperationReport } from '@/api';
import { useCallback, useDeferredValue, useEffect, useMemo, useState } from 'react';
import { Plus, RefreshCw, Package, Download, Search } from 'lucide-react';
import {
  ApiError,
  listSkills,
  deleteSkill,
  syncSkills,
  importSkills,
  toggleSkill,
  type Skill,
} from '../api';
import { PageHeader } from '@/components/PageHeader';
import { Button } from '@/components/ui/button';
import { Card, CardContent } from '@/components/ui/card';
import { Input } from '@/components/ui/input';
import { SkillGroup } from '@/components/skills/SkillGroup';
import { SkillFormDialog } from '@/components/skills/SkillFormDialog';
import { cacheGet, cacheSet } from '@/lib/fetchCache';
import { SkillBackupsPanel } from '@/components/skills/SkillBackupsPanel';
import { useUnsavedChanges } from '@/lib/useUnsavedChanges';

const DEFAULT_COLLECTION = 'Other';
const SEARCH_BATCH = 120;

function groupByCollection(skills: Skill[]): [string, Skill[]][] {
  const groups = new Map<string, Skill[]>();
  for (const s of skills) {
    const coll = s.collection || DEFAULT_COLLECTION;
    if (!groups.has(coll)) groups.set(coll, []);
    groups.get(coll)!.push(s);
  }
  return [...groups.entries()].sort((a, b) => b[1].length - a[1].length);
}

export default function SkillsPage() {
  const cached = cacheGet<Skill[]>('skills');
  const { confirmAction, confirmation } = useConfirm();
  const [skills, setSkills] = useState<Skill[]>(cached ?? []);
  const [loading, setLoading] = useState(!cached);
  const [error, setError] = useState('');
  const [searchQuery, setSearchQuery] = useState('');
  const deferredSearch = useDeferredValue(searchQuery.trim().toLowerCase());
  const [searchWindow, setSearchWindow] = useState({ query: '', limit: SEARCH_BATCH });
  const [showForm, setShowForm] = useState(false);
  const [syncing, setSyncing] = useState(false);
  const [importing, setImporting] = useState(false);
  const [editingSkill, setEditingSkill] = useState<Skill | null>(null);
  const [pendingSkillId, setPendingSkillId] = useState<string | null>(null);
  const [notice, setNotice] = useState('');
  const [deletionRefresh, setDeletionRefresh] = useState(0);
  const [hasPendingDeletion, setHasPendingDeletion] = useState(false);
  const [operation, setOperation] = useState<{ report: OperationReport; action: 'Import' | 'Sync' } | null>(null);
  const [recovering, setRecovering] = useState(false);
  const operationBusy = syncing || importing || pendingSkillId !== null || recovering;
  const busy = operationBusy || hasPendingDeletion;
  useUnsavedChanges(false, operationBusy);

  const loadSkills = useCallback(async (signal?: AbortSignal) => {
    try {
      const data = await listSkills({ signal });
      setSkills(data.skills);
      cacheSet('skills', data.skills);
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
    loadSkills(ctrl.signal);
    return () => ctrl.abort();
  }, [loadSkills]);

  const filtered = useMemo(() => {
    if (!deferredSearch) return skills;
    const q = deferredSearch;
    return skills.filter(
      (s) =>
        s.name.toLowerCase().includes(q) ||
        (s.description ?? '').toLowerCase().includes(q) ||
        s.directory.toLowerCase().includes(q) ||
        (s.collection ?? '').toLowerCase().includes(q),
    );
  }, [skills, deferredSearch]);

  const isSearching = deferredSearch.length > 0;
  const searchLimit = searchWindow.query === deferredSearch ? searchWindow.limit : SEARCH_BATCH;
  const visibleResults = useMemo(() => isSearching ? filtered.slice(0, searchLimit) : filtered, [filtered, isSearching, searchLimit]);
  const groups = useMemo(() => groupByCollection(visibleResults), [visibleResults]);
  const updatingSearch = searchQuery.trim().toLowerCase() !== deferredSearch;
  const handleEdit = useCallback((skill: Skill) => { setEditingSkill(skill); setShowForm(true); }, []);

  const handleDelete = useCallback(async (id: string) => {
    if (!await confirmAction('Delete this managed skill? Its live directory will be backed up; source files are retained.', { title: 'Delete managed item?', confirmLabel: 'Delete', destructive: true })) return;
    try {
      setPendingSkillId(id);
      setError('');
      setNotice(''); setOperation(null);
      await deleteSkill(id);
      await loadSkills();
    } catch (e) {
      // A lost response or partial cleanup can follow a committed deletion.
      const message = e instanceof Error ? e.message : 'Delete failed';
      await loadSkills();
      setError(message);
    } finally {
      setDeletionRefresh((value) => value + 1);
      setPendingSkillId(null);
    }
  }, [loadSkills, confirmAction]);

  const handleSync = async () => {
    setSyncing(true);
    setNotice(''); setOperation(null); setError('');
    try {
      const report = await syncSkills();
      setOperation({ report, action: 'Sync' });
    } catch (e) { setError(e instanceof Error ? e.message : 'Sync failed'); }
    finally { setSyncing(false); }
  };

  const handleImport = async () => {
    setImporting(true);
    setNotice(''); setOperation(null); setError('');
    try {
      const report = await importSkills();
      setOperation({ report, action: 'Import' });
      if (!await loadSkills()) setError((previous) => `Import results are retained, but refreshing records failed: ${previous}`);
    } catch (e) {
      // A lost response can still follow committed imports; refresh observed records.
      const message = e instanceof Error ? e.message : 'Import failed';
      const refreshed = await loadSkills();
      setError((previous) => refreshed ? message : `${message}. Refresh failed: ${previous}`);
    } finally { setImporting(false); }
  };

  const handleToggle = useCallback(async (skill: Skill) => {
    try {
      setPendingSkillId(skill.id);
      setError('');
      setNotice(''); setOperation(null);
      const res = await toggleSkill(skill.id);
      setNotice('Skill state saved and synced to Claude Code.');
      setSkills((prev) => {
        const next = prev.map((item) =>
          item.id === skill.id ? { ...item, enabled: res.enabled } : item,
        );
        cacheSet('skills', next);
        return next;
      });
    } catch (e) {
      if (e instanceof ApiError && e.saved) await loadSkills();
      setError(e instanceof Error ? e.message : 'Toggle failed');
    } finally {
      setPendingSkillId(null);
    }
  }, [loadSkills]);

  if (loading) {
    return (
      <div className="space-y-6">
        <PageHeader title="Skills" description="Manage installed skills" />
        <div className="flex items-center justify-center py-20 text-muted-foreground">Loading...</div>
      </div>
    );
  }

  return (
    <div className="space-y-6">
      {confirmation}
      <PageHeader
        title="Skills"
        description="Manage skills available to Claude Code."
        action={
          <div className="flex flex-wrap items-center gap-2">
            <Button variant="outline" onClick={handleImport} className="gap-1.5" disabled={busy || showForm}>
              <Download className={importing ? 'animate-spin h-3.5 w-3.5' : 'h-3.5 w-3.5'} /> {importing ? 'Importing...' : 'Import'}
            </Button>
            <Button variant="outline" onClick={handleSync} className="gap-1.5" disabled={busy || showForm}>
              <RefreshCw className={syncing ? 'animate-spin h-3.5 w-3.5' : 'h-3.5 w-3.5'} /> {syncing ? 'Syncing...' : 'Sync'}
            </Button>
            <Button onClick={() => { setEditingSkill(null); setShowForm(true); }} disabled={busy || showForm} className="gap-1.5">
              <Plus className="h-3.5 w-3.5" /> Add
            </Button>
          </div>
        }
      />

      {error && <ToastNotice tone="error" message={error} onDismiss={() => setError('')} />}
      {notice && <ToastNotice message={notice} onDismiss={() => setNotice('')} />}
      {operation && <OperationResults report={operation.report} action={operation.action} />}
      <DeletionRecoveryPanel refreshKey={deletionRefresh} onPendingChange={setHasPendingDeletion} disabled={operationBusy || showForm} onBusyChange={setRecovering} onResolved={async () => { setOperation(null); await loadSkills(); }} />

      <SkillBackupsPanel disabled={busy || showForm} onRestored={async () => { setOperation(null); await loadSkills(); }} onBusyChange={setRecovering} />

      <div className="relative">
        <Search className="absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-muted-foreground/50" />
        <Input
          aria-label="Search skills"
          placeholder="Search name, description, directory or collection..."
          value={searchQuery}
          onChange={(e) => setSearchQuery(e.target.value)}
          className="pl-9"
        />
      </div>

      {isSearching && <p role="status" className="mt-2 text-xs text-muted-foreground">{updatingSearch ? 'Updating results…' : `${filtered.length} matching skills · ${visibleResults.length} loaded`}</p>}
      {skills.length === 0 ? (
        <Card>
          <CardContent className="flex flex-col items-center justify-center py-16 text-center">
            <Package className="mb-3 h-10 w-10 text-muted-foreground/40" />
            <p className="text-sm font-medium text-muted-foreground">No skills installed</p>
            <p className="mt-1 text-xs text-muted-foreground/60">Add skills to sync them to ~/.claude/skills/</p>
          </CardContent>
        </Card>
      ) : groups.length === 0 ? (
        <Card>
          <CardContent className="flex flex-col items-center justify-center py-16 text-center">
            <Search className="mb-3 h-10 w-10 text-muted-foreground/40" />
            <p className="text-sm font-medium text-muted-foreground">No skills match "{searchQuery}"</p>
            <p className="mt-1 text-xs text-muted-foreground/60">Try a different search term</p>
          </CardContent>
        </Card>
      ) : (
        <div className="mt-3 max-h-[70dvh] overflow-y-auto overscroll-contain [scrollbar-gutter:stable]" aria-busy={updatingSearch}>
          <div className="space-y-2">
            {groups.map(([collection, collSkills]) => (
              <SkillGroup
                key={collection}
                collection={collection}
                skills={collSkills}
                isSearching={isSearching}
                onToggle={handleToggle}
                onDelete={handleDelete}
                onEdit={handleEdit}
                busySkillId={pendingSkillId}
                busy={busy || showForm}
              />
            ))}
            {isSearching && visibleResults.length < filtered.length && (
              <Button variant="outline" className="w-full" onClick={() => setSearchWindow({ query: deferredSearch, limit: searchLimit + SEARCH_BATCH })}>
                Show next {Math.min(SEARCH_BATCH, filtered.length - visibleResults.length)} matches
              </Button>
            )}
          </div>
        </div>
      )}

      <SkillFormDialog
        key={`skill-form-${showForm}-${editingSkill?.id ?? 'new'}`}
        open={showForm}
        initialSkill={editingSkill}
        onClose={() => { setShowForm(false); setEditingSkill(null); }}
        onSaved={async () => { setOperation(null); await loadSkills(); }}
      />
    </div>
  );
}

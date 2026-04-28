import { useEffect, useState } from 'react';
import { listProviders, saveProvider, deleteProvider, switchProvider, type Provider } from '../api';
import { PresetSelector } from '../components/providers/PresetSelector';
import type { ProviderPreset } from '../config/providerPresets';

export default function ProvidersPage() {
  const [providers, setProviders] = useState<Record<string, Provider>>({});
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState('');

  // Form state
  const [showForm, setShowForm] = useState(false);
  const [editingId, setEditingId] = useState<string | null>(null);
  const [selectedPreset, setSelectedPreset] = useState<ProviderPreset | null>(null);
  const [formData, setFormData] = useState({
    id: '',
    name: '',
    websiteUrl: '',
    notes: '',
    apiKey: '',
  });
  const [saving, setSaving] = useState(false);

  const loadProviders = async () => {
    try {
      const data = await listProviders();
      setProviders(data.providers);
    } catch (e) {
      setError(e instanceof Error ? e.message : 'Failed to load');
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    loadProviders();
  }, []);

  const handlePresetSelect = (preset: ProviderPreset) => {
    setSelectedPreset(preset);
    setEditingId(null);
    setFormData({
      id: preset.id,
      name: preset.name,
      websiteUrl: preset.websiteUrl,
      notes: '',
      apiKey: '',
    });
    setShowForm(true);
  };

  const handleAdd = () => {
    setSelectedPreset(null);
    setEditingId(null);
    setFormData({ id: '', name: '', websiteUrl: '', notes: '', apiKey: '' });
    setShowForm(true);
  };

  const handleEdit = (p: Provider) => {
    setEditingId(p.id);
    setSelectedPreset(null);
    setFormData({
      id: p.id,
      name: p.name,
      websiteUrl: p.websiteUrl || '',
      notes: p.notes || '',
      apiKey: '',
    });
    setShowForm(true);
  };

  const handleDelete = async (id: string) => {
    if (!confirm('Delete this provider?')) return;
    try {
      await deleteProvider(id);
      loadProviders();
    } catch (e) {
      setError(e instanceof Error ? e.message : 'Delete failed');
    }
  };

  const handleSwitch = async (id: string) => {
    try {
      await switchProvider(id);
      loadProviders();
    } catch (e) {
      setError(e instanceof Error ? e.message : 'Switch failed');
    }
  };

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    setSaving(true);
    setError('');

    try {
      let settingsConfig: Record<string, unknown> = {};

      if (selectedPreset) {
        const env: Record<string, string> = {};
        for (const [key, value] of Object.entries(selectedPreset.settingsConfig.env)) {
          if (key === 'ANTHROPIC_AUTH_TOKEN') {
            env[key] = formData.apiKey;
          } else {
            env[key] = value;
          }
        }
        settingsConfig = { env };
      } else if (formData.websiteUrl) {
        settingsConfig = {
          env: {
            ANTHROPIC_BASE_URL: formData.websiteUrl,
            ANTHROPIC_AUTH_TOKEN: formData.apiKey,
          },
        };
      } else {
        settingsConfig = {
          env: {
            ANTHROPIC_AUTH_TOKEN: formData.apiKey,
          },
        };
      }

      const provider: Provider = {
        id: formData.id,
        name: formData.name,
        settingsConfig: settingsConfig,
        websiteUrl: selectedPreset?.websiteUrl || formData.websiteUrl || undefined,
        notes: formData.notes || undefined,
        meta: {},
        inFailoverQueue: false,
      };

      await saveProvider(provider);
      setShowForm(false);
      loadProviders();
    } catch (e) {
      setError(e instanceof Error ? e.message : 'Save failed');
    } finally {
      setSaving(false);
    }
  };

  return (
    <div style={styles.container}>
      <div style={styles.header}>
        <h1 style={styles.title}>Providers</h1>
        <button style={styles.addBtn} onClick={handleAdd}>+ Add Provider</button>
      </div>

      {error && <div style={styles.error}>{error}</div>}

      {loading ? (
        <p style={styles.loading}>Loading...</p>
      ) : (
        <>
          <div style={styles.list}>
            {Object.values(providers).length === 0 && (
              <p style={styles.empty}>No providers yet. Select a preset below to get started.</p>
            )}
            {Object.values(providers).map((p) => (
              <div key={p.id} style={styles.card}>
                <div style={styles.cardHeader}>
                  <span style={styles.providerName}>{p.name}</span>
                  {(p as any).is_current && <span style={styles.currentBadge}>current</span>}
                </div>
                {p.websiteUrl && (
                  <a href={p.websiteUrl} target="_blank" rel="noopener noreferrer" style={styles.url}>
                    {p.websiteUrl}
                  </a>
                )}
                {p.notes && <p style={styles.notes}>{p.notes}</p>}
                <div style={styles.actions}>
                  <button style={styles.switchBtn} onClick={() => handleSwitch(p.id)}>Switch</button>
                  <button style={styles.editBtn} onClick={() => handleEdit(p)}>Edit</button>
                  <button style={styles.deleteBtn} onClick={() => handleDelete(p.id)}>Delete</button>
                </div>
              </div>
            ))}
          </div>
        </>
      )}

      {showForm && (
        <div style={styles.modalOverlay}>
          <div style={styles.modal}>
            <h2 style={styles.modalTitle}>
              {editingId ? 'Edit Provider' : selectedPreset ? `Add ${selectedPreset.name}` : 'Add Custom Provider'}
            </h2>

            {!editingId && !selectedPreset && (
              <>
                <p style={styles.subtitle}>Select a preset or add custom provider</p>
                <PresetSelector onSelect={handlePresetSelect} />
                <div style={styles.divider}>
                  <span style={styles.dividerText}>or enter manually</span>
                </div>
              </>
            )}

            {selectedPreset && (
              <div style={styles.presetInfo}>
                <div style={{ ...styles.presetBadge, background: selectedPreset.iconColor }}>
                  {selectedPreset.name}
                </div>
                {selectedPreset.description && (
                  <p style={styles.presetDesc}>{selectedPreset.description}</p>
                )}
              </div>
            )}

            <form onSubmit={handleSubmit} style={styles.form}>
              <input
                placeholder="Provider ID"
                value={formData.id}
                onChange={(e) => setFormData({ ...formData, id: e.target.value })}
                style={styles.input}
                required
                disabled={!!editingId}
              />
              <input
                placeholder="Name"
                value={formData.name}
                onChange={(e) => setFormData({ ...formData, name: e.target.value })}
                style={styles.input}
                required
              />
              {(selectedPreset || formData.websiteUrl) && (
                <input
                  placeholder="API Key"
                  type="password"
                  value={formData.apiKey}
                  onChange={(e) => setFormData({ ...formData, apiKey: e.target.value })}
                  style={styles.input}
                  required={!!selectedPreset}
                />
              )}
              {!selectedPreset && (
                <input
                  placeholder="Base URL (optional, for custom providers)"
                  value={formData.websiteUrl}
                  onChange={(e) => setFormData({ ...formData, websiteUrl: e.target.value })}
                  style={styles.input}
                />
              )}
              <textarea
                placeholder="Notes (optional)"
                value={formData.notes}
                onChange={(e) => setFormData({ ...formData, notes: e.target.value })}
                style={styles.textarea}
                rows={3}
              />
              <div style={styles.formActions}>
                <button type="button" style={styles.cancelBtn} onClick={() => setShowForm(false)}>
                  Cancel
                </button>
                <button type="submit" style={styles.saveBtn} disabled={saving}>
                  {saving ? 'Saving...' : 'Save'}
                </button>
              </div>
            </form>
          </div>
        </div>
      )}
    </div>
  );
}

const styles: Record<string, React.CSSProperties> = {
  container: { padding: '40px', maxWidth: '900px', margin: '0 auto', background: '#1a1a2e', minHeight: '100vh', color: '#eee', fontFamily: '-apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif' },
  header: { display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: '30px' },
  title: { color: '#00d4ff', margin: 0 },
  addBtn: { padding: '10px 20px', background: '#00d4ff', border: 'none', borderRadius: '6px', cursor: 'pointer', fontWeight: '500', color: '#1a1a2e' },
  error: { color: '#e74c3c', padding: '12px', background: 'rgba(231, 76, 60, 0.1)', borderRadius: '6px', marginBottom: '20px' },
  loading: { color: '#888' },
  empty: { color: '#888', textAlign: 'center', padding: '40px' },
  list: { display: 'flex', flexDirection: 'column', gap: '15px' },
  card: { background: '#16213e', borderRadius: '8px', padding: '20px', border: '1px solid #0f3460' },
  cardHeader: { display: 'flex', alignItems: 'center', gap: '10px', marginBottom: '8px' },
  providerName: { fontWeight: 'bold', fontSize: '1.1em' },
  currentBadge: { fontSize: '0.75em', background: '#2ecc71', color: '#fff', padding: '2px 8px', borderRadius: '10px' },
  url: { color: '#00d4ff', fontSize: '0.9em', textDecoration: 'none' },
  notes: { color: '#888', fontSize: '0.9em', margin: '8px 0' },
  actions: { display: 'flex', gap: '10px', marginTop: '15px' },
  switchBtn: { padding: '6px 16px', background: '#2ecc71', border: 'none', borderRadius: '6px', cursor: 'pointer', color: '#fff', fontSize: '0.9em' },
  editBtn: { padding: '6px 16px', background: 'transparent', border: '1px solid #0f3460', borderRadius: '6px', cursor: 'pointer', color: '#888', fontSize: '0.9em' },
  deleteBtn: { padding: '6px 16px', background: 'transparent', border: '1px solid #e74c3c', borderRadius: '6px', cursor: 'pointer', color: '#e74c3c', fontSize: '0.9em' },
  modalOverlay: { position: 'fixed', inset: 0, background: 'rgba(0,0,0,0.7)', display: 'flex', alignItems: 'center', justifyContent: 'center', zIndex: 100 },
  modal: { background: '#16213e', borderRadius: '8px', padding: '30px', border: '1px solid #0f3460', width: '100%', maxWidth: '500px', maxHeight: '90vh', overflowY: 'auto' },
  modalTitle: { color: '#00d4ff', marginTop: 0, marginBottom: '10px' },
  subtitle: { color: '#888', marginBottom: '15px', fontSize: '0.9em' },
  divider: { display: 'flex', alignItems: 'center', margin: '20px 0' },
  dividerText: { color: '#666', fontSize: '0.85em', padding: '0 10px', background: '#16213e' },
  presetInfo: { marginBottom: '20px', padding: '15px', background: '#0f3460', borderRadius: '8px' },
  presetBadge: { display: 'inline-block', padding: '4px 12px', borderRadius: '12px', color: '#fff', fontSize: '0.85em', fontWeight: 'bold' },
  presetDesc: { color: '#888', margin: '8px 0 0', fontSize: '0.85em' },
  form: { display: 'flex', flexDirection: 'column', gap: '15px' },
  input: { width: '100%', padding: '12px', background: '#0f3460', border: '1px solid #1a4f7a', borderRadius: '6px', color: '#fff', fontSize: '14px', boxSizing: 'border-box' as const },
  textarea: { width: '100%', padding: '12px', background: '#0f3460', border: '1px solid #1a4f7a', borderRadius: '6px', color: '#fff', fontSize: '14px', boxSizing: 'border-box' as const, resize: 'vertical' as const },
  formActions: { display: 'flex', justifyContent: 'flex-end', gap: '10px', marginTop: '10px' },
  cancelBtn: { padding: '10px 20px', background: 'transparent', border: '1px solid #0f3460', borderRadius: '6px', cursor: 'pointer', color: '#888' },
  saveBtn: { padding: '10px 20px', background: '#00d4ff', border: 'none', borderRadius: '6px', cursor: 'pointer', fontWeight: '500', color: '#1a1a2e' },
};

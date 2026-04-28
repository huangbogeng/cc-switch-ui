import { useState } from 'react';

type Language = 'en' | 'zh';
type Theme = 'dark' | 'light';

interface Settings {
  language: Language;
  theme: Theme;
  proxyPort: number;
}

export default function SettingsPage() {
  const [settings, setSettings] = useState<Settings>(() => {
    const saved = localStorage.getItem('ccswitch_settings');
    return saved ? JSON.parse(saved) : {
      language: (localStorage.getItem('ccswitch_language') as Language) || 'en',
      theme: 'dark',
      proxyPort: 15721,
    };
  });
  const [saved, setSaved] = useState(false);

  const handleSave = () => {
    localStorage.setItem('ccswitch_settings', JSON.stringify(settings));
    localStorage.setItem('ccswitch_language', settings.language);
    setSaved(true);
    setTimeout(() => setSaved(false), 2000);
  };

  const handleReset = () => {
    const defaults = { language: 'en' as Language, theme: 'dark' as Theme, proxyPort: 15721 };
    setSettings(defaults);
    localStorage.removeItem('ccswitch_settings');
    localStorage.removeItem('ccswitch_language');
  };

  return (
    <div style={styles.container}>
      <h1 style={styles.header}>Settings</h1>

      {/* Language Settings */}
      <div style={styles.card}>
        <h2 style={styles.cardTitle}>Language / 语言</h2>
        <div style={styles.formGroup}>
          <label style={styles.label}>Interface Language</label>
          <div style={styles.buttonGroup}>
            <button
              style={{ ...styles.toggleButton, ...(settings.language === 'en' ? styles.toggleActive : {}) }}
              onClick={() => setSettings({ ...settings, language: 'en' })}
            >
              English
            </button>
            <button
              style={{ ...styles.toggleButton, ...(settings.language === 'zh' ? styles.toggleActive : {}) }}
              onClick={() => setSettings({ ...settings, language: 'zh' })}
            >
              中文
            </button>
          </div>
        </div>
      </div>

      {/* Proxy Settings */}
      <div style={styles.card}>
        <h2 style={styles.cardTitle}>Proxy Settings</h2>
        <div style={styles.formGroup}>
          <label style={styles.label}>Proxy Listen Port</label>
          <input
            type="number"
            style={styles.input}
            value={settings.proxyPort}
            onChange={(e) => setSettings({ ...settings, proxyPort: parseInt(e.target.value) || 15721 })}
            min={1024}
            max={65535}
          />
          <span style={styles.hint}>Port for the proxy server to listen on (default: 15721)</span>
        </div>
      </div>

      {/* About */}
      <div style={styles.card}>
        <h2 style={styles.cardTitle}>About</h2>
        <div style={styles.aboutInfo}>
          <p><strong>CC Switch Web</strong></p>
          <p style={styles.version}>Version 0.1.0</p>
          <p style={styles.description}>
            A lightweight pure Web architecture Claude Code provider manager.
            Forked from <a href="https://github.com/farion1231/cc-switch" target="_blank" rel="noopener noreferrer" style={styles.link}>cc-switch</a>.
          </p>
        </div>
      </div>

      {/* Actions */}
      <div style={styles.actions}>
        <button style={styles.buttonSecondary} onClick={handleReset}>Reset to Defaults</button>
        <button style={styles.buttonPrimary} onClick={handleSave}>
          {saved ? 'Saved!' : 'Save Settings'}
        </button>
      </div>
    </div>
  );
}

const styles: Record<string, React.CSSProperties> = {
  container: { padding: '40px', maxWidth: '600px', margin: '0 auto', background: '#1a1a2e', minHeight: '100vh', color: '#eee', fontFamily: '-apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif' },
  header: { color: '#00d4ff', marginBottom: '30px' },
  card: { background: '#16213e', borderRadius: '8px', padding: '24px', marginBottom: '20px', border: '1px solid #0f3460' },
  cardTitle: { color: '#00d4ff', marginBottom: '15px', fontSize: '1.1em' },
  formGroup: { marginBottom: '15px' },
  label: { display: 'block', color: '#888', marginBottom: '8px', fontSize: '0.9em' },
  buttonGroup: { display: 'flex', gap: '10px' },
  toggleButton: { padding: '8px 16px', background: '#0f3460', border: '1px solid #0f3460', borderRadius: '6px', cursor: 'pointer', color: '#888', transition: 'all 0.2s' },
  toggleActive: { background: '#00d4ff', color: '#1a1a2e', borderColor: '#00d4ff' },
  input: { width: '100%', padding: '10px', background: '#0f3460', border: '1px solid #0f3460', borderRadius: '6px', color: '#eee', fontSize: '1em', boxSizing: 'border-box' },
  hint: { display: 'block', color: '#666', fontSize: '0.8em', marginTop: '4px' },
  aboutInfo: { color: '#888', lineHeight: 1.6 },
  version: { color: '#00d4ff', margin: '5px 0' },
  description: { marginTop: '10px', lineHeight: 1.5 },
  link: { color: '#00d4ff', textDecoration: 'none' },
  actions: { display: 'flex', gap: '10px', justifyContent: 'flex-end' },
  buttonPrimary: { padding: '10px 20px', background: '#00d4ff', border: 'none', borderRadius: '6px', cursor: 'pointer', fontWeight: '500', color: '#1a1a2e' },
  buttonSecondary: { padding: '10px 20px', background: 'transparent', border: '1px solid #0f3460', borderRadius: '6px', cursor: 'pointer', color: '#888' },
};

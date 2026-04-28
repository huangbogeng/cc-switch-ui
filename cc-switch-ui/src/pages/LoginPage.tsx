import { useState } from 'react';
import { login as apiLogin, setAuthToken } from '../api';

interface Props {
  onLogin: () => void;
}

export default function LoginPage({ onLogin }: Props) {
  const [token, setToken] = useState('');
  const [error, setError] = useState('');
  const [loading, setLoading] = useState(false);

  const handleLogin = async (e: React.FormEvent) => {
    e.preventDefault();
    setError('');
    setLoading(true);
    try {
      const res = await apiLogin(token);
      if (res.success) {
        setAuthToken(token);
        onLogin();
      } else {
        setError(res.message);
      }
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Connection failed');
    } finally {
      setLoading(false);
    }
  };

  return (
    <div style={styles.container}>
      <div style={styles.card}>
        <h1 style={styles.title}>CC Switch</h1>
        <p style={styles.subtitle}>Web Admin Interface</p>
        <form onSubmit={handleLogin} style={styles.form}>
          <input
            type="password"
            value={token}
            onChange={(e) => setToken(e.target.value)}
            placeholder="Enter admin token"
            style={styles.input}
            autoFocus
          />
          {error && <div style={styles.error}>{error}</div>}
          <button type="submit" style={styles.button} disabled={loading}>
            {loading ? 'Logging in...' : 'Login'}
          </button>
        </form>
        <p style={styles.hint}>
          Token is printed in console on first run or set via CC_SWITCH_ADMIN_TOKEN env.
        </p>
      </div>
    </div>
  );
}

const styles: Record<string, React.CSSProperties> = {
  container: {
    minHeight: '100vh',
    display: 'flex',
    alignItems: 'center',
    justifyContent: 'center',
    background: '#1a1a2e',
    color: '#eee',
    fontFamily: '-apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif',
  },
  card: {
    background: '#16213e',
    borderRadius: '8px',
    padding: '40px',
    border: '1px solid #0f3460',
    maxWidth: '400px',
    width: '100%',
  },
  title: {
    color: '#00d4ff',
    margin: '0 0 8px 0',
    fontSize: '2em',
    textAlign: 'center' as const,
  },
  subtitle: {
    color: '#888',
    margin: '0 0 30px 0',
    textAlign: 'center' as const,
  },
  form: {
    display: 'flex',
    flexDirection: 'column' as const,
    gap: '15px',
  },
  input: {
    width: '100%',
    padding: '14px',
    background: '#0f3460',
    border: '1px solid #1a4f7a',
    borderRadius: '6px',
    color: '#fff',
    fontSize: '16px',
    boxSizing: 'border-box' as const,
  },
  button: {
    width: '100%',
    padding: '14px',
    background: '#00d4ff',
    border: 'none',
    borderRadius: '6px',
    color: '#1a1a2e',
    fontWeight: 'bold',
    fontSize: '16px',
    cursor: 'pointer',
  },
  error: {
    color: '#e74c3c',
    padding: '12px',
    background: 'rgba(231, 76, 60, 0.1)',
    borderRadius: '6px',
    fontSize: '14px',
  },
  hint: {
    color: '#666',
    fontSize: '12px',
    marginTop: '20px',
    textAlign: 'center' as const,
  },
};

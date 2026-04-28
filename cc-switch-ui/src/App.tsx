import { useState } from 'react';
import LoginPage from './pages/LoginPage';
import DashboardPage from './pages/DashboardPage';
import ProvidersPage from './pages/ProvidersPage';
import { clearAuthToken } from './api';

type Page = 'dashboard' | 'providers';

export default function App() {
  const [authenticated, setAuthenticated] = useState(!!localStorage.getItem('ccswitch_token'));
  const [currentPage, setCurrentPage] = useState<Page>('dashboard');

  const handleLogin = () => setAuthenticated(true);
  const handleLogout = () => {
    clearAuthToken();
    setAuthenticated(false);
  };

  if (!authenticated) {
    return <LoginPage onLogin={handleLogin} />;
  }

  return (
    <div style={styles.app}>
      <nav style={styles.nav}>
        <span style={styles.navTitle}>CC Switch</span>
        <div style={styles.navLinks}>
          <button
            style={{ ...styles.navBtn, ...(currentPage === 'dashboard' ? styles.navBtnActive : {}) }}
            onClick={() => setCurrentPage('dashboard')}
          >
            Dashboard
          </button>
          <button
            style={{ ...styles.navBtn, ...(currentPage === 'providers' ? styles.navBtnActive : {}) }}
            onClick={() => setCurrentPage('providers')}
          >
            Providers
          </button>
          <button style={styles.navBtn} onClick={handleLogout}>Logout</button>
        </div>
      </nav>
      <main style={styles.main}>
        {currentPage === 'dashboard' && <DashboardPage />}
        {currentPage === 'providers' && <ProvidersPage />}
      </main>
    </div>
  );
}

const styles: Record<string, React.CSSProperties> = {
  app: { minHeight: '100vh', background: '#1a1a2e', color: '#eee', fontFamily: '-apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif' },
  nav: { display: 'flex', alignItems: 'center', justifyContent: 'space-between', padding: '0 40px', height: '56px', background: '#16213e', borderBottom: '1px solid #0f3460' },
  navTitle: { color: '#00d4ff', fontWeight: 'bold', fontSize: '1.2em' },
  navLinks: { display: 'flex', gap: '8px' },
  navBtn: { padding: '8px 16px', background: 'transparent', border: '1px solid transparent', borderRadius: '6px', cursor: 'pointer', color: '#888', fontSize: '14px' },
  navBtnActive: { background: '#0f3460', color: '#00d4ff', borderColor: '#00d4ff' },
  main: {},
};

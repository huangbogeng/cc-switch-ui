import { BrandMark } from '@/components/BrandMark';
import { lazy, Suspense, useEffect, useState } from 'react';
import { LayoutDashboard, Settings, Users, LogOut, Key, BarChart3, Server, Package, type LucideIcon } from 'lucide-react';
import { ServerStatus } from '@/components/ServerStatus';
import LoginPage from './pages/LoginPage';
import { queryClient } from '@/lib/queryClient';
import { cacheClear } from '@/lib/fetchCache';
import { clearAuthToken } from './api';
import { Button } from '@/components/ui/button';
import { cn } from '@/lib/utils';
import { confirmDiscardChanges } from '@/lib/useUnsavedChanges';
import { ErrorBoundary } from '@/components/ErrorBoundary';

const pageLoaders = {
  dashboard: () => import('./pages/DashboardPage'),
  providers: () => import('./pages/ProvidersPage'),
  settings: () => import('./pages/SettingsPage'),
  oauth: () => import('./pages/OAuthPage'),
  usage: () => import('./pages/UsagePage'),
  mcp: () => import('./pages/McpPage'),
  skills: () => import('./pages/SkillsPage'),
};
const DashboardPage = lazy(pageLoaders.dashboard);
const ProvidersPage = lazy(pageLoaders.providers);
const SettingsPage = lazy(pageLoaders.settings);
const OAuthPage = lazy(pageLoaders.oauth);
const UsagePage = lazy(pageLoaders.usage);
const McpPage = lazy(pageLoaders.mcp);
const SkillsPage = lazy(pageLoaders.skills);

function preloadPage(page: Page) {
  void pageLoaders[page]().catch(() => { /* Navigation retries through React.lazy. */ });
}

type Page = 'dashboard' | 'providers' | 'mcp' | 'skills' | 'oauth' | 'usage' | 'settings';
type NavItem = { id: Page; label: string; description: string; icon: LucideIcon };

const pages = new Set<Page>(['dashboard', 'providers', 'mcp', 'skills', 'oauth', 'usage', 'settings']);

function pageFromHash(): Page {
  const value = window.location.hash.replace(/^#\/?/, '') as Page;
  return pages.has(value) ? value : 'dashboard';
}

export default function App() {
  const [authenticated, setAuthenticated] = useState(!!localStorage.getItem('ccswitch_token'));
  const [currentPage, setCurrentPage] = useState<Page>(pageFromHash);

  useEffect(() => {
    const handleNavigation = () => {
      const nextPage = pageFromHash();
      if (nextPage === currentPage) return;
      if (confirmDiscardChanges()) setCurrentPage(nextPage);
      else window.history.replaceState(null, '', `#${currentPage}`);
    };
    window.addEventListener('hashchange', handleNavigation);
    window.addEventListener('popstate', handleNavigation);
    return () => {
      window.removeEventListener('hashchange', handleNavigation);
      window.removeEventListener('popstate', handleNavigation);
    };
  }, [currentPage]);

  const navigate = (page: Page) => {
    if (page === currentPage || !confirmDiscardChanges()) return;
    window.history.pushState(null, '', `#${page}`);
    setCurrentPage(page);
  };

  const clearSessionCache = () => {
    void queryClient.cancelQueries();
    queryClient.clear();
    cacheClear();
  };
  const handleLogin = () => { clearSessionCache(); setAuthenticated(true); };
  const handleLogout = () => {
    if (!confirmDiscardChanges()) return;
    clearSessionCache();
    clearAuthToken();
    setAuthenticated(false);
  };

  if (!authenticated) {
    return <LoginPage onLogin={handleLogin} />;
  }

  const navItems: NavItem[] = [
    { id: 'dashboard', label: 'Dashboard', description: 'Live status', icon: LayoutDashboard },
    { id: 'providers', label: 'Providers', description: 'Model routes', icon: Users },
    { id: 'mcp', label: 'MCP', description: 'Server configs', icon: Server },
    { id: 'skills', label: 'Skills', description: 'CLI extensions', icon: Package },
    { id: 'oauth', label: 'OAuth', description: 'Accounts', icon: Key },
    { id: 'usage', label: 'Usage', description: 'Request stats', icon: BarChart3 },
    { id: 'settings', label: 'Settings', description: 'Preferences', icon: Settings },
  ];

  return (
    <div className="min-h-screen bg-background text-foreground selection:bg-primary/30 selection:text-primary-foreground relative">
      <div className="mx-auto flex min-h-screen w-full max-w-[1440px] flex-col lg:grid lg:grid-cols-[236px_minmax(0,1fr)]">
        <aside className="border-b border-white/[0.06] bg-card/40 px-4 py-4 lg:sticky lg:top-0 lg:h-screen lg:border-b-0 lg:border-r lg:px-5 lg:py-8 flex flex-col">
          <div className="flex items-center justify-between gap-3 lg:block">
            <div className="grid min-w-0 grid-cols-[40px_minmax(0,1fr)] items-center gap-3">
              <BrandMark />
              <div className="min-w-0">
                <div className="truncate text-[15px] font-semibold tracking-[-0.02em] text-foreground">CC Switch</div>
                <div className="truncate text-xs text-muted-foreground">Provider Desk</div>
              </div>
            </div>

            <Button aria-label="Logout" variant="ghost" size="icon" onClick={handleLogout} className="h-9 w-9 lg:hidden hover:bg-white/10 text-muted-foreground hover:text-foreground">
              <LogOut className="h-4 w-4" />
            </Button>
          </div>

          <nav className="mt-4 flex gap-1.5 overflow-x-auto pb-1 lg:mt-8 lg:flex-col lg:overflow-visible lg:pb-0 scrollbar-none">
            {navItems.map((item) => (
              <button
                key={item.id}
                onClick={() => navigate(item.id)}
                onMouseEnter={() => preloadPage(item.id)}
                onFocus={() => preloadPage(item.id)}
                aria-current={currentPage === item.id ? 'page' : undefined}
                className={cn(
                  "group relative grid min-w-max grid-cols-[16px_minmax(0,1fr)] items-center gap-3 rounded-lg px-3 py-3 text-left transition-colors duration-150 lg:min-w-0 outline-none focus-visible:ring-2 focus-visible:ring-primary/50",
                  currentPage === item.id
                    ? "bg-primary/10 text-primary"
                    : "text-muted-foreground hover:bg-white/[0.04] hover:text-foreground"
                )}
              >
                <item.icon className={cn("h-4 w-4 transition-colors", currentPage === item.id ? "text-primary" : "text-muted-foreground group-hover:text-foreground")} />
                <span className="min-w-0">
                  <span className={cn("block truncate text-sm leading-5 transition-colors", currentPage === item.id ? "font-semibold" : "font-medium")}>{item.label}</span>
                  <span className="hidden truncate text-xs leading-5 text-muted-foreground lg:block">{item.description}</span>
                </span>
              </button>
            ))}
          </nav>

          <div className="mt-auto hidden lg:block">
            <div className="border-t border-white/[0.06] px-3 pt-5 pb-2">
              <div className="text-xs font-medium text-muted-foreground">Active Workspace</div>
              <ServerStatus />
            </div>
          </div>

          <Button variant="ghost" size="sm" onClick={handleLogout} className="mt-4 hidden w-full justify-start text-muted-foreground hover:text-foreground hover:bg-white/5 lg:flex rounded-xl transition-colors">
            <LogOut className="h-4 w-4 mr-2" />
            Logout
          </Button>
        </aside>

        <main className="min-w-0 px-4 py-6 sm:px-6 lg:px-10 lg:py-10 xl:px-12 relative">
          <div key={currentPage} className="mx-auto w-full max-w-[1200px] page-enter">
            <ErrorBoundary page>
              <Suspense fallback={<div role="status" className="min-h-64 rounded-2xl border border-white/10 bg-card/40 p-6 text-sm text-muted-foreground">Loading page…</div>}>
                {currentPage === 'dashboard' && <DashboardPage />}
                {currentPage === 'providers' && <ProvidersPage />}
                {currentPage === 'mcp' && <McpPage />}
                {currentPage === 'skills' && <SkillsPage />}
                {currentPage === 'oauth' && <OAuthPage />}
                {currentPage === 'usage' && <UsagePage />}
                {currentPage === 'settings' && <SettingsPage />}
              </Suspense>
            </ErrorBoundary>
          </div>
        </main>
      </div>
    </div>
  );
}

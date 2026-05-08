import { useState } from 'react';
import { LayoutDashboard, Settings, Users, LogOut, Shield, Sparkles, Key, BarChart3, type LucideIcon } from 'lucide-react';
import LoginPage from './pages/LoginPage';
import DashboardPage from './pages/DashboardPage';
import ProvidersPage from './pages/ProvidersPage';
import SettingsPage from './pages/SettingsPage';
import OAuthPage from './pages/OAuthPage';
import UsagePage from './pages/UsagePage';
import { clearAuthToken } from './api';
import { Button } from '@/components/ui/button';
import { cn } from '@/lib/utils';

type Page = 'dashboard' | 'providers' | 'oauth' | 'settings' | 'usage';
type NavItem = { id: Page; label: string; description: string; icon: LucideIcon };

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

  const navItems: NavItem[] = [
    { id: 'dashboard', label: 'Dashboard', description: 'Live status', icon: LayoutDashboard },
    { id: 'providers', label: 'Providers', description: 'Model routes', icon: Users },
    { id: 'oauth', label: 'OAuth', description: 'Accounts', icon: Key },
    { id: 'usage', label: 'Usage', description: 'Request stats', icon: BarChart3 },
    { id: 'settings', label: 'Settings', description: 'Preferences', icon: Settings },
  ];

  return (
    <div className="min-h-screen bg-background text-foreground selection:bg-primary/30 selection:text-primary-foreground relative">
      <div className="fixed inset-0 z-[-1] bg-[radial-gradient(ellipse_at_top_right,_var(--tw-gradient-stops))] from-primary/10 via-background to-background pointer-events-none" />
      <div className="mx-auto flex min-h-screen w-full max-w-[1440px] flex-col lg:grid lg:grid-cols-[272px_minmax(0,1fr)]">
        <aside className="border-b border-white/5 bg-card/40 px-4 py-4 shadow-2xl shadow-black/20 backdrop-blur-3xl lg:sticky lg:top-0 lg:h-screen lg:border-b-0 lg:border-r lg:px-5 lg:py-6 flex flex-col">
          <div className="flex items-center justify-between gap-3 lg:block">
            <div className="grid min-w-0 grid-cols-[40px_minmax(0,1fr)] items-center gap-3">
              <div className="flex h-10 w-10 items-center justify-center rounded-2xl border border-white/10 bg-gradient-to-br from-white/10 to-white/0 shadow-inner shadow-white/5">
                <Shield className="h-5 w-5 text-primary" />
              </div>
              <div className="min-w-0">
                <div className="truncate text-sm font-bold tracking-tight text-foreground">CC Switch</div>
                <div className="truncate text-xs font-medium text-muted-foreground">Provider Desk</div>
              </div>
            </div>
            <div className="hidden items-center gap-1.5 rounded-full border border-primary/20 bg-primary/10 px-2.5 py-1 text-xs font-semibold text-primary lg:mt-6 lg:inline-flex shadow-[0_0_15px_rgba(var(--primary),0.15)]">
              <Sparkles className="h-3.5 w-3.5" />
              Web Admin
            </div>
            <Button variant="ghost" size="icon" onClick={handleLogout} className="h-9 w-9 lg:hidden hover:bg-white/10 text-muted-foreground hover:text-foreground">
              <LogOut className="h-4 w-4" />
            </Button>
          </div>

          <nav className="mt-4 flex gap-1.5 overflow-x-auto pb-1 lg:mt-8 lg:flex-col lg:overflow-visible lg:pb-0 scrollbar-none">
            {navItems.map((item) => (
              <button
                key={item.id}
                onClick={() => setCurrentPage(item.id)}
                className={cn(
                  "group relative grid min-w-max grid-cols-[16px_minmax(0,1fr)] items-center gap-3.5 rounded-xl px-3 py-2.5 text-left transition-all duration-200 lg:min-w-0 outline-none focus-visible:ring-2 focus-visible:ring-primary/50",
                  currentPage === item.id
                    ? "bg-primary/15 text-foreground shadow-sm ring-1 ring-primary/25"
                    : "text-muted-foreground hover:bg-white/[0.08] hover:text-foreground"
                )}
              >
                {currentPage === item.id && (
                  <div className="absolute inset-y-1.5 left-0 w-1 rounded-r-full bg-primary lg:block hidden" />
                )}
                <item.icon className={cn("h-4 w-4 transition-colors", currentPage === item.id ? "text-primary" : "text-muted-foreground group-hover:text-foreground")} />
                <span className="min-w-0">
                  <span className={cn("block truncate text-sm leading-5 transition-all", currentPage === item.id ? "font-semibold" : "font-medium")}>{item.label}</span>
                  <span className="hidden truncate text-[11px] leading-4 text-muted-foreground/80 lg:block">{item.description}</span>
                </span>
              </button>
            ))}
          </nav>

          <div className="mt-auto hidden lg:block">
            <div className="rounded-2xl border border-white/5 bg-black/20 p-4 backdrop-blur-md relative overflow-hidden group">
              <div className="absolute inset-0 bg-gradient-to-br from-primary/5 via-transparent to-transparent opacity-0 transition-opacity duration-500 group-hover:opacity-100" />
              <div className="text-[10px] font-bold uppercase tracking-wider text-muted-foreground">Active Workspace</div>
              <div className="mt-1.5 flex items-center gap-2">
                <div className="h-2 w-2 rounded-full bg-emerald-500 animate-pulse shadow-[0_0_8px_rgba(16,185,129,0.6)]" />
                <span className="truncate text-sm font-medium text-foreground">Local Server</span>
              </div>
            </div>
          </div>

          <Button variant="ghost" size="sm" onClick={handleLogout} className="mt-4 hidden w-full justify-start text-muted-foreground hover:text-foreground hover:bg-white/5 lg:flex rounded-xl transition-colors">
            <LogOut className="h-4 w-4 mr-2" />
            Logout
          </Button>
        </aside>

        <main className="min-w-0 px-4 py-6 sm:px-6 lg:px-8 lg:py-8 relative">
          <div className="mx-auto w-full max-w-[1200px] animate-in fade-in slide-in-from-bottom-4 duration-500">
            {currentPage === 'dashboard' && <DashboardPage />}
            {currentPage === 'providers' && <ProvidersPage />}
            {currentPage === 'oauth' && <OAuthPage />}
            {currentPage === 'usage' && <UsagePage />}
            {currentPage === 'settings' && <SettingsPage />}
          </div>
        </main>
      </div>
    </div>
  );
}

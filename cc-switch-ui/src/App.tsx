import { useState } from 'react';
import { LayoutDashboard, Settings, Users, LogOut, Shield, Sparkles, type LucideIcon } from 'lucide-react';
import LoginPage from './pages/LoginPage';
import DashboardPage from './pages/DashboardPage';
import ProvidersPage from './pages/ProvidersPage';
import SettingsPage from './pages/SettingsPage';
import { clearAuthToken } from './api';
import { Button } from '@/components/ui/button';
import { cn } from '@/lib/utils';

type Page = 'dashboard' | 'providers' | 'settings';
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
    { id: 'settings', label: 'Settings', description: 'Preferences', icon: Settings },
  ];

  return (
    <div className="min-h-screen bg-background text-foreground">
      <div className="mx-auto flex min-h-screen w-full max-w-[1440px] flex-col lg:grid lg:grid-cols-[272px_minmax(0,1fr)]">
        <aside className="border-b border-white/10 bg-card/55 px-4 py-4 shadow-2xl shadow-black/20 backdrop-blur-2xl lg:sticky lg:top-0 lg:h-screen lg:border-b-0 lg:border-r lg:px-5 lg:py-6">
          <div className="flex items-center justify-between gap-3 lg:block">
            <div className="grid min-w-0 grid-cols-[40px_minmax(0,1fr)] items-center gap-3">
              <div className="flex h-10 w-10 items-center justify-center rounded-2xl border border-white/10 bg-white/[0.06] shadow-inner shadow-white/5">
                <Shield className="h-5 w-5 text-primary" />
              </div>
              <div className="min-w-0">
                <div className="truncate text-sm font-semibold leading-5 text-foreground">CC Switch</div>
                <div className="truncate text-xs leading-4 text-muted-foreground">Claude provider desk</div>
              </div>
            </div>
            <div className="hidden items-center gap-1 rounded-full border border-primary/20 bg-primary/10 px-2.5 py-1 text-xs font-medium text-primary lg:mt-5 lg:inline-flex">
              <Sparkles className="h-3.5 w-3.5" />
              macOS mode
            </div>
            <Button variant="ghost" size="icon" onClick={handleLogout} className="h-9 w-9 lg:hidden">
              <LogOut className="h-4 w-4" />
            </Button>
          </div>

          <nav className="mt-4 flex gap-2 overflow-x-auto pb-1 lg:mt-8 lg:flex-col lg:overflow-visible lg:pb-0">
            {navItems.map((item) => (
              <button
                key={item.id}
                onClick={() => setCurrentPage(item.id)}
                className={cn(
                  "group grid min-w-max grid-cols-[16px_minmax(0,1fr)] items-center gap-3 rounded-2xl px-3 py-2.5 text-left transition lg:min-w-0",
                  currentPage === item.id
                    ? "bg-primary/15 text-foreground shadow-inner shadow-primary/10 ring-1 ring-primary/25"
                    : "text-muted-foreground hover:bg-white/[0.06] hover:text-foreground"
                )}
              >
                <item.icon className={cn("h-4 w-4", currentPage === item.id ? "text-primary" : "text-muted-foreground group-hover:text-foreground")} />
                <span className="min-w-0">
                  <span className="block truncate text-sm font-medium leading-5">{item.label}</span>
                  <span className="hidden truncate text-xs leading-4 text-muted-foreground lg:block">{item.description}</span>
                </span>
              </button>
            ))}
          </nav>

          <div className="mt-5 hidden lg:block">
            <div className="rounded-3xl border border-white/10 bg-white/[0.035] p-4">
              <div className="text-xs font-medium uppercase text-muted-foreground">Active workspace</div>
              <div className="mt-1 truncate text-sm text-foreground">Local Web Admin</div>
            </div>
          </div>

          <Button variant="ghost" size="sm" onClick={handleLogout} className="mt-4 hidden w-full justify-start lg:flex">
            <LogOut className="h-4 w-4" />
            Logout
          </Button>
        </aside>

        <main className="min-w-0 px-4 py-5 sm:px-6 lg:px-8 lg:py-8">
          <div className="mx-auto w-full max-w-6xl">
            {currentPage === 'dashboard' && <DashboardPage />}
            {currentPage === 'providers' && <ProvidersPage />}
            {currentPage === 'settings' && <SettingsPage />}
          </div>
        </main>
      </div>
    </div>
  );
}

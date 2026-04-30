import { useState } from 'react';
import { LockKeyhole, Shield } from 'lucide-react';
import { login as apiLogin, setAuthToken } from '../api';
import { Button } from '@/components/ui/button';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';

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
    <div className="flex min-h-screen items-center justify-center px-4 py-10 relative overflow-hidden bg-background text-foreground selection:bg-primary/30 selection:text-primary-foreground">
      <div className="fixed inset-0 z-[-1] bg-[radial-gradient(ellipse_at_center,_var(--tw-gradient-stops))] from-primary/10 via-background to-background pointer-events-none" />
      <div className="absolute top-1/2 left-1/2 -translate-x-1/2 -translate-y-1/2 w-[800px] h-[800px] bg-primary/5 rounded-full blur-3xl pointer-events-none" />
      
      <Card className="w-full max-w-[420px] overflow-hidden border-white/10 bg-card/60 backdrop-blur-2xl shadow-2xl relative z-10">
        <div className="absolute inset-0 bg-gradient-to-br from-primary/5 via-transparent to-transparent pointer-events-none" />
        
        <CardHeader className="items-center border-b border-white/5 bg-black/10 py-8 text-center">
          <div className="mb-4 flex h-14 w-14 items-center justify-center rounded-2xl border border-primary/20 bg-gradient-to-br from-primary/20 to-primary/5 shadow-[0_0_20px_rgba(var(--primary),0.15)]">
            <Shield className="h-7 w-7 text-primary drop-shadow-sm" />
          </div>
          <CardTitle className="text-3xl font-bold tracking-tight">CC Switch</CardTitle>
          <p className="mt-1.5 text-sm font-medium text-muted-foreground/80 uppercase tracking-wider">Web Admin Interface</p>
        </CardHeader>
        
        <CardContent className="p-8">
          <form onSubmit={handleLogin} className="space-y-5">
            <div className="space-y-2.5">
              <Label htmlFor="admin-token" className="text-xs font-bold uppercase tracking-wider text-muted-foreground/80">Admin token</Label>
              <div className="relative group">
                <div className="absolute inset-0 rounded-xl bg-gradient-to-r from-primary/50 to-primary/30 blur opacity-0 group-focus-within:opacity-30 transition-opacity duration-300" />
                <LockKeyhole className="pointer-events-none absolute left-3.5 top-1/2 h-4 w-4 -translate-y-1/2 text-muted-foreground transition-colors group-focus-within:text-primary" />
                <Input
                  id="admin-token"
                  type="password"
                  value={token}
                  onChange={(e) => setToken(e.target.value)}
                  placeholder="Enter admin token"
                  className="pl-10 h-12 rounded-xl border-white/10 bg-black/20 font-mono shadow-inner transition-all focus:border-primary/50 focus:ring-2 focus:ring-primary/20 relative z-10"
                  autoFocus
                />
              </div>
            </div>
            
            {error && (
              <div className="rounded-xl border border-destructive/30 bg-destructive/10 px-4 py-3 text-sm font-medium text-destructive shadow-sm animate-in fade-in slide-in-from-top-1">
                {error}
              </div>
            )}
            
            <Button type="submit" className="w-full h-12 rounded-xl text-base font-semibold shadow-[0_0_15px_rgba(var(--primary),0.2)] hover:shadow-[0_0_20px_rgba(var(--primary),0.3)] transition-all" disabled={loading}>
              {loading ? (
                <div className="flex items-center gap-2">
                  <div className="h-4 w-4 rounded-full border-2 border-white/80 border-t-transparent animate-spin" />
                  Authenticating...
                </div>
              ) : 'Login to Dashboard'}
            </Button>
          </form>
          
          <div className="mt-8 pt-6 border-t border-white/5">
            <p className="text-center text-[11px] leading-relaxed font-medium text-muted-foreground/60">
              Token is printed in the server console on first run<br />or can be set via <code className="font-mono bg-white/5 px-1 py-0.5 rounded text-muted-foreground/80">CC_SWITCH_ADMIN_TOKEN</code> env.
            </p>
          </div>
        </CardContent>
      </Card>
    </div>
  );
}

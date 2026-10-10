import { BrandMark } from '@/components/BrandMark';
import { useState } from 'react';
import { LockKeyhole } from 'lucide-react';
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

      
      <Card className="relative z-10 w-full max-w-[420px]">
        
        <CardHeader className="items-center  py-8 text-center">
          <BrandMark className="mb-2 h-14 w-14" />
          <CardTitle className="text-3xl font-semibold tracking-[-0.04em]">CC Switch</CardTitle>
          <p className="mt-1.5 text-sm text-muted-foreground">Web Admin Interface</p>
        </CardHeader>
        
        <CardContent className="p-8">
          <form onSubmit={handleLogin} className="space-y-5">
            <div className="space-y-2.5">
              <Label htmlFor="admin-token" className="text-sm font-medium text-muted-foreground">Admin token</Label>
              <div className="relative group">
                <LockKeyhole className="pointer-events-none absolute left-3.5 top-1/2 h-4 w-4 -translate-y-1/2 text-muted-foreground transition-colors group-focus-within:text-primary" />
                <Input
                  id="admin-token"
                  type="password"
                  value={token}
                  onChange={(e) => setToken(e.target.value)}
                  placeholder="Enter admin token"
                  className="relative z-10 h-11 pl-10 font-mono"
                  autoFocus
                />
              </div>
            </div>
            
            {error && (
              <div className="rounded-xl border border-destructive/30 bg-destructive/10 px-4 py-3 text-sm font-medium text-destructive shadow-sm animate-in fade-in slide-in-from-top-1">
                {error}
              </div>
            )}
            
            <Button type="submit" className="w-full h-12 rounded-xl text-base font-medium" disabled={loading}>
              {loading ? (
                <div className="flex items-center gap-2">
                  <div className="h-4 w-4 rounded-full border-2 border-white/80 border-t-transparent animate-spin" />
                  Authenticating...
                </div>
              ) : 'Login to Dashboard'}
            </Button>
          </form>
          
          <div className="mt-8 pt-6 border-t border-white/5">
            <p className="text-center text-xs leading-relaxed text-muted-foreground">
              Token is printed in the server console on first run<br />or can be set via <code className="font-mono bg-white/5 px-1 py-0.5 rounded text-muted-foreground/80">CC_SWITCH_ADMIN_TOKEN</code> env.
            </p>
          </div>
        </CardContent>
      </Card>
    </div>
  );
}

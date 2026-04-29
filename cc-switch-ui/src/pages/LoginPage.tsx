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
    <div className="flex min-h-screen items-center justify-center px-4 py-10">
      <Card className="w-full max-w-md overflow-hidden">
        <CardHeader className="items-center border-b border-white/10 bg-white/[0.025] py-6 text-center">
          <div className="mb-3 flex h-12 w-12 items-center justify-center rounded-2xl border border-white/10 bg-primary/15">
            <Shield className="h-6 w-6 text-primary" />
          </div>
          <CardTitle className="text-2xl leading-8">CC Switch</CardTitle>
          <p className="text-sm leading-5 text-muted-foreground">Web Admin Interface</p>
        </CardHeader>
        <CardContent className="p-6">
          <form onSubmit={handleLogin} className="space-y-4">
            <div className="space-y-2">
              <Label htmlFor="admin-token">Admin token</Label>
              <div className="relative">
                <LockKeyhole className="pointer-events-none absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-muted-foreground" />
                <Input
                  id="admin-token"
                  type="password"
                  value={token}
                  onChange={(e) => setToken(e.target.value)}
                  placeholder="Enter admin token"
                  className="pl-9"
                  autoFocus
                />
              </div>
            </div>
            {error && (
              <div className="rounded-xl border border-destructive/30 bg-destructive/10 px-3 py-2 text-sm text-destructive">
                {error}
              </div>
            )}
            <Button type="submit" className="w-full" disabled={loading}>
              {loading ? 'Logging in...' : 'Login'}
            </Button>
          </form>
          <p className="mt-5 text-center text-xs leading-5 text-muted-foreground">
            Token is printed in console on first run or set via CC_SWITCH_ADMIN_TOKEN env.
          </p>
        </CardContent>
      </Card>
    </div>
  );
}

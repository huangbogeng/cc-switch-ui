import { Component, type ErrorInfo, type ReactNode } from 'react';
import { Button } from '@/components/ui/button';

interface Props {
  children: ReactNode;
}

interface State {
  error: Error | null;
}

export class ErrorBoundary extends Component<Props, State> {
  state: State = { error: null };

  static getDerivedStateFromError(error: Error): State {
    return { error };
  }

  componentDidCatch(error: Error, info: ErrorInfo) {
    console.error('Unhandled UI error', error, info.componentStack);
  }

  render() {
    if (!this.state.error) return this.props.children;

    return (
      <main className="flex min-h-screen items-center justify-center bg-background p-6 text-foreground">
        <div role="alert" className="w-full max-w-lg rounded-2xl border border-destructive/30 bg-card p-6 shadow-xl">
          <h1 className="text-lg font-semibold">The interface encountered an unexpected error</h1>
          <p className="mt-2 break-words text-sm text-muted-foreground">{this.state.error.message}</p>
          <Button className="mt-5" onClick={() => window.location.reload()}>Reload interface</Button>
        </div>
      </main>
    );
  }
}

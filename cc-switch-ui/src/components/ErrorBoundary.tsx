import { Component, type ErrorInfo, type ReactNode } from 'react';
import { Button } from '@/components/ui/button';

interface Props {
  children: ReactNode;
  page?: boolean;
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

    const moduleFailed = /Failed to fetch dynamically imported module|error loading dynamically imported module|Importing a module script failed|Unable to preload CSS/i.test(this.state.error.message);
    const Container = this.props.page ? 'section' : 'main';

    return (
      <Container className={`flex ${this.props.page ? 'min-h-64' : 'min-h-screen'} items-center justify-center bg-background p-6 text-foreground`}>
        <div role="alert" className="w-full max-w-lg rounded-2xl border border-destructive/30 bg-card p-6 shadow-sm">
          <h1 className="text-lg font-semibold">{moduleFailed ? 'The page could not be loaded' : 'The interface encountered an unexpected error'}</h1>
          <p className="mt-2 break-words text-sm text-muted-foreground">{moduleFailed
            ? 'A page file is unavailable after an update or a network interruption. Reload the interface to load the installed version. If this continues, check that the service is reachable.'
            : this.state.error.message}</p>
          <Button className="mt-5" onClick={() => window.location.reload()}>Reload interface</Button>
        </div>
      </Container>
    );
  }
}

import React from 'react';
import ReactDOM from 'react-dom/client';
import { App } from './app/App';
import './styles.css';

class AppErrorBoundary extends React.Component<React.PropsWithChildren, { error: Error | null }> {
  state = { error: null as Error | null };

  static getDerivedStateFromError(error: Error) {
    return { error };
  }

  render() {
    if (this.state.error) {
      return <main className="fatal-error"><div className="fatal-error-card"><span className="eyebrow">SYNTHESIZE / RECOVERY</span><h1>The workspace failed to render.</h1><p>{this.state.error.message}</p><button className="primary" onClick={() => window.location.reload()}>Reload workspace</button></div></main>;
    }
    return this.props.children;
  }
}

ReactDOM.createRoot(document.getElementById('root') as HTMLElement).render(
  <React.StrictMode>
    <AppErrorBoundary><App /></AppErrorBoundary>
  </React.StrictMode>
);

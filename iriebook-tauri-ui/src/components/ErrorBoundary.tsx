import { Component, ErrorInfo, ReactNode } from "react";
import { toast } from "sonner";
import i18n from "../i18n/config";

interface Props {
  children: ReactNode;
}

interface State {
  hasError: boolean;
  error: Error | null;
}

class ErrorBoundary extends Component<Props, State> {
  constructor(props: Props) {
    super(props);
    this.state = { hasError: false, error: null };
  }

  static getDerivedStateFromError(error: Error): State {
    return { hasError: true, error };
  }

  componentDidCatch(error: Error, errorInfo: ErrorInfo) {
    console.error("ErrorBoundary caught an error:", error, errorInfo);

    // Show toast notification
    toast.error(i18n.t('toasts.error.generic'), {
      description: error.message,
    });
  }

  render() {
    if (this.state.hasError) {
      return (
        <div className="flex items-center justify-center min-h-screen bg-background">
          <div className="max-w-md p-6 bg-card border border-border rounded-lg shadow-lg">
            <h2 className="text-xl font-bold text-destructive mb-4">
              {i18n.t('errors.boundary.title')}
            </h2>
            <p className="text-muted-foreground mb-4">
              {this.state.error?.message || i18n.t('errors.boundary.message')}
            </p>
            <button
              onClick={() => window.location.reload()}
              className="px-4 py-2 bg-primary text-primary-foreground rounded hover:bg-primary/90"
            >
              {i18n.t('errors.boundary.reload')}
            </button>
          </div>
        </div>
      );
    }

    return this.props.children;
  }
}

export default ErrorBoundary;

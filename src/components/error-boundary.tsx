/**
 * Error Boundary — catches React render crashes and shows the error
 * instead of a blank screen.
 *
 * Without this, any runtime error in a component produces a blank
 * navy screen with no indication of what went wrong.
 */

import { Component, type ReactNode } from "react";

interface Props {
  children: ReactNode;
}

interface State {
  hasError: boolean;
  error: Error | null;
}

export class ErrorBoundary extends Component<Props, State> {
  constructor(props: Props) {
    super(props);
    this.state = { hasError: false, error: null };
  }

  static getDerivedStateFromError(error: Error): State {
    return { hasError: true, error };
  }

  componentDidCatch(error: Error, errorInfo: React.ErrorInfo) {
    console.error("MetaRDU Error Boundary caught:", error, errorInfo);
  }

  render() {
    if (this.state.hasError && this.state.error) {
      return (
        <div
          style={{
            padding: "40px",
            fontFamily: "JetBrains Mono, monospace",
            fontSize: "13px",
            color: "#FF6B6B",
            background: "#0A192F",
            minHeight: "100vh",
            whiteSpace: "pre-wrap",
            overflow: "auto",
          }}
        >
          <div style={{ fontSize: "18px", fontWeight: "bold", marginBottom: "20px", color: "#FFA500" }}>
            ⚠ MetaRDU Industrial — Render Error
          </div>
          <div style={{ marginBottom: "16px" }}>
            <strong>Error:</strong> {this.state.error.message}
          </div>
          <div style={{ marginBottom: "16px", color: "#94A3B8" }}>
            <strong>Stack:</strong>
            {"\n"}
            {this.state.error.stack}
          </div>
          <button
            onClick={() => {
              this.setState({ hasError: false, error: null });
              window.location.reload();
            }}
            style={{
              padding: "8px 16px",
              background: "#FFA500",
              color: "#0A192F",
              border: "none",
              borderRadius: "4px",
              cursor: "pointer",
              fontWeight: "bold",
            }}
          >
            Reload App
          </button>
        </div>
      );
    }
    return this.props.children;
  }
}

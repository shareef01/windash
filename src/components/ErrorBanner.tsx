interface Props {
  message: string;
  detail?: string;
  onDismiss: () => void;
  onRetry?: () => void;
}

export function ErrorBanner({ message, detail, onDismiss, onRetry }: Props) {
  return (
    <div className="banner" role="alert">
      <div className="banner-body">
        <span className="banner-msg">{message}</span>
        {detail && (
          <details className="banner-detail">
            <summary>Details</summary>
            <pre>{detail}</pre>
          </details>
        )}
      </div>
      <div className="banner-actions">
        {onRetry && (
          <button type="button" className="banner-btn" onClick={onRetry}>
            Retry
          </button>
        )}
        <button type="button" className="banner-btn" onClick={onDismiss} aria-label="Dismiss error">
          Dismiss
        </button>
      </div>
    </div>
  );
}

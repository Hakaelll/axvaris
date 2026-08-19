import {
  Component,
  useCallback,
  useEffect,
  useMemo,
  useRef,
  useState,
  type ReactNode,
} from "react";
import { Icon } from "../components/Icon";
import { translate, useI18n, useLanguage } from "../i18n";
import { desktopApi } from "#backend";
import {
  compareListenerSnapshots,
  formatListenerEndpoint,
  listenerKey,
  normalizeListenerSnapshots,
} from "../lib/exposure";
import type {
  DashboardSnapshot,
  ListenerExposure,
  ListenerSnapshot,
} from "../types";

interface ExposureViewProps {
  snapshot: DashboardSnapshot;
  onExport: () => void;
  onOpenEvidence: (connectionId: string | null, query: string) => void;
}

function shortHash(hash: string): string {
  return hash.length > 18 ? `${hash.slice(0, 12)}…${hash.slice(-6)}` : hash;
}

interface BoundaryProps {
  children: ReactNode;
  fallback: (reset: () => void) => ReactNode;
}

interface BoundaryState {
  failed: boolean;
}

class ExposureBoundaryCore extends Component<BoundaryProps, BoundaryState> {
  override state: BoundaryState = { failed: false };

  static getDerivedStateFromError(): BoundaryState {
    return { failed: true };
  }

  private readonly reset = () => this.setState({ failed: false });

  override render() {
    return this.state.failed
      ? this.props.fallback(this.reset)
      : this.props.children;
  }
}

export function ExposureViewBoundary({ children }: { children: ReactNode }) {
  const t = useI18n();
  return (
    <ExposureBoundaryCore
      fallback={(reset) => (
        <section
          aria-labelledby="exposure-recovery-title"
          className="exposure-view exposure-view--recovery"
        >
          <div className="today-empty" role="alert">
            <Icon name="shield" />
            <strong id="exposure-recovery-title">
              {t("Exposure could not be displayed")}
            </strong>
            <p>
              {t(
                "The view isolated an unexpected rendering error. Network capture and retained evidence remain available.",
              )}
            </p>
            <button
              className="button button--primary"
              onClick={reset}
              type="button"
            >
              {t("Reload Exposure")}
            </button>
          </div>
        </section>
      )}
    >
      {children}
    </ExposureBoundaryCore>
  );
}

export function ExposureView({
  snapshot,
  onExport,
  onOpenEvidence,
}: ExposureViewProps) {
  const t = useI18n();
  const language = useLanguage();
  const [snapshots, setSnapshots] = useState<ListenerSnapshot[]>([]);
  const [capturing, setCapturing] = useState(false);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [warning, setWarning] = useState<string | null>(null);
  const refreshSequence = useRef(0);

  const refresh = useCallback(async () => {
    const sequence = refreshSequence.current + 1;
    refreshSequence.current = sequence;
    setLoading(true);
    try {
      const result = normalizeListenerSnapshots(
        await desktopApi.listListenerSnapshots(100),
      );
      if (refreshSequence.current !== sequence) return;
      setSnapshots(result.snapshots);
      setWarning(
        result.rejectedListeners + result.rejectedSnapshots > 0
          ? translate(
              language,
              "Some malformed listener records were ignored to keep Exposure available.",
            )
          : null,
      );
      setError(null);
    } catch (cause: unknown) {
      if (refreshSequence.current !== sequence) return;
      setError(cause instanceof Error ? cause.message : String(cause));
    } finally {
      if (refreshSequence.current === sequence) setLoading(false);
    }
  }, [language]);

  useEffect(() => {
    void refresh();
    return () => {
      refreshSequence.current += 1;
    };
  }, [refresh]);

  const capture = async () => {
    setCapturing(true);
    try {
      await desktopApi.captureListenerSnapshot();
      await refresh();
    } catch (cause: unknown) {
      setError(cause instanceof Error ? cause.message : String(cause));
    } finally {
      setCapturing(false);
    }
  };

  const latest = snapshots[0] ?? null;
  const previous = snapshots[1] ?? null;
  const comparison = useMemo(
    () => compareListenerSnapshots(latest, previous),
    [latest, previous],
  );

  const liveByInstance = useMemo(
    () =>
      new Map(
        (Array.isArray(snapshot.connections) ? snapshot.connections : []).map(
          (connection) => [connection.key.process_instance, connection],
        ),
      ),
    [snapshot.connections],
  );

  const rows = useMemo(
    () => [
      ...comparison.added.map((listener) => ({
        change: "added" as const,
        listener,
      })),
      ...comparison.current.map((listener) => ({
        change: "current" as const,
        listener,
      })),
      ...comparison.removed.map((listener) => ({
        change: "removed" as const,
        listener,
      })),
    ],
    [comparison],
  );

  const renderListener = (
    listener: ListenerExposure,
    change: "added" | "removed" | "current",
    index: number,
  ) => {
    const connection = liveByInstance.get(listener.process_instance) ?? null;
    return (
      <tr key={`${change}:${listenerKey(listener)}:${index}`}>
        <td>
          <span className={`change-pill change-pill--${change}`}>
            {t(change)}
          </span>
        </td>
        <td>
          <strong>{connection?.process.name ?? t("Unknown process")}</strong>
          <small>
            {listener.process_instance ?? t("No process instance evidence")}
          </small>
        </td>
        <td>
          <code>{formatListenerEndpoint(listener)}</code>
        </td>
        <td>{listener.protocol.toUpperCase()}</td>
        <td>
          <span className={`scope-pill scope-pill--${listener.scope}`}>
            {t(listener.scope)}
          </span>
        </td>
        <td>
          {connection?.process.executable_revision_id ??
            t("Revision unavailable")}
        </td>
        <td>{connection?.process.user_id ?? t("User unavailable")}</td>
        <td>{connection?.interface ?? t("Interface unavailable")}</td>
        <td>
          <strong>{t(listener.firewall)}</strong>
          <small>{listener.limitation}</small>
        </td>
        <td>
          <button
            className="button button--secondary"
            disabled={!connection}
            onClick={() =>
              onOpenEvidence(connection?.id ?? null, "state:listening")
            }
            type="button"
          >
            {t("Evidence")}
          </button>
        </td>
      </tr>
    );
  };

  const displayed = latest?.listeners ?? [];
  return (
    <section
      aria-busy={loading}
      aria-labelledby="exposure-title"
      className="exposure-view"
    >
      <header className="view-heading">
        <div>
          <span className="eyebrow">{t("LOCAL LISTENER AUDIT")}</span>
          <h1 id="exposure-title">{t("Exposure")}</h1>
          <p>
            {t(
              "What is listening, since when, and where its bind may be reachable. No network scan is performed.",
            )}
          </p>
        </div>
        <div className="exposure-actions">
          <button
            className="button button--primary"
            disabled={capturing}
            onClick={() => void capture()}
            type="button"
          >
            <Icon name="activity" />
            {capturing
              ? t("Capturing local snapshot…")
              : t("Capture signed snapshot")}
          </button>
          <button
            className="button button--secondary"
            onClick={onExport}
            type="button"
          >
            <Icon name="download" />
            {t("Export redacted evidence")}
          </button>
        </div>
      </header>

      <div className="exposure-summary" role="status">
        <article>
          <strong>{displayed.length}</strong>
          <span>{t("listeners in latest snapshot")}</span>
        </article>
        <article>
          <strong>{comparison.added.length}</strong>
          <span>{t("added since previous snapshot")}</span>
        </article>
        <article>
          <strong>{comparison.removed.length}</strong>
          <span>{t("removed since previous snapshot")}</span>
        </article>
        <article>
          <strong>{comparison.unchanged}</strong>
          <span>{t("unchanged")}</span>
        </article>
      </div>

      <section
        className="snapshot-ledger"
        aria-labelledby="snapshot-ledger-title"
      >
        <header>
          <div>
            <h2 id="snapshot-ledger-title">
              {t("Hash-signed local snapshots")}
            </h2>
            <p>
              {t(
                "SHA-256 covers the deterministic listener set stored in SQLite.",
              )}
            </p>
          </div>
          <span>{t("{count} retained", { count: snapshots.length })}</span>
        </header>
        <div className="snapshot-ledger__list">
          {snapshots.slice(0, 8).map((item, index) => (
            <article key={`${item.id ?? "new"}:${item.snapshot_sha256}`}>
              <span>
                {index === 0
                  ? t("Latest")
                  : new Date(item.observed_at_ms).toLocaleString()}
              </span>
              <code title={item.snapshot_sha256}>
                {shortHash(item.snapshot_sha256)}
              </code>
              <strong>
                {t("{count} listeners", { count: item.listeners.length })}
              </strong>
            </article>
          ))}
          {loading && snapshots.length === 0 && (
            <p className="list-empty">{t("Loading listener snapshots…")}</p>
          )}
          {!loading && snapshots.length === 0 && (
            <p className="list-empty">
              {t(
                "Capture the first local listener snapshot to establish a comparison point.",
              )}
            </p>
          )}
        </div>
      </section>

      <section
        className="exposure-table-section"
        aria-labelledby="exposure-table-title"
      >
        <header>
          <div>
            <h2 id="exposure-table-title">{t("Listener evidence")}</h2>
            <p>
              {t(
                "Wildcard means all local interfaces; it does not prove Internet accessibility.",
              )}
            </p>
          </div>
        </header>
        <div className="table-scroll" tabIndex={0}>
          <table className="evidence-table">
            <caption className="sr-only">
              {t("Current and changed local listener evidence")}
            </caption>
            <thead>
              <tr>
                <th>{t("Change")}</th>
                <th>{t("Process / instance")}</th>
                <th>{t("Bind endpoint")}</th>
                <th>{t("Protocol")}</th>
                <th>{t("Scope")}</th>
                <th>{t("Executable revision")}</th>
                <th>{t("User")}</th>
                <th>{t("Interface")}</th>
                <th>{t("Firewall evidence")}</th>
                <th>{t("Action")}</th>
              </tr>
            </thead>
            <tbody>
              {rows.map(({ listener, change }, index) =>
                renderListener(listener, change, index),
              )}
            </tbody>
          </table>
        </div>
        {displayed.length === 0 &&
          comparison.removed.length === 0 &&
          !loading && (
            <div className="today-empty">
              <Icon name="shield" />
              <strong>
                {t("No listener evidence in the latest snapshot")}
              </strong>
              <p>
                {t(
                  "This is an observation result, not proof that no service can listen between snapshots.",
                )}
              </p>
            </div>
          )}
      </section>

      <aside className="evidence-limitation">
        <Icon name="shield" />
        <div>
          <strong>{t("Reachability limitation")}</strong>
          <p>
            {t(
              "Firewall state is reported as not queryable until a native read-only rule provider supplies evidence. Bind scope alone never becomes a vulnerability verdict.",
            )}
          </p>
        </div>
      </aside>
      {warning && (
        <p className="exposure-warning" role="status">
          {warning}
        </p>
      )}
      {error && (
        <p className="error-copy" role="alert">
          {error}
        </p>
      )}
    </section>
  );
}

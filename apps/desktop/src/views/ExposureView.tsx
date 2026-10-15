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
import { isListenerOnly } from "../lib/filter";
import {
  compareListenerSnapshots,
  listenerKey,
  normalizeListenerSnapshots,
} from "../lib/exposure";
import type {
  DashboardSnapshot,
  ListenerExposure,
  ListenerSnapshot,
} from "../types";
import { usePresentation } from "../privacy/PresentationMode";

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
  const { display, displayText } = usePresentation();
  const [snapshots, setSnapshots] = useState<ListenerSnapshot[]>([]);
  const [capturing, setCapturing] = useState(false);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [warning, setWarning] = useState<string | null>(null);
  const [selectedKey, setSelectedKey] = useState<string | null>(null);
  const [selectedSnapshotHash, setSelectedSnapshotHash] = useState<
    string | null
  >(null);
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
  const viewedSnapshot =
    snapshots.find((item) => item.snapshot_sha256 === selectedSnapshotHash) ??
    latest;
  const comparison = useMemo(
    () => compareListenerSnapshots(latest, previous),
    [latest, previous],
  );

  const liveByListener = useMemo(
    () =>
      new Map(
        (Array.isArray(snapshot.connections) ? snapshot.connections : []).filter(isListenerOnly).map(
          (connection) => [listenerKey({ endpoint: connection.key.local, protocol: connection.protocol, process_instance: connection.key.process_instance }), connection],
        ),
      ),
    [snapshot.connections],
  );

  const rows = useMemo(
    () =>
      viewedSnapshot && viewedSnapshot !== latest
        ? viewedSnapshot.listeners.map((listener) => ({
            change: "current" as const,
            listener,
          }))
        : [
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
    [comparison, latest, viewedSnapshot],
  );

  const renderListener = (
    listener: ListenerExposure,
    change: "added" | "removed" | "current",
    index: number,
  ) => {
    const connection = liveByListener.get(listenerKey(listener)) ?? null;
    return (
      <tr
        className={
          selectedRow?.listener === listener ? "is-selected" : undefined
        }
        key={`${change}:${listenerKey(listener)}:${index}`}
      >
        <td>
          <span className={`change-pill change-pill--${change}`}>
            {t(
              change === "current"
                ? "Current"
                : change === "added"
                  ? "Added listener"
                  : "Removed listener",
            )}
          </span>
        </td>
        <td>
          <strong>
            {connection
              ? display("process", connection.process.name)
              : t("Unknown process")}
          </strong>
          <small>
            {listener.process_instance ?? t("No process instance evidence")}
          </small>
        </td>
        <td>
          <code>
            {display("local_ip", listener.endpoint.address)}:
            {listener.endpoint.port}
          </code>
        </td>
        <td>{listener.protocol.toUpperCase()}</td>
        <td>
          <span className={`scope-pill scope-pill--${listener.scope}`}>
            {t(listener.scope)}
          </span>
        </td>
        <td>
          <button
            className="button button--secondary"
            aria-pressed={selectedRow?.listener === listener}
            onClick={() => setSelectedKey(`${change}:${listenerKey(listener)}`)}
            type="button"
          >
            {t("Evidence")}
          </button>
        </td>
      </tr>
    );
  };

  const displayed = viewedSnapshot?.listeners ?? [];
  const selectSnapshot = (hash: string) => {
    setSelectedSnapshotHash(hash);
    setSelectedKey(null);
  };
  const selectedRow =
    rows.find(
      ({ change, listener }) =>
        `${change}:${listenerKey(listener)}` === selectedKey,
    ) ?? rows[0];
  const selectedConnection = selectedRow
    ? liveByListener.get(listenerKey(selectedRow.listener))
    : undefined;
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
          <strong>{latest?.listeners.length ?? 0}</strong>
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
        {latest && previous && (
          <div className="snapshot-comparison">
            <button
              className="snapshot-node"
              aria-pressed={viewedSnapshot === previous}
              onClick={() => selectSnapshot(previous.snapshot_sha256)}
              type="button"
            >
              <small>{t("Previous snapshot")}</small>
              <time>{new Date(previous.observed_at_ms).toLocaleString()}</time>
              <code title={previous.snapshot_sha256}>
                {shortHash(previous.snapshot_sha256)}
              </code>
              <strong>
                {t("{count} listeners", { count: previous.listeners.length })}
              </strong>
            </button>
            <div className="snapshot-comparison__changes">
              <span>
                {comparison.added.length} {t("added")} <Icon name="chevron" />
              </span>
              <span>
                {comparison.removed.length} {t("removed")}{" "}
                <Icon name="chevron" />
              </span>
              <span>
                {comparison.unchanged} {t("unchanged")} <Icon name="chevron" />
              </span>
            </div>
            <button
              className="snapshot-node is-current"
              aria-pressed={viewedSnapshot === latest}
              onClick={() => selectSnapshot(latest.snapshot_sha256)}
              type="button"
            >
              <small>{t("Current snapshot")}</small>
              <time>{new Date(latest.observed_at_ms).toLocaleString()}</time>
              <code title={latest.snapshot_sha256}>
                {shortHash(latest.snapshot_sha256)}
              </code>
              <strong>
                {t("{count} listeners", { count: latest.listeners.length })}
              </strong>
            </button>
          </div>
        )}
        <div className="snapshot-ledger__list">
          {snapshots.slice(previous ? 2 : 0, 8).map((item) => (
            <button
              className="snapshot-node"
              aria-pressed={viewedSnapshot === item}
              onClick={() => selectSnapshot(item.snapshot_sha256)}
              key={`${item.id ?? "new"}:${item.snapshot_sha256}`}
              type="button"
            >
              <span>
                {item === latest
                  ? t("Latest")
                  : new Date(item.observed_at_ms).toLocaleString()}
              </span>
              <code title={item.snapshot_sha256}>
                {shortHash(item.snapshot_sha256)}
              </code>
              <strong>
                {t("{count} listeners", { count: item.listeners.length })}
              </strong>
            </button>
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

      <div className="exposure-evidence">
        <section
          className="exposure-table-section"
          aria-labelledby="exposure-table-title"
        >
          <header>
            <div>
              <h2 id="exposure-table-title">{t("Listener evidence")}</h2>
              {viewedSnapshot && (
                <p>
                  {new Date(viewedSnapshot.observed_at_ms).toLocaleString()} ·{" "}
                  {shortHash(viewedSnapshot.snapshot_sha256)}
                </p>
              )}
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
        {selectedRow && (
          <aside
            className="listener-inspector"
            aria-label={t("Listener evidence")}
          >
            <header>
              <Icon name="process" />
              <h2>
                {selectedConnection
                  ? display("process", selectedConnection.process.name)
                  : t("Unknown process")}
              </h2>
              <span
                className={`change-pill change-pill--${selectedRow.change}`}
              >
                {t(
                  selectedRow.change === "current"
                    ? "Current"
                    : selectedRow.change === "added"
                      ? "Added listener"
                      : "Removed listener",
                )}
              </span>
            </header>
            <dl>
              <div>
                <dt>{t("Bind endpoint")}</dt>
                <dd>
                  <code>
                    {display("local_ip", selectedRow.listener.endpoint.address)}
                    :{selectedRow.listener.endpoint.port}
                  </code>
                </dd>
              </div>
              <div>
                <dt>{t("Protocol")}</dt>
                <dd>{selectedRow.listener.protocol.toUpperCase()}</dd>
              </div>
              <div>
                <dt>{t("Executable revision")}</dt>
                <dd>
                  {selectedConnection?.process.executable_revision_id ??
                    t("Revision unavailable")}
                </dd>
              </div>
              <div>
                <dt>{t("User")}</dt>
                <dd>
                  {selectedConnection?.process.user_id
                    ? display("user", selectedConnection.process.user_id)
                    : t("User unavailable")}
                </dd>
              </div>
              <div>
                <dt>{t("Interface")}</dt>
                <dd>
                  {selectedConnection?.interface
                    ? display("interface", selectedConnection.interface)
                    : t("Interface unavailable")}
                </dd>
              </div>
              <div>
                <dt>{t("Firewall evidence")}</dt>
                <dd>{t(selectedRow.listener.firewall)}</dd>
              </div>
            </dl>
            <p>{displayText(selectedRow.listener.limitation)}</p>
            <button
              className="button button--secondary"
              disabled={!selectedConnection}
              onClick={() =>
                onOpenEvidence(
                  selectedConnection?.id ?? null,
                  "state:listening",
                )
              }
              type="button"
            >
              {t("Open evidence")}
              <Icon name="chevron" />
            </button>
          </aside>
        )}
      </div>

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

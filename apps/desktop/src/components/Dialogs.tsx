import { useEffect, useState } from "react";
import { useI18n } from "../i18n";
import type { ExportRequest, RecordingRequest, RecordingState } from "../types";
import { formatBytes, formatDuration } from "../lib/format";
import { Icon } from "./Icon";

interface RecordingDialogProps {
  open: boolean;
  state: RecordingState | null;
  onClose: () => void;
  onStart: (request: RecordingRequest) => void;
  onStop: () => void;
}

export function RecordingDialog({
  open,
  state,
  onClose,
  onStart,
  onStop,
}: RecordingDialogProps) {
  const t = useI18n();
  const [path, setPath] = useState("sabicheiro-capture.pcapng");
  const [size, setSize] = useState(256);
  const [duration, setDuration] = useState(15);
  const [rotate, setRotate] = useState(true);
  const [nowMs, setNowMs] = useState(Date.now());
  const recoverableError =
    state?.status === "error" && Boolean(state.output_path);
  const active =
    state?.status === "recording" ||
    state?.status === "starting" ||
    state?.status === "stopping" ||
    recoverableError;
  useEffect(() => {
    if (state?.output_path) setPath(state.output_path);
  }, [state?.output_path]);
  useEffect(() => {
    if (!active) return;
    setNowMs(Date.now());
    const timer = window.setInterval(() => setNowMs(Date.now()), 1_000);
    return () => window.clearInterval(timer);
  }, [active]);
  if (!open) return null;
  return (
    <div className="modal-backdrop" role="presentation">
      <section
        aria-labelledby="record-title"
        aria-modal="true"
        className="dialog"
        role="dialog"
      >
        <header>
          <div>
            <Icon name="record" />
            <div>
              <h2 id="record-title">{t("Packet recording")}</h2>
              <p>{t("Explicit, bounded PCAPNG capture")}</p>
            </div>
          </div>
          <button
            aria-label={t("Close")}
            className="icon-button"
            onClick={onClose}
            type="button"
          >
            <Icon name="close" />
          </button>
        </header>
        {active ? (
          <div className="recording-active">
            <span className="recording-active__dot" />
            <div>
              <strong>
                {recoverableError
                  ? t("Recording needs finalization")
                  : t("Recording packets locally")}
              </strong>
              <p>{state?.output_path}</p>
              <small>
                {recoverableError
                  ? t(state?.message ?? "")
                  : t("{duration} / {bytes} / limit {limit} MB per file", {
                      duration: formatDuration(
                        Math.max(0, nowMs - (state?.started_at_ms ?? nowMs)),
                      ),
                      bytes: formatBytes(state?.bytes_written ?? 0),
                      limit: state?.max_file_size_mb ?? 0,
                    })}
              </small>
            </div>
          </div>
        ) : (
          <>
            <div className="warning-copy">
              <Icon name="shield" />
              <p>
                {t(
                  "Unlike normal observation, a packet recording can contain sensitive payloads and credentials. Only start it with authorization.",
                )}
              </p>
            </div>
            <label className="dialog-field">
              <span>{t("Output file")}</span>
              <input
                onChange={(event) => setPath(event.target.value)}
                value={path}
              />
            </label>
            <div className="dialog-fields">
              <label className="dialog-field">
                <span>{t("Maximum size per file")}</span>
                <select
                  onChange={(event) => setSize(Number(event.target.value))}
                  value={size}
                >
                  <option value="64">64 MB</option>
                  <option value="256">256 MB</option>
                  <option value="512">512 MB</option>
                  <option value="1024">1 GB</option>
                </select>
              </label>
              <label className="dialog-field">
                <span>{t("Duration")}</span>
                <select
                  onChange={(event) => setDuration(Number(event.target.value))}
                  value={duration}
                >
                  <option value="5">{t("5 minutes")}</option>
                  <option value="15">{t("15 minutes")}</option>
                  <option value="30">{t("30 minutes")}</option>
                  <option value="60">{t("1 hour")}</option>
                </select>
              </label>
            </div>
            <label className="dialog-check">
              <input
                checked={rotate}
                onChange={(event) => setRotate(event.target.checked)}
                type="checkbox"
              />
              <span>
                <strong>{t("Rotate recording files")}</strong>
                <small>{t("Keep a bounded ring of at most four files.")}</small>
              </span>
            </label>
            <p className="dialog-note">
              {t(
                "Windows uses the built-in free Pktmon tool when available. Linux requires a locally installed free capture helper and appropriate permissions.",
              )}
            </p>
          </>
        )}
        <footer>
          <button
            className="button button--secondary"
            onClick={onClose}
            type="button"
          >
            {t("Cancel")}
          </button>
          {active ? (
            <button
              className="button button--danger"
              onClick={onStop}
              type="button"
            >
              <Icon name="stop" />
              {recoverableError ? t("Retry finalization") : t("Stop recording")}
            </button>
          ) : (
            <button
              className="button button--primary"
              disabled={!path.trim()}
              onClick={() =>
                onStart({
                  output_path: path.trim(),
                  max_file_size_mb: size,
                  rotate,
                  duration_limit_seconds: duration * 60,
                })
              }
              type="button"
            >
              <Icon name="record" />
              {t("Start recording")}
            </button>
          )}
        </footer>
      </section>
    </div>
  );
}

interface ExportDialogProps {
  open: boolean;
  query: string;
  sessionId: string | null;
  fromMs: number | null;
  toMs: number | null;
  onClose: () => void;
  onExport: (request: ExportRequest) => void;
}

export function ExportDialog({
  open,
  query,
  sessionId,
  fromMs,
  toMs,
  onClose,
  onExport,
}: ExportDialogProps) {
  const t = useI18n();
  const [format, setFormat] = useState<"csv" | "json">("csv");
  const [path, setPath] = useState("sabicheiro-export.csv");
  useEffect(() => setPath(`sabicheiro-export.${format}`), [format]);
  if (!open) return null;
  return (
    <div className="modal-backdrop" role="presentation">
      <section
        aria-labelledby="export-title"
        aria-modal="true"
        className="dialog dialog--compact"
        role="dialog"
      >
        <header>
          <div>
            <Icon name="download" />
            <div>
              <h2 id="export-title">{t("Export current view")}</h2>
              <p>{t("Metadata only; packet payloads are never included")}</p>
            </div>
          </div>
          <button
            aria-label={t("Close")}
            className="icon-button"
            onClick={onClose}
            type="button"
          >
            <Icon name="close" />
          </button>
        </header>
        <div className="format-choice">
          <button
            className={format === "csv" ? "is-active" : undefined}
            onClick={() => setFormat("csv")}
            type="button"
          >
            <strong>CSV</strong>
            <span>{t("For spreadsheets and analysis")}</span>
          </button>
          <button
            className={format === "json" ? "is-active" : undefined}
            onClick={() => setFormat("json")}
            type="button"
          >
            <strong>JSON</strong>
            <span>{t("Structured full-fidelity records")}</span>
          </button>
        </div>
        <label className="dialog-field">
          <span>{t("Output file")}</span>
          <input
            onChange={(event) => setPath(event.target.value)}
            value={path}
          />
        </label>
        <div className="export-scope">
          <span>{t("Scope")}</span>
          <strong>
            {query
              ? t("Filter: {query}", { query })
              : sessionId
                ? t("Session: {session}", { session: sessionId })
                : fromMs || toMs
                  ? t("Selected history range")
                  : t("All visible records")}
          </strong>
        </div>
        <footer>
          <button
            className="button button--secondary"
            onClick={onClose}
            type="button"
          >
            {t("Cancel")}
          </button>
          <button
            className="button button--primary"
            disabled={!path.trim()}
            onClick={() =>
              onExport({
                format,
                path: path.trim(),
                session_id: sessionId,
                process_key: null,
                from_ms: fromMs,
                to_ms: toMs,
                query: query || null,
              })
            }
            type="button"
          >
            {t("Export {format}", { format: format.toUpperCase() })}
          </button>
        </footer>
      </section>
    </div>
  );
}

export function ErrorBanner({
  title = "Operation could not be completed",
  message,
  onClose,
}: {
  title?: string | undefined;
  message: string;
  onClose: () => void;
}) {
  const t = useI18n();
  return (
    <div className="error-banner" role="alert">
      <span>!</span>
      <div>
        <strong>{t(title)}</strong>
        <p>{t(message)}</p>
      </div>
      <button aria-label={t("Dismiss error")} onClick={onClose} type="button">
        <Icon name="close" />
      </button>
    </div>
  );
}

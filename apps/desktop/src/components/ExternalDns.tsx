import { useEffect, useRef, useState } from "react";
import { desktopApi } from "#backend";
import { useI18n } from "../i18n";
import { LatestRequest } from "../lib/latestRequest";
import { formatDateTime } from "../lib/format";
import { usePresentation } from "../privacy/PresentationMode";
import type {
  DnsImportRequest,
  DnsImportPreview,
  DnsImportReport,
  ExternalDnsPage,
  ExternalDnsQuery,
} from "../types";

type ImportState =
  | { kind: "idle" }
  | { kind: "reading" }
  | { kind: "preview"; request: DnsImportRequest; preview: DnsImportPreview }
  | { kind: "importing"; jobId: string }
  | { kind: "done"; report: DnsImportReport }
  | { kind: "error"; message: string };

function errorMessage(error: unknown): string {
  if (error instanceof Error) return error.message;
  if (
    typeof error === "object" &&
    error !== null &&
    "message" in error &&
    typeof error.message === "string"
  )
    return error.message;
  return "The operation failed. Check Diagnostics and retry.";
}

export function ExternalDns() {
  const t = useI18n();
  const { active, display, displayText, restoreText } = usePresentation();
  const [source, setSource] = useState("");
  const [state, setState] = useState<ImportState>({ kind: "idle" });
  const [page, setPage] = useState<ExternalDnsPage>({
    rows: [],
    next_id: null,
  });
  const [client, setClient] = useState("");
  const [domain, setDomain] = useState("");
  const [from, setFrom] = useState("");
  const [to, setTo] = useState("");
  const [queryError, setQueryError] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);
  const gates = useRef({
    file: new LatestRequest(),
    query: new LatestRequest(),
  });
  const job = useRef<string | null>(null);
  const busy = state.kind === "reading" || state.kind === "importing";
  useEffect(
    () => () => {
      gates.current.file.invalidate();
      gates.current.query.invalidate();
      if (job.current)
        void desktopApi.cancelDnsImport(job.current).catch(() => undefined);
    },
    [],
  );

  const query = async (beforeId: number | null = null) => {
    const token = gates.current.query.begin();
    setLoading(true);
    setQueryError(null);
    try {
      const result = await desktopApi.queryExternalDns({
        from_ms: from ? new Date(from).getTime() : null,
        to_ms: to ? new Date(to).getTime() : null,
        client_ip: client.trim() || null,
        domain: domain.trim() || null,
        before_id: beforeId,
        limit: 100,
      });
      if (gates.current.query.isCurrent(token)) setPage(result);
    } catch (error) {
      if (gates.current.query.isCurrent(token))
        setQueryError(errorMessage(error));
    } finally {
      if (gates.current.query.isCurrent(token)) setLoading(false);
    }
  };
  useEffect(() => {
    void query();
  }, []);

  const read = async (file: File) => {
    const token = gates.current.file.begin();
    setState({ kind: "reading" });
    try {
      if (file.size > 16 * 1024 * 1024)
        throw new Error(t("The import exceeds the 16 MiB limit."));
      const content = await file.text();
      const format = file.name.toLowerCase().endsWith(".json") ? "json" : "csv";
      const request: DnsImportRequest = {
        format,
        content,
        source: source.trim(),
      };
      const preview = await desktopApi.previewDnsImport(request);
      if (gates.current.file.isCurrent(token))
        setState({ kind: "preview", request, preview });
    } catch (error) {
      if (gates.current.file.isCurrent(token))
        setState({ kind: "error", message: errorMessage(error) });
    }
  };
  const commit = async () => {
    if (state.kind !== "preview") return;
    const request = state.request;
    const token = gates.current.file.begin();
    const jobId = crypto.randomUUID();
    job.current = jobId;
    setState({ kind: "importing", jobId });
    try {
      const report = await desktopApi.importDnsQueries(request, jobId);
      if (gates.current.file.isCurrent(token)) {
        setState({ kind: "done", report });
        await query();
      }
    } catch (error) {
      if (gates.current.file.isCurrent(token))
        setState({ kind: "error", message: errorMessage(error) });
    } finally {
      if (job.current === jobId) job.current = null;
    }
  };
  const renderRow = (row: ExternalDnsQuery, key: string, rowSource: string) => (
    <tr key={key}>
      <td>{formatDateTime(row.timestamp_ms)}</td>
      <td className="mono">
        {active ? display("local_ip", row.client_ip) : row.client_ip}
      </td>
      <td>{active ? display("domain", row.query_name) : row.query_name}</td>
      <td>{row.query_type ?? "—"}</td>
      <td>
        {row.result ? (active ? displayText(row.result) : row.result) : "—"}
      </td>
      <td>{active ? display("generic", rowSource) : rowSource}</td>
    </tr>
  );
  const preview = state.kind === "preview" ? state.preview : null;
  return (
    <section
      className="external-dns"
      id="external-dns"
      aria-labelledby="external-dns-title"
    >
      <header>
        <div>
          <h2 id="external-dns-title">{t("External DNS evidence")}</h2>
          <p>
            {t(
              "Import resolver logs to follow clients and queried domains. DNS queries do not prove a connection or identify a remote process.",
            )}
          </p>
        </div>
      </header>
      <div className="dns-import-controls">
        <label>
          {t("Source name")}
          <input
            value={active ? displayText(source) : source}
            maxLength={128}
            disabled={busy}
            placeholder={t("Home resolver")}
            onChange={(event) => {
              setSource(restoreText(event.target.value));
              gates.current.file.invalidate();
              setState({ kind: "idle" });
            }}
          />
        </label>
        <label>
          {t("CSV or JSON file")}
          <input
            type="file"
            accept=".csv,.json"
            disabled={busy || !source.trim()}
            onChange={(event) => {
              const file = event.target.files?.[0];
              event.target.value = "";
              if (file) void read(file);
            }}
          />
        </label>
        <div className="dns-template-links">
          <a href="/templates/dns-queries.csv" download>
            {t("CSV template")}
          </a>
          <a href="/templates/dns-queries.json" download>
            {t("JSON template")}
          </a>
        </div>
      </div>
      <p className="dns-import-note">
        {t(
          "Timestamps must include a timezone. Imports stay local and are deduplicated per source. Limit: 16 MiB / 100,000 queries.",
        )}
      </p>
      <div role="status" aria-live="polite">
        {state.kind === "reading" && <p>{t("Validating DNS queries...")}</p>}
        {state.kind === "importing" && (
          <p>
            {t("Importing DNS queries...")}{" "}
            <button
              type="button"
              onClick={() =>
                void desktopApi
                  .cancelDnsImport(state.jobId)
                  .catch((error: unknown) => setQueryError(errorMessage(error)))
              }
            >
              {t("Cancel import")}
            </button>
          </p>
        )}
        {state.kind === "done" && (
          <p>
            {t(
              "Imported {inserted} queries; skipped {duplicates} duplicates.",
              { ...state.report },
            )}
          </p>
        )}
      </div>
      {state.kind === "error" && (
        <p role="alert" className="dns-error">
          {state.message}
        </p>
      )}
      {preview && (
        <div className="dns-preview-heading">
          <p>
            {t("{count} valid queries. Review the sample before importing.", {
              count: preview.total_rows,
            })}
          </p>
          <button
            className="button button--primary"
            type="button"
            onClick={() => void commit()}
          >
            {t("Import queries")}
          </button>
          <button
            className="button"
            type="button"
            onClick={() => {
              gates.current.file.invalidate();
              setState({ kind: "idle" });
            }}
          >
            {t("Discard preview")}
          </button>
        </div>
      )}
      <form
        className="dns-query-controls"
        onSubmit={(event) => {
          event.preventDefault();
          void query();
        }}
      >
        <label>
          {t("Client IP")}
          <input
            value={active ? displayText(client) : client}
            onChange={(event) => setClient(restoreText(event.target.value))}
          />
        </label>
        <label>
          {t("Domain contains")}
          <input
            value={active ? displayText(domain) : domain}
            onChange={(event) => setDomain(restoreText(event.target.value))}
          />
        </label>
        <label>
          {t("From")}
          <input
            type="datetime-local"
            value={from}
            onChange={(event) => setFrom(event.target.value)}
          />
        </label>
        <label>
          {t("To")}
          <input
            type="datetime-local"
            value={to}
            onChange={(event) => setTo(event.target.value)}
          />
        </label>
        <button className="button" type="submit" disabled={loading}>
          {t("Apply filters")}
        </button>
      </form>
      {queryError && (
        <p role="alert" className="dns-error">
          {queryError}
        </p>
      )}
      <div
        className="dns-table-scroll"
        tabIndex={0}
        role="region"
        aria-label={t("DNS query evidence")}
        aria-busy={loading}
      >
        <table>
          <caption className="sr-only">
            {preview ? t("Import preview") : t("Imported DNS history")}
          </caption>
          <thead>
            <tr>
              {["Time", "Client IP", "Domain", "Type", "Result", "Source"].map(
                (label) => (
                  <th scope="col" key={label}>
                    {t(label)}
                  </th>
                ),
              )}
            </tr>
          </thead>
          <tbody>
            {preview
              ? preview.sample.map((row, index) =>
                  renderRow(row, `preview-${index}`, preview.source),
                )
              : page.rows.map((row) =>
                  renderRow(row.query, String(row.id), row.source),
                )}
          </tbody>
        </table>
        {!preview && page.rows.length === 0 && (
          <p className="dns-empty">
            {t(
              "No imported queries match. Import a file or adjust your filters.",
            )}
          </p>
        )}
      </div>
      {!preview && page.next_id !== null && (
        <button
          className="button"
          disabled={loading}
          onClick={() => void query(page.next_id)}
          type="button"
        >
          {t("Next page")}
        </button>
      )}
    </section>
  );
}

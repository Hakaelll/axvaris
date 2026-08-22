import { useEffect, useMemo, useRef, useState } from "react";
import { useI18n } from "../i18n";
import type { HistoryEntry } from "../types";
import { formatDateTime, formatDuration } from "../lib/format";
import {
  usePresentation,
  type SensitiveKind,
} from "../privacy/PresentationMode";

type ColumnId =
  | "time"
  | "process"
  | "destination"
  | "dns"
  | "protocol"
  | "path"
  | "state"
  | "quality"
  | "source";
type SortDirection = "ascending" | "descending";
type GroupMode = "none" | "process" | "domain" | "source";

interface TablePreset {
  name: string;
  visible: ColumnId[];
  widths: Record<ColumnId, number>;
  sort: ColumnId;
  direction: SortDirection;
}

interface EvidenceTableProps {
  entries: HistoryEntry[];
  loading: boolean;
  pinnedIds: string[];
  onTogglePin: (connectionId: string) => void;
  onOpen: (entry: HistoryEntry) => void;
}

const columnIds: ColumnId[] = [
  "time",
  "process",
  "destination",
  "dns",
  "protocol",
  "path",
  "state",
  "quality",
  "source",
];
const defaultWidths: Record<ColumnId, number> = {
  time: 158,
  process: 190,
  destination: 210,
  dns: 190,
  protocol: 110,
  path: 155,
  state: 105,
  quality: 125,
  source: 150,
};
const builtInPresets: TablePreset[] = [
  {
    name: "Evidence",
    visible: columnIds,
    widths: defaultWidths,
    sort: "time",
    direction: "descending",
  },
  {
    name: "Causal",
    visible: [
      "time",
      "process",
      "destination",
      "dns",
      "path",
      "quality",
      "source",
    ],
    widths: { ...defaultWidths, dns: 230, path: 190 },
    sort: "time",
    direction: "descending",
  },
  {
    name: "Compact",
    visible: ["time", "process", "destination", "protocol", "state"],
    widths: { ...defaultWidths, time: 140, process: 165, destination: 180 },
    sort: "time",
    direction: "descending",
  },
];

const rowHeight = 50;
const viewportHeight = 510;
const pageSize = 100;

function valueFor(entry: HistoryEntry, column: ColumnId): string | number {
  const connection = entry.connection;
  switch (column) {
    case "time":
      return connection.first_seen_ms;
    case "process":
      return `${connection.process.name}\u0000${connection.key.process_instance ?? ""}`;
    case "destination":
      return connection.domain ?? connection.key.remote.address;
    case "dns":
      return connection.hostname ?? connection.domain ?? "";
    case "protocol":
      return `${connection.protocol}:${connection.key.remote.port}`;
    case "path":
      return connection.interface ?? "";
    case "state":
      return connection.state;
    case "quality":
      return connection.evidence.correlation_quality;
    case "source":
      return connection.evidence.observation_source;
  }
}

function cell(
  entry: HistoryEntry,
  column: ColumnId,
  t: ReturnType<typeof useI18n>,
  display: (kind: SensitiveKind, value: string | null | undefined) => string,
) {
  const connection = entry.connection;
  const remoteKind: SensitiveKind =
    connection.key.remote.scope === "global" ? "public_ip" : "local_ip";
  switch (column) {
    case "time":
      return (
        <>
          <strong>{formatDateTime(connection.first_seen_ms)}</strong>
          <small>{formatDuration(connection.duration_ms)}</small>
        </>
      );
    case "process":
      return (
        <>
          <strong>{display("process", connection.process.name)}</strong>
          <small>
            {connection.key.process_instance ??
              `PID ${connection.process.pid ?? "?"}`}
          </small>
        </>
      );
    case "destination":
      return (
        <>
          <strong>
            {connection.domain
              ? display("domain", connection.domain)
              : display(remoteKind, connection.key.remote.address)}
          </strong>
          <small>
            {display(remoteKind, connection.key.remote.address)}:
            {connection.key.remote.port}
          </small>
        </>
      );
    case "dns":
      return (
        <>
          <strong>
            {connection.hostname || connection.domain
              ? display("domain", connection.hostname ?? connection.domain)
              : t("No DNS evidence")}
          </strong>
          <small>
            {connection.hostname || connection.domain
              ? t("Observed or correlated name")
              : t("IP evidence remains valid")}
          </small>
        </>
      );
    case "protocol":
      return (
        <>
          <strong>
            {connection.protocol.toUpperCase()} {connection.key.remote.port}
          </strong>
          <small>{t(connection.direction)}</small>
        </>
      );
    case "path":
      return (
        <>
          <strong>
            {connection.interface
              ? display("interface", connection.interface)
              : t("System default")}
          </strong>
          <small>{t("Open row for native route evidence")}</small>
        </>
      );
    case "state":
      return (
        <>
          <strong>{t(connection.state)}</strong>
          <small>{connection.active ? t("active") : t("closed")}</small>
        </>
      );
    case "quality":
      return (
        <>
          <strong>{t(connection.evidence.correlation_quality)}</strong>
          <small>{t(connection.evidence.endpoint_role)}</small>
        </>
      );
    case "source":
      return (
        <>
          <strong>{t(connection.evidence.observation_source)}</strong>
          <small>#{connection.evidence.sequence || "—"}</small>
        </>
      );
  }
}

function loadCustomPreset(): TablePreset | null {
  try {
    const raw = localStorage.getItem("sabicheiro:evidence-table-preset:v1");
    if (!raw) return null;
    const parsed = JSON.parse(raw) as TablePreset;
    if (!Array.isArray(parsed.visible) || !columnIds.includes(parsed.sort))
      return null;
    return parsed;
  } catch {
    return null;
  }
}

export function EvidenceTable({
  entries,
  loading,
  pinnedIds,
  onTogglePin,
  onOpen,
}: EvidenceTableProps) {
  const t = useI18n();
  const { display } = usePresentation();
  const [preset, setPreset] = useState<TablePreset>(
    () => loadCustomPreset() ?? builtInPresets[0]!,
  );
  const [mode, setMode] = useState<"virtual" | "semantic">("virtual");
  const [group, setGroup] = useState<GroupMode>("none");
  const [scrollTop, setScrollTop] = useState(0);
  const [page, setPage] = useState(0);
  const viewportRef = useRef<HTMLDivElement>(null);

  const sorted = useMemo(() => {
    const indexed = entries.map((entry, index) => ({ entry, index }));
    indexed.sort((left, right) => {
      const a = valueFor(left.entry, preset.sort);
      const b = valueFor(right.entry, preset.sort);
      const compared =
        typeof a === "number" && typeof b === "number"
          ? a - b
          : String(a).localeCompare(String(b));
      const stable = compared || left.index - right.index;
      return preset.direction === "ascending" ? stable : -stable;
    });
    if (group === "none") return indexed.map((item) => item.entry);
    const groupValue = (entry: HistoryEntry) =>
      group === "process"
        ? entry.connection.process.name
        : group === "domain"
          ? (entry.connection.domain ?? entry.connection.key.remote.address)
          : entry.connection.evidence.observation_source;
    return indexed
      .map((item) => item.entry)
      .sort(
        (a, b) =>
          groupValue(a).localeCompare(groupValue(b)) ||
          a.connection.id.localeCompare(b.connection.id),
      );
  }, [entries, group, preset.direction, preset.sort]);

  useEffect(() => {
    setPage((current) =>
      Math.min(current, Math.max(0, Math.ceil(sorted.length / pageSize) - 1)),
    );
  }, [sorted.length]);

  const start = Math.max(0, Math.floor(scrollTop / rowHeight) - 6);
  const end = Math.min(
    sorted.length,
    Math.ceil((scrollTop + viewportHeight) / rowHeight) + 6,
  );
  const virtualRows = sorted.slice(start, end);
  const semanticRows = sorted.slice(page * pageSize, (page + 1) * pageSize);
  const gridTemplate = preset.visible
    .map((column) => `${preset.widths[column] ?? defaultWidths[column]}px`)
    .join(" ");

  const applyPreset = (next: TablePreset) => {
    setPreset({
      ...next,
      widths: { ...next.widths },
      visible: [...next.visible],
    });
    setScrollTop(0);
    if (viewportRef.current) viewportRef.current.scrollTop = 0;
  };
  const toggleColumn = (column: ColumnId) => {
    setPreset((current) => ({
      ...current,
      name: "Custom",
      visible: current.visible.includes(column)
        ? current.visible.filter((item) => item !== column)
        : columnIds.filter(
            (item) => current.visible.includes(item) || item === column,
          ),
    }));
  };
  const changeSort = (column: ColumnId) => {
    setPreset((current) => ({
      ...current,
      name: "Custom",
      sort: column,
      direction:
        current.sort === column && current.direction === "ascending"
          ? "descending"
          : "ascending",
    }));
  };
  const savePreset = () => {
    const custom = { ...preset, name: "Saved local preset" };
    localStorage.setItem(
      "sabicheiro:evidence-table-preset:v1",
      JSON.stringify(custom),
    );
    setPreset(custom);
  };

  const row = (entry: HistoryEntry, virtual: boolean) => {
    const pinned = pinnedIds.includes(entry.connection.id);
    return (
      <div
        aria-rowindex={sorted.indexOf(entry) + 2}
        className={`evidence-grid__row${pinned ? " is-pinned" : ""}`}
        key={`${entry.session_id}:${entry.connection.id}`}
        role="row"
        style={{
          gridTemplateColumns: `${gridTemplate} 88px`,
          height: virtual ? rowHeight : undefined,
        }}
      >
        {preset.visible.map((column) => (
          <div key={column} role="cell">
            {cell(entry, column, t, display)}
          </div>
        ))}
        <div className="evidence-grid__actions" role="cell">
          <button
            aria-pressed={pinned}
            onClick={() => onTogglePin(entry.connection.id)}
            type="button"
          >
            {pinned ? t("Unpin") : t("Pin")}
          </button>
          <button onClick={() => onOpen(entry)} type="button">
            {t("Open")}
          </button>
        </div>
      </div>
    );
  };

  return (
    <section className="evidence-grid" aria-labelledby="evidence-grid-title">
      <header>
        <div>
          <span className="eyebrow">{t("FIRST-CLASS EVIDENCE TABLE")}</span>
          <h2 id="evidence-grid-title">{t("Connections")}</h2>
          <p>{t("The exact filtered IDs used by timeline, map and export.")}</p>
        </div>
        <strong>{t("{count} exact rows", { count: sorted.length })}</strong>
      </header>
      <div className="evidence-grid__toolbar">
        <label>
          <span>{t("Preset")}</span>
          <select
            value={preset.name}
            onChange={(event) => {
              const next = [...builtInPresets, loadCustomPreset()].find(
                (item) => item?.name === event.target.value,
              );
              if (next) applyPreset(next);
            }}
          >
            {builtInPresets.map((item) => (
              <option key={item.name} value={item.name}>
                {t(item.name)}
              </option>
            ))}
            {loadCustomPreset() && (
              <option value="Saved local preset">
                {t("Saved local preset")}
              </option>
            )}
            <option disabled value="Custom">
              {t("Custom")}
            </option>
          </select>
        </label>
        <label>
          <span>{t("Group")}</span>
          <select
            onChange={(event) => setGroup(event.target.value as GroupMode)}
            value={group}
          >
            <option value="none">{t("None")}</option>
            <option value="process">{t("Process")}</option>
            <option value="domain">{t("Domain")}</option>
            <option value="source">{t("Source")}</option>
          </select>
        </label>
        <div
          className="segmented"
          aria-label={t("Accessibility rendering mode")}
        >
          <button
            aria-pressed={mode === "virtual"}
            className={mode === "virtual" ? "is-active" : undefined}
            onClick={() => setMode("virtual")}
            type="button"
          >
            {t("Virtual 100k+")}
          </button>
          <button
            aria-pressed={mode === "semantic"}
            className={mode === "semantic" ? "is-active" : undefined}
            onClick={() => setMode("semantic")}
            type="button"
          >
            {t("Screen reader pages")}
          </button>
        </div>
        <details>
          <summary>{t("Columns and widths")}</summary>
          <div>
            {columnIds.map((column) => (
              <label key={column}>
                <input
                  checked={preset.visible.includes(column)}
                  disabled={
                    preset.visible.length === 1 &&
                    preset.visible.includes(column)
                  }
                  onChange={() => toggleColumn(column)}
                  type="checkbox"
                />
                {t(column)}
                <input
                  aria-label={t("{column} width", { column: t(column) })}
                  max={360}
                  min={80}
                  onChange={(event) =>
                    setPreset((current) => ({
                      ...current,
                      name: "Custom",
                      widths: {
                        ...current.widths,
                        [column]: Number(event.target.value),
                      },
                    }))
                  }
                  type="range"
                  value={preset.widths[column] ?? defaultWidths[column]}
                />
              </label>
            ))}
          </div>
        </details>
        <button
          className="button button--secondary"
          onClick={savePreset}
          type="button"
        >
          {t("Save preset locally")}
        </button>
      </div>
      {loading ? (
        <div className="loading-line" />
      ) : mode === "virtual" ? (
        <div
          aria-colcount={preset.visible.length + 1}
          aria-label={t("Virtualized connection evidence")}
          aria-rowcount={sorted.length + 1}
          className="evidence-grid__viewport"
          onScroll={(event) => setScrollTop(event.currentTarget.scrollTop)}
          ref={viewportRef}
          role="table"
          style={{ height: viewportHeight }}
          tabIndex={0}
        >
          <div
            className="evidence-grid__head"
            role="row"
            style={{ gridTemplateColumns: `${gridTemplate} 88px` }}
          >
            {preset.visible.map((column) => (
              <button
                aria-sort={preset.sort === column ? preset.direction : "none"}
                key={column}
                onClick={() => changeSort(column)}
                role="columnheader"
                type="button"
              >
                {t(column)}
                {preset.sort === column
                  ? preset.direction === "ascending"
                    ? " ↑"
                    : " ↓"
                  : ""}
              </button>
            ))}
            <span role="columnheader">{t("Actions")}</span>
          </div>
          <div
            style={{
              height: sorted.length * rowHeight,
              position: "relative",
              minWidth: `calc(${gridTemplate.split(" ").join(" + ")} + 88px)`,
            }}
          >
            <div
              style={{ position: "absolute", top: start * rowHeight, left: 0 }}
            >
              {virtualRows.map((entry) => row(entry, true))}
            </div>
          </div>
        </div>
      ) : (
        <div className="evidence-grid__semantic">
          <table>
            <caption>
              {t("Page {page} of {pages}; {count} exact rows", {
                page: page + 1,
                pages: Math.max(1, Math.ceil(sorted.length / pageSize)),
                count: sorted.length,
              })}
            </caption>
            <thead>
              <tr>
                {preset.visible.map((column) => (
                  <th
                    aria-sort={
                      preset.sort === column ? preset.direction : "none"
                    }
                    key={column}
                  >
                    <button onClick={() => changeSort(column)} type="button">
                      {t(column)}
                    </button>
                  </th>
                ))}
                <th>{t("Actions")}</th>
              </tr>
            </thead>
            <tbody>
              {semanticRows.map((entry) => (
                <tr key={`${entry.session_id}:${entry.connection.id}`}>
                  {preset.visible.map((column) => (
                    <td key={column}>{cell(entry, column, t, display)}</td>
                  ))}
                  <td>
                    <button
                      onClick={() => onTogglePin(entry.connection.id)}
                      type="button"
                    >
                      {pinnedIds.includes(entry.connection.id)
                        ? t("Unpin")
                        : t("Pin")}
                    </button>
                    <button onClick={() => onOpen(entry)} type="button">
                      {t("Open")}
                    </button>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
          <nav aria-label={t("Evidence pages")}>
            <button
              disabled={page === 0}
              onClick={() => setPage((current) => current - 1)}
              type="button"
            >
              {t("Previous")}
            </button>
            <span>
              {page + 1} / {Math.max(1, Math.ceil(sorted.length / pageSize))}
            </span>
            <button
              disabled={(page + 1) * pageSize >= sorted.length}
              onClick={() => setPage((current) => current + 1)}
              type="button"
            >
              {t("Next")}
            </button>
          </nav>
        </div>
      )}
      {!loading && sorted.length === 0 && (
        <p className="list-empty">
          {t("No retained evidence matches this query.")}
        </p>
      )}
    </section>
  );
}

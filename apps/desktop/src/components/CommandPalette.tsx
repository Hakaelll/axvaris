import { useEffect, useMemo, useRef, useState } from "react";
import { useI18n } from "../i18n";
import type { PrimaryView, ViewMode } from "../types";
import { Icon, type IconName } from "./Icon";

interface CommandPaletteProps {
  open: boolean;
  paused: boolean;
  recording: boolean;
  localTrafficVisible: boolean;
  onClose: () => void;
  onView: (view: PrimaryView) => void;
  onVisualization: (mode: ViewMode) => void;
  onPause: () => void;
  onRecord: () => void;
  onExport: () => void;
  onClearSession: () => void;
  onSearchPreset: (query: string) => void;
  onToggleLocalTraffic: () => void;
}

export function CommandPalette({
  open,
  paused,
  recording,
  localTrafficVisible,
  onClose,
  onView,
  onVisualization,
  onPause,
  onRecord,
  onExport,
  onClearSession,
  onSearchPreset,
  onToggleLocalTraffic,
}: CommandPaletteProps) {
  const t = useI18n();
  const [query, setQuery] = useState("");
  const [activeIndex, setActiveIndex] = useState(0);
  const inputRef = useRef<HTMLInputElement>(null);
  const actions = useMemo(
    () => [
      {
        label: t("Open Today"),
        keywords: "world network mundo red",
        icon: "globe" as IconName,
        run: () => onView("today"),
      },
      {
        label: t("Switch to 3D globe"),
        keywords: "visualization visualización globo",
        icon: "globe" as IconName,
        run: () => {
          onView("live");
          onVisualization("globe");
        },
      },
      {
        label: t("Switch to 2D map"),
        keywords: "visualization fallback visualización mapa",
        icon: "map" as IconName,
        run: () => {
          onView("live");
          onVisualization("map");
        },
      },
      {
        label: t("Open Investigate"),
        keywords: "applications aplicaciones procesos",
        icon: "process" as IconName,
        run: () => onView("investigate"),
      },
      {
        label: t("Open Exposure"),
        keywords: "countries domains ip países dominios destinos",
        icon: "map" as IconName,
        run: () => onView("exposure"),
      },
      {
        label: t("Search process"),
        keywords: "filter application filtro aplicación proceso",
        icon: "search" as IconName,
        run: () => onSearchPreset("process:"),
      },
      {
        label: t("Search destination"),
        keywords: "filter domain ip filtro dominio destino",
        icon: "search" as IconName,
        run: () => onSearchPreset("domain:"),
      },
      {
        label: t("Open Live"),
        keywords: "sessions timeline sesiones historial",
        icon: "history" as IconName,
        run: () => onView("live"),
      },
      {
        label: paused ? t("Resume visualization") : t("Pause visualization"),
        keywords: "freeze",
        icon: paused ? ("play" as IconName) : ("pause" as IconName),
        run: onPause,
      },
      {
        label: recording
          ? t("Stop packet recording")
          : t("Start packet recording"),
        keywords: "pcapng pktmon dumpcap",
        icon: recording ? ("stop" as IconName) : ("record" as IconName),
        run: onRecord,
      },
      {
        label: t("Clear current session"),
        keywords: "reset",
        icon: "close" as IconName,
        run: onClearSession,
      },
      {
        label: localTrafficVisible
          ? t("Hide local traffic")
          : t("Show local traffic"),
        keywords: "loopback lan localhost",
        icon: "activity" as IconName,
        run: onToggleLocalTraffic,
      },
      {
        label: t("Export current view"),
        keywords: "csv json",
        icon: "download" as IconName,
        run: onExport,
      },
      {
        label: t("Open Data & Health"),
        keywords: "privacy diagnostics",
        icon: "settings" as IconName,
        run: () => onView("health"),
      },
    ],
    [
      localTrafficVisible,
      onClearSession,
      onExport,
      onPause,
      onRecord,
      onSearchPreset,
      onToggleLocalTraffic,
      onView,
      onVisualization,
      paused,
      recording,
      t,
    ],
  );
  const filtered = actions.filter((action) =>
    `${action.label} ${action.keywords}`
      .toLowerCase()
      .includes(query.toLowerCase()),
  );

  useEffect(() => {
    if (!open) return;
    setQuery("");
    setActiveIndex(0);
    requestAnimationFrame(() => inputRef.current?.focus());
  }, [open]);
  useEffect(() => setActiveIndex(0), [query]);
  if (!open) return null;

  const run = (index: number) => {
    const action = filtered[index];
    if (!action) return;
    action.run();
    onClose();
  };

  return (
    <div
      className="modal-backdrop"
      onMouseDown={(event) => {
        if (event.target === event.currentTarget) onClose();
      }}
      role="presentation"
    >
      <section
        aria-label={t("Command palette")}
        aria-modal="true"
        className="command-palette"
        role="dialog"
      >
        <label>
          <Icon name="search" />
          <input
            onChange={(event) => setQuery(event.target.value)}
            onKeyDown={(event) => {
              if (event.key === "Escape") onClose();
              if (event.key === "ArrowDown") {
                event.preventDefault();
                setActiveIndex((index) =>
                  Math.max(0, Math.min(filtered.length - 1, index + 1)),
                );
              }
              if (event.key === "ArrowUp") {
                event.preventDefault();
                setActiveIndex((index) => Math.max(0, index - 1));
              }
              if (event.key === "Enter") run(activeIndex);
            }}
            placeholder={t("Type a command...")}
            ref={inputRef}
            value={query}
          />
          <kbd>ESC</kbd>
        </label>
        <div role="listbox">
          {filtered.map((action, index) => (
            <button
              aria-selected={index === activeIndex}
              className={index === activeIndex ? "is-active" : undefined}
              key={action.label}
              onClick={() => run(index)}
              onMouseEnter={() => setActiveIndex(index)}
              role="option"
              type="button"
            >
              <Icon name={action.icon} />
              <span>{action.label}</span>
              {index === activeIndex && <kbd>ENTER</kbd>}
            </button>
          ))}
          {filtered.length === 0 && <p>{t("No matching commands.")}</p>}
        </div>
        <footer>
          <span>
            <kbd>UP/DOWN</kbd> {t("navigate")}
          </span>
          <span>
            <kbd>ENTER</kbd> {t("select")}
          </span>
          <span>{t("Everything runs locally")}</span>
        </footer>
      </section>
    </div>
  );
}

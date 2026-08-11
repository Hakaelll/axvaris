import type {
  CaptureMode,
  CaptureSummary,
  NetworkInterface,
  PrimaryView,
} from "../types";
import { useI18n } from "../i18n";
import { Icon, type IconName } from "./Icon";

const navigation: { id: PrimaryView; label: string; icon: IconName }[] = [
  { id: "globe", label: "Globe", icon: "globe" },
  { id: "processes", label: "Processes", icon: "process" },
  { id: "destinations", label: "Destinations", icon: "map" },
  { id: "history", label: "History", icon: "history" },
];

interface TopBarProps {
  primaryView: PrimaryView;
  onView: (view: PrimaryView) => void;
  query: string;
  queryError: string | null;
  onQuery: (query: string) => void;
  capture: CaptureSummary;
  interfaces: NetworkInterface[];
  onCapture: (mode: CaptureMode, selectedInterface: string | null) => void;
  onPalette: () => void;
  onRecord: () => void;
  recording: boolean;
}

export function TopBar({
  primaryView,
  onView,
  query,
  queryError,
  onQuery,
  capture,
  interfaces,
  onCapture,
  onPalette,
  onRecord,
  recording,
}: TopBarProps) {
  const t = useI18n();
  const captureActive =
    capture.mode !== "off" && capture.status.state === "capturing";
  return (
    <header className="topbar">
      <button className="brand" onClick={() => onView("globe")} type="button">
        <img
          alt=""
          aria-hidden="true"
          className="brand__mark"
          height="32"
          src="/sabicheiro-logo.png"
          width="32"
        />
        <span>SABICHEIRO</span>
      </button>
      <nav aria-label={t("Primary")} className="primary-nav">
        {navigation.map((item) => (
          <button
            aria-current={primaryView === item.id ? "page" : undefined}
            className={primaryView === item.id ? "is-active" : undefined}
            key={item.id}
            onClick={() => onView(item.id)}
            type="button"
          >
            <Icon name={item.icon} />
            <span>{t(item.label)}</span>
          </button>
        ))}
      </nav>
      <div className={`query-box${queryError ? " has-error" : ""}`}>
        <Icon name="search" />
        <input
          aria-label={t("Search connections")}
          aria-invalid={queryError ? "true" : undefined}
          onChange={(event) => onQuery(event.target.value)}
          placeholder={t("Search or process:firefox country:US")}
          value={query}
        />
        <kbd>Ctrl F</kbd>
        {queryError && (
          <span className="query-box__error">{t(queryError)}</span>
        )}
      </div>
      <div className="topbar__actions">
        <div className="capture-control">
          <span
            aria-hidden="true"
            className={`status-dot${captureActive ? " is-live" : ""}`}
          />
          <select
            aria-label={t("Capture mode")}
            onChange={(event) => {
              const mode = event.target.value as CaptureMode;
              const selectedIsUsable = interfaces.some(
                (item) =>
                  item.id === capture.selected_interface &&
                  item.is_up &&
                  item.addresses.length > 0,
              );
              void onCapture(
                mode,
                mode === "live" && !selectedIsUsable
                  ? null
                  : capture.selected_interface,
              );
            }}
            value={capture.mode}
          >
            <option value="off">{t("Capture off")}</option>
            <option value="live">{t("Live capture")}</option>
            <option value="demo">{t("Demo data")}</option>
          </select>
        </div>
        {interfaces.length > 0 && capture.mode === "live" && (
          <select
            aria-label={t("Network interface")}
            className="interface-select"
            onChange={(event) =>
              void onCapture("live", event.target.value || null)
            }
            value={capture.selected_interface ?? ""}
          >
            <option value="">{t("All interfaces")}</option>
            {interfaces
              .filter((item) => item.id !== "all")
              .map((item) => (
                <option
                  disabled={!item.is_up || item.addresses.length === 0}
                  key={item.id}
                  value={item.id}
                >
                  {item.name}
                  {item.addresses.length === 0
                    ? t(" (address unavailable)")
                    : ""}
                </option>
              ))}
          </select>
        )}
        <button
          aria-label={
            recording ? t("Recording in progress") : t("Start packet recording")
          }
          className={`icon-button${recording ? " is-recording" : ""}`}
          onClick={onRecord}
          title={recording ? t("Recording in progress") : t("Record packets")}
          type="button"
        >
          <Icon name="record" />
        </button>
        <button
          aria-label={t("Open command palette")}
          className="icon-button"
          onClick={onPalette}
          title={t("Command palette (Ctrl+K)")}
          type="button"
        >
          <Icon name="command" />
        </button>
        <button
          aria-label={t("Open settings")}
          className="icon-button"
          onClick={() => onView("settings")}
          title={t("Settings")}
          type="button"
        >
          <Icon name="settings" />
        </button>
      </div>
    </header>
  );
}

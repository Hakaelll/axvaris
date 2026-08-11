import { useState } from "react";
import { useI18n } from "../i18n";
import type {
  AppSettings,
  DashboardSnapshot,
  FocusRules,
  GeoDatabaseStatus,
  NetworkInterface,
  RetentionPolicy,
  RuleKind,
} from "../types";
import { formatBytes, formatDateTime } from "../lib/format";
import { Icon } from "../components/Icon";

interface SettingsViewProps {
  settings: AppSettings;
  interfaces: NetworkInterface[];
  snapshot: DashboardSnapshot;
  geoStatus: GeoDatabaseStatus | null;
  focusRules: FocusRules;
  onUpdate: (settings: AppSettings) => void;
  onUpdateGeo: () => void;
  onClearSession: () => void;
  onAddBookmark: (kind: Exclude<RuleKind, "localhost">, value: string) => void;
  onRemoveBookmark: (id: number) => void;
  onAddIgnoreRule: (kind: RuleKind, value: string) => void;
  onToggleIgnoreRule: (id: number, enabled: boolean) => void;
  onRemoveIgnoreRule: (id: number) => void;
}

function Toggle({
  checked,
  label,
  detail,
  onChange,
}: {
  checked: boolean;
  label: string;
  detail: string;
  onChange: (value: boolean) => void;
}) {
  return (
    <label className="setting-row setting-row--toggle">
      <span>
        <strong>{label}</strong>
        <small>{detail}</small>
      </span>
      <input
        checked={checked}
        onChange={(event) => onChange(event.target.checked)}
        type="checkbox"
      />
      <i aria-hidden="true" />
    </label>
  );
}

export function SettingsView({
  settings,
  interfaces,
  snapshot,
  geoStatus,
  focusRules,
  onUpdate,
  onUpdateGeo,
  onClearSession,
  onAddBookmark,
  onRemoveBookmark,
  onAddIgnoreRule,
  onToggleIgnoreRule,
  onRemoveIgnoreRule,
}: SettingsViewProps) {
  const t = useI18n();
  const diagnostics = snapshot.diagnostics;
  const [ruleMode, setRuleMode] = useState<"bookmark" | "ignore">("bookmark");
  const [ruleKind, setRuleKind] = useState<RuleKind>("process");
  const [ruleValue, setRuleValue] = useState("");
  const submitRule = () => {
    const value = ruleKind === "localhost" ? "localhost" : ruleValue.trim();
    if (!value) return;
    if (ruleMode === "bookmark" && ruleKind !== "localhost")
      onAddBookmark(ruleKind, value);
    else onAddIgnoreRule(ruleKind, value);
    setRuleValue("");
  };
  return (
    <section className="data-view settings-view">
      <header className="view-heading">
        <div>
          <span className="eyebrow">{t("LOCAL CONTROL")}</span>
          <h1>{t("Settings")}</h1>
          <p>
            {t(
              "Capture, storage and display preferences. Nothing is synced to a cloud service.",
            )}
          </p>
        </div>
      </header>
      <div className="settings-layout">
        <nav aria-label={t("Settings sections")}>
          <a href="#capture">{t("Capture")}</a>
          <a href="#privacy">{t("Privacy & storage")}</a>
          <a href="#focus">{t("Focus rules")}</a>
          <a href="#geo">{t("GeoIP data")}</a>
          <a href="#display">{t("Display")}</a>
          <a href="#diagnostics">{t("Diagnostics")}</a>
        </nav>
        <div className="settings-content">
          <section id="capture">
            <header>
              <Icon name="activity" />
              <div>
                <h2>{t("Capture")}</h2>
                <p>
                  {t(
                    "Native socket observation runs inside the local desktop process.",
                  )}
                </p>
              </div>
            </header>
            <label className="setting-row">
              <span>
                <strong>{t("Default network interface")}</strong>
                <small>
                  {t("All active interfaces is the safest default.")}
                </small>
              </span>
              <select
                value={settings.selected_interface ?? ""}
                onChange={(event) =>
                  onUpdate({
                    ...settings,
                    selected_interface: event.target.value || null,
                  })
                }
              >
                <option value="">{t("All active interfaces")}</option>
                {interfaces
                  .filter((item) => item.id !== "all")
                  .map((item) => (
                    <option
                      disabled={!item.is_up || item.addresses.length === 0}
                      key={item.id}
                      value={item.id}
                    >
                      {item.name}
                      {item.is_vpn ? " · VPN" : ""}
                      {item.addresses.length === 0
                        ? t(" (address unavailable)")
                        : ""}
                    </option>
                  ))}
              </select>
            </label>
            <label className="setting-row">
              <span>
                <strong>{t("Feed grouping window")}</strong>
                <small>
                  {t(
                    "Coalesce the same process, remote IP, port and protocol.",
                  )}
                </small>
              </span>
              <input
                max={300}
                min={1}
                onChange={(event) =>
                  onUpdate({
                    ...settings,
                    feed_group_window_seconds: Number(event.target.value),
                  })
                }
                type="number"
                value={settings.feed_group_window_seconds}
              />
            </label>
            <Toggle
              checked={settings.show_local_traffic}
              detail={t(
                "Include loopback, LAN and private-address destinations in lists.",
              )}
              label={t("Show local traffic")}
              onChange={(value) =>
                onUpdate({ ...settings, show_local_traffic: value })
              }
            />
            <Toggle
              checked={settings.show_listening_sockets}
              detail={t(
                "Include bound TCP/UDP listeners separately from real remote connections.",
              )}
              label={t("Show listening sockets")}
              onChange={(value) =>
                onUpdate({ ...settings, show_listening_sockets: value })
              }
            />
          </section>
          <section id="focus">
            <header>
              <Icon name="search" />
              <div>
                <h2>{t("Focus rules")}</h2>
                <p>
                  {t(
                    "Bookmarks are quick local filters. Ignore rules affect only views and local signals; they never block traffic.",
                  )}
                </p>
              </div>
            </header>
            <div className="rule-builder">
              <select
                aria-label={t("Rule action")}
                onChange={(event) => {
                  const mode = event.target.value as "bookmark" | "ignore";
                  setRuleMode(mode);
                  if (mode === "bookmark" && ruleKind === "localhost")
                    setRuleKind("process");
                }}
                value={ruleMode}
              >
                <option value="bookmark">{t("Bookmark")}</option>
                <option value="ignore">{t("Ignore in views")}</option>
              </select>
              <select
                aria-label={t("Rule kind")}
                onChange={(event) =>
                  setRuleKind(event.target.value as RuleKind)
                }
                value={ruleKind}
              >
                <option value="process">{t("Process")}</option>
                <option value="domain">{t("Domain")}</option>
                <option value="ip">{t("IP address")}</option>
                <option value="country">{t("Country code")}</option>
                {ruleMode === "ignore" && (
                  <option value="localhost">{t("Localhost / LAN")}</option>
                )}
              </select>
              <input
                aria-label={t("Rule value")}
                disabled={ruleKind === "localhost"}
                onChange={(event) => setRuleValue(event.target.value)}
                placeholder={
                  ruleKind === "country"
                    ? "ES"
                    : ruleKind === "ip"
                      ? "1.1.1.1"
                      : t("Value")
                }
                value={ruleValue}
              />
              <button
                className="button button--secondary"
                disabled={ruleKind !== "localhost" && !ruleValue.trim()}
                onClick={submitRule}
                type="button"
              >
                {t("Add locally")}
              </button>
            </div>
            <div className="rule-lists">
              <div>
                <h3>{t("Bookmarks")}</h3>
                {focusRules.bookmarks.map((bookmark) => (
                  <div className="saved-rule" key={bookmark.id}>
                    <span>
                      <small>{bookmark.kind}</small>
                      <strong>{bookmark.value}</strong>
                    </span>
                    <button
                      aria-label={`Remove bookmark ${bookmark.value}`}
                      onClick={() => onRemoveBookmark(bookmark.id)}
                      type="button"
                    >
                      {t("Remove")}
                    </button>
                  </div>
                ))}
                {focusRules.bookmarks.length === 0 && (
                  <p className="list-empty">{t("No bookmarks yet.")}</p>
                )}
              </div>
              <div>
                <h3>{t("Ignored in views")}</h3>
                {focusRules.ignore_rules.map((rule) => (
                  <div
                    className={`saved-rule${rule.enabled ? "" : " is-disabled"}`}
                    key={rule.id}
                  >
                    <label>
                      <input
                        checked={rule.enabled}
                        onChange={(event) =>
                          onToggleIgnoreRule(rule.id, event.target.checked)
                        }
                        type="checkbox"
                      />
                      <span>
                        <small>{rule.kind}</small>
                        <strong>{rule.value}</strong>
                      </span>
                    </label>
                    <button
                      aria-label={`Remove ignore rule ${rule.value}`}
                      onClick={() => onRemoveIgnoreRule(rule.id)}
                      type="button"
                    >
                      {t("Remove")}
                    </button>
                  </div>
                ))}
                {focusRules.ignore_rules.length === 0 && (
                  <p className="list-empty">
                    {t("No traffic is explicitly ignored.")}
                  </p>
                )}
              </div>
            </div>
          </section>
          <section id="privacy">
            <header>
              <Icon name="shield" />
              <div>
                <h2>{t("Privacy & storage")}</h2>
                <p>
                  {t(
                    "Metadata is stored in SQLite on this device. Payloads are never stored during normal capture.",
                  )}
                </p>
              </div>
            </header>
            <label className="setting-row">
              <span>
                <strong>{t("History retention")}</strong>
                <small>
                  {t(
                    "Expired sessions are removed locally and transactionally.",
                  )}
                </small>
              </span>
              <select
                value={settings.retention}
                onChange={(event) =>
                  onUpdate({
                    ...settings,
                    retention: event.target.value as RetentionPolicy,
                  })
                }
              >
                <option value="off">{t("Do not retain history")}</option>
                <option value="one_day">{t("1 day")}</option>
                <option value="seven_days">{t("7 days")}</option>
                <option value="thirty_days">{t("30 days")}</option>
                <option value="ninety_days">{t("90 days")}</option>
                <option value="unlimited">{t("Unlimited")}</option>
              </select>
            </label>
            <Toggle
              checked={settings.reverse_dns_enabled}
              detail={t(
                "Opt in to reverse lookups through your configured system DNS resolver; remote IPs may be visible to that resolver.",
              )}
              label={t("System reverse DNS lookups")}
              onChange={(value) =>
                onUpdate({ ...settings, reverse_dns_enabled: value })
              }
            />
            <div className="setting-row">
              <span>
                <strong>{t("Current session")}</strong>
                <small>
                  {t(
                    "Clearing removes the active in-memory view; retention still governs saved history.",
                  )}
                </small>
              </span>
              <button
                className="button button--danger"
                onClick={onClearSession}
                type="button"
              >
                {t("Clear session")}
              </button>
            </div>
            <div className="privacy-note">
              <strong>{t("Local by default")}</strong>
              <p>
                {t(
                  "Sabicheiro has no analytics SDK, account, subscription, remote logging or paid API. Reverse DNS is opt-in; GeoIP lookup uses an optional local free database.",
                )}
              </p>
            </div>
          </section>
          <section id="geo">
            <header>
              <Icon name="database" />
              <div>
                <h2>{t("GeoIP data")}</h2>
                <p>
                  {t(
                    "Optional local DB-IP Lite data. Location is approximate and never authoritative.",
                  )}
                </p>
              </div>
            </header>
            <div className="setting-row">
              <span>
                <strong>{geoStatus?.dataset ?? t("GeoIP database")}</strong>
                <small>
                  {geoStatus?.installed
                    ? t("Version {version} · updated {date}", {
                        version: geoStatus.version ?? t("unknown"),
                        date: formatDateTime(geoStatus.last_update_ms),
                      })
                    : t("Not installed; public IPs remain unlocated.")}
                </small>
              </span>
              <button
                className="button button--secondary"
                onClick={onUpdateGeo}
                type="button"
              >
                {geoStatus?.installed
                  ? t("Check free update")
                  : t("Install free database")}
              </button>
            </div>
            <p className="attribution">
              {geoStatus?.attribution ??
                "IP geolocation by DB-IP.com, licensed under CC BY 4.0."}
            </p>
          </section>
          <section id="display">
            <header>
              <Icon name="globe" />
              <div>
                <h2>{t("Display")}</h2>
                <p>{t("Visual preferences never change captured evidence.")}</p>
              </div>
            </header>
            <label className="setting-row">
              <span>
                <strong>{t("Interface language")}</strong>
                <small>
                  {t("Changes navigation, controls, views and help text.")}
                </small>
              </span>
              <select
                aria-label={t("Interface language")}
                onChange={(event) =>
                  onUpdate({ ...settings, ui_language: event.target.value })
                }
                value={settings.ui_language.startsWith("es") ? "es" : "en"}
              >
                <option value="en">English</option>
                <option value="es">Castellano</option>
              </select>
            </label>
            <Toggle
              checked={settings.reduce_motion}
              detail={t("Disables camera focus and decorative transitions.")}
              label={t("Reduce motion")}
              onChange={(value) =>
                onUpdate({ ...settings, reduce_motion: value })
              }
            />
            <Toggle
              checked={settings.auto_focus}
              detail={t(
                "Briefly moves the globe toward a selected destination.",
              )}
              label={t("Smart destination focus")}
              onChange={(value) => onUpdate({ ...settings, auto_focus: value })}
            />
          </section>
          <section id="diagnostics">
            <header>
              <Icon name="terminal" />
              <div>
                <h2>{t("Diagnostics")}</h2>
                <p>{t("Local runtime counters; none are transmitted.")}</p>
              </div>
            </header>
            <div className="diagnostics-grid">
              <div>
                <small>{t("BACKEND")}</small>
                <strong>{diagnostics.backend}</strong>
              </div>
              <div>
                <small>CPU</small>
                <strong>{diagnostics.cpu_percent.toFixed(1)}%</strong>
              </div>
              <div>
                <small>{t("MEMORY")}</small>
                <strong>{formatBytes(diagnostics.memory_bytes)}</strong>
              </div>
              <div>
                <small>{t("RENDER")}</small>
                <strong>{Math.round(diagnostics.render_fps)} fps</strong>
              </div>
              <div>
                <small>{t("CAPTURE RATE")}</small>
                <strong>
                  {diagnostics.capture_events_per_second.toFixed(1)}/s
                </strong>
              </div>
              <div>
                <small>{t("DATABASE")}</small>
                <strong>{formatBytes(diagnostics.database_size_bytes)}</strong>
              </div>
              <div>
                <small>{t("QUEUE")}</small>
                <strong>
                  {diagnostics.capture_queue_depth}/
                  {diagnostics.capture_queue_capacity}
                </strong>
              </div>
              <div>
                <small>{t("DROPPED")}</small>
                <strong>
                  {diagnostics.dropped_capture_events +
                    diagnostics.dropped_ui_events}
                </strong>
              </div>
            </div>
          </section>
        </div>
      </div>
    </section>
  );
}

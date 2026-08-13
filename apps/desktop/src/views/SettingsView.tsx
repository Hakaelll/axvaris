import { useState } from "react";
import { useI18n } from "../i18n";
import type {
  AppSettings,
  CaptureSelfTest,
  DashboardSnapshot,
  ExportResult,
  FocusRules,
  GeoDatabaseStatus,
  NetworkInterface,
  NotificationRuleKind,
  LocalDataCategory,
  LocalDataRemovalResult,
  RetentionPolicy,
  RuleKind,
  SupportBundleRequest,
  StorageHealth,
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
  onImportGeo: (path: string, kind: "city" | "asn") => Promise<void>;
  onRollbackGeo: (kind: "city" | "asn") => Promise<GeoDatabaseStatus | null>;
  onRemoveGeo: (kind: "city" | "asn") => Promise<GeoDatabaseStatus | null>;
  onCompactStorage: () => Promise<StorageHealth | null>;
  onClearLocalData: (
    category: LocalDataCategory,
  ) => Promise<LocalDataRemovalResult | null>;
  onRunSelfTest: () => Promise<CaptureSelfTest | null>;
  onExportSupportBundle: (
    request: SupportBundleRequest,
  ) => Promise<ExportResult | null>;
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
  onImportGeo,
  onRollbackGeo,
  onRemoveGeo,
  onCompactStorage,
  onClearLocalData,
  onRunSelfTest,
  onExportSupportBundle,
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
  const [notificationKind, setNotificationKind] =
    useState<NotificationRuleKind>("new_process");
  const [notificationValue, setNotificationValue] = useState("");
  const [geoImportPath, setGeoImportPath] = useState("");
  const [geoImportKind, setGeoImportKind] = useState<"city" | "asn">("city");
  const [selfTest, setSelfTest] = useState<CaptureSelfTest | null>(null);
  const [selfTestRunning, setSelfTestRunning] = useState(false);
  const [bundlePath, setBundlePath] = useState("sabicheiro-support.zip");
  const [bundleIncludeSettings, setBundleIncludeSettings] = useState(true);
  const [bundleIncludeLogs, setBundleIncludeLogs] = useState(true);
  const [bundleResult, setBundleResult] = useState<ExportResult | null>(null);
  const [storageHealth, setStorageHealth] = useState<StorageHealth | null>(null);
  const [localDataCategory, setLocalDataCategory] =
    useState<LocalDataCategory>("sessions");
  const [localDataResult, setLocalDataResult] =
    useState<LocalDataRemovalResult | null>(null);
  const submitRule = () => {
    const value = ruleKind === "localhost" ? "localhost" : ruleValue.trim();
    if (!value) return;
    if (ruleMode === "bookmark" && ruleKind !== "localhost")
      onAddBookmark(ruleKind, value);
    else onAddIgnoreRule(ruleKind, value);
    setRuleValue("");
  };
  const addNotificationRule = () => {
    const needsValue =
      notificationKind === "port" || notificationKind === "asn";
    const value = notificationValue.trim();
    if (needsValue && !value) return;
    onUpdate({
      ...settings,
      notification_rules: [
        ...settings.notification_rules,
        {
          id: `notification-${Date.now()}-${Math.random().toString(16).slice(2)}`,
          kind: notificationKind,
          value: needsValue ? value : null,
          enabled: true,
          cooldown_seconds: 60,
        },
      ],
    });
    setNotificationValue("");
    if (
      typeof Notification !== "undefined" &&
      Notification.permission === "default"
    ) {
      void Notification.requestPermission();
    }
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
          <a href="#notifications">{t("Notifications")}</a>
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
            <div className="privacy-note">
              <strong>{t("Separate connection workspaces")}</strong>
              <p>
                {t(
                  "Remote peers, local traffic and listeners are always kept in separate tabs so their counts and meaning are never mixed.",
                )}
              </p>
            </div>
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
          <section id="notifications">
            <header>
              <Icon name="activity" />
              <div>
                <h2>{t("Local notifications")}</h2>
                <p>
                  {t(
                    "Opt-in, explainable rules are evaluated locally and rate-limited. They never block traffic.",
                  )}
                </p>
              </div>
            </header>
            <div className="rule-builder">
              <select
                aria-label={t("Notification rule")}
                onChange={(event) =>
                  setNotificationKind(
                    event.target.value as NotificationRuleKind,
                  )
                }
                value={notificationKind}
              >
                <option value="new_process">{t("New network process")}</option>
                <option value="new_country">
                  {t("New destination country")}
                </option>
                <option value="asn">{t("ASN number or organization")}</option>
                <option value="port">{t("Remote port")}</option>
              </select>
              <input
                aria-label={t("Notification value")}
                disabled={
                  notificationKind === "new_process" ||
                  notificationKind === "new_country"
                }
                inputMode={notificationKind === "port" ? "numeric" : "text"}
                onChange={(event) => setNotificationValue(event.target.value)}
                placeholder={notificationKind === "port" ? "443" : "15169"}
                value={notificationValue}
              />
              <button
                className="button button--secondary"
                disabled={
                  (notificationKind === "port" || notificationKind === "asn") &&
                  !notificationValue.trim()
                }
                onClick={addNotificationRule}
                type="button"
              >
                {t("Add notification")}
              </button>
            </div>
            <div className="rule-lists rule-lists--single">
              <div>
                {settings.notification_rules.map((rule) => (
                  <div
                    className={`saved-rule${rule.enabled ? "" : " is-disabled"}`}
                    key={rule.id}
                  >
                    <label>
                      <input
                        checked={rule.enabled}
                        onChange={(event) =>
                          onUpdate({
                            ...settings,
                            notification_rules: settings.notification_rules.map(
                              (item) =>
                                item.id === rule.id
                                  ? { ...item, enabled: event.target.checked }
                                  : item,
                            ),
                          })
                        }
                        type="checkbox"
                      />
                      <span>
                        <small>{t(rule.kind)}</small>
                        <strong>
                          {rule.value ?? t("Any new observation")} /{" "}
                          {rule.cooldown_seconds}s
                        </strong>
                      </span>
                    </label>
                    <button
                      aria-label={t("Remove notification rule")}
                      onClick={() =>
                        onUpdate({
                          ...settings,
                          notification_rules:
                            settings.notification_rules.filter(
                              (item) => item.id !== rule.id,
                            ),
                        })
                      }
                      type="button"
                    >
                      {t("Remove")}
                    </button>
                  </div>
                ))}
                {settings.notification_rules.length === 0 && (
                  <p className="list-empty">
                    {t("No notification rules yet.")}
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
              checked={settings.ephemeral_capture}
              detail={t(
                "New live sessions stay in memory only: no connection history, baselines or session metadata are written to SQLite. This takes effect on the next capture.",
              )}
              label={t("Ephemeral capture")}
              onChange={(value) =>
                onUpdate({ ...settings, ephemeral_capture: value })
              }
            />
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
            <div className="local-data-panel">
              <div>
                <strong>{t("Local data controls")}</strong>
                <p>
                  {t(
                    "Delete one local category at a time. Capture must be stopped before changing retained sessions or database files.",
                  )}
                </p>
              </div>
              <div className="local-data-panel__controls">
                <select
                  aria-label={t("Local data category")}
                  onChange={(event) =>
                    setLocalDataCategory(
                      event.target.value as LocalDataCategory,
                    )
                  }
                  value={localDataCategory}
                >
                  <option value="sessions">{t("Retained sessions")}</option>
                  <option value="geo_ip">{t("GeoIP databases")}</option>
                  <option value="dns_cache">{t("DNS cache")}</option>
                  <option value="logs">{t("Application logs")}</option>
                  <option value="all">{t("All local data and settings")}</option>
                </select>
                <button
                  className="button button--danger"
                  onClick={() => {
                    const confirmed = window.confirm(
                      t(
                        "Permanently delete the selected local data? This cannot be undone inside Sabicheiro.",
                      ),
                    );
                    if (!confirmed) return;
                    void onClearLocalData(localDataCategory).then(
                      setLocalDataResult,
                    );
                  }}
                  type="button"
                >
                  {t("Delete selected data")}
                </button>
                <button
                  className="button button--secondary"
                  onClick={() => void onCompactStorage().then(setStorageHealth)}
                  type="button"
                >
                  {t("Check and compact SQLite")}
                </button>
              </div>
              {localDataResult && (
                <small role="status">
                  {t("Removed {count} local items from {category}.", {
                    count: localDataResult.removed_items,
                    category: t(localDataResult.category),
                  })}
                </small>
              )}
              {storageHealth && (
                <div
                  className={`storage-health ${
                    storageHealth.quick_check_ok ? "is-pass" : "is-fail"
                  }`}
                  role="status"
                >
                  <strong>
                    {storageHealth.quick_check_ok
                      ? t("SQLite integrity check passed")
                      : t("SQLite integrity check failed")}
                  </strong>
                  <span>{storageHealth.detail}</span>
                  <small>
                    {t("Database {size} · WAL {log}/{checkpointed} pages", {
                      size: formatBytes(storageHealth.database_size_bytes),
                      log: storageHealth.wal_log_pages,
                      checkpointed: storageHealth.wal_checkpointed_pages,
                    })}
                  </small>
                </div>
              )}
            </div>
            <div className="support-bundle-panel">
              <div>
                <strong>{t("Diagnostic support bundle")}</strong>
                <p>
                  {t(
                    "Review and export versions, redacted settings and local logs. Connection history, raw IPs, payloads, recordings and secrets are excluded.",
                  )}
                </p>
              </div>
              <div className="support-bundle-panel__controls">
                <input
                  aria-label={t("Support bundle filename")}
                  onChange={(event) => setBundlePath(event.target.value)}
                  value={bundlePath}
                />
                <label>
                  <input
                    checked={bundleIncludeSettings}
                    onChange={(event) =>
                      setBundleIncludeSettings(event.target.checked)
                    }
                    type="checkbox"
                  />
                  {t("Redacted settings")}
                </label>
                <label>
                  <input
                    checked={bundleIncludeLogs}
                    onChange={(event) =>
                      setBundleIncludeLogs(event.target.checked)
                    }
                    type="checkbox"
                  />
                  {t("Local application logs")}
                </label>
                <button
                  className="button button--secondary"
                  disabled={!bundlePath.trim()}
                  onClick={() =>
                    void onExportSupportBundle({
                      path: bundlePath.trim(),
                      include_settings: bundleIncludeSettings,
                      include_logs: bundleIncludeLogs,
                    }).then(setBundleResult)
                  }
                  type="button"
                >
                  {t("Export reviewed ZIP")}
                </button>
              </div>
              {bundleResult && (
                <small>
                  {t("Saved {count} redacted files to {path}", {
                    count: bundleResult.records,
                    path: bundleResult.path,
                  })}
                </small>
              )}
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
            <div className="geo-actions">
              <button
                className="button button--secondary"
                onClick={onUpdateGeo}
                type="button"
              >
                {geoStatus?.city.installed
                  ? t("Check free City update")
                  : t("Install free City database")}
              </button>
              <small>
                {t(
                  "Downloads happen only after this action and fetch the complete free dataset over HTTPS. Observed IPs are never sent.",
                )}
              </small>
            </div>
            <div className="geo-dataset-grid">
              {(["city", "asn"] as const).map((kind) => {
                const dataset = geoStatus?.[kind];
                return (
                  <article className="geo-dataset-card" key={kind}>
                    <header>
                      <div>
                        <small>{kind === "city" ? "CITY" : "ASN"}</small>
                        <strong>
                          {dataset?.installed
                            ? dataset.dataset
                            : t("Not installed")}
                        </strong>
                      </div>
                      <span
                        className={`status-dot ${
                          dataset?.installed ? "is-active" : ""
                        }`}
                      >
                        {dataset?.installed ? t("Ready") : t("Offline")}
                      </span>
                    </header>
                    <dl>
                      <div>
                        <dt>{t("Edition")}</dt>
                        <dd>{dataset?.edition ?? t("Unavailable")}</dd>
                      </div>
                      <div>
                        <dt>{t("Version")}</dt>
                        <dd>{dataset?.version ?? t("Unavailable")}</dd>
                      </div>
                      <div>
                        <dt>{t("Built")}</dt>
                        <dd>{formatDateTime(dataset?.build_epoch_ms ?? null)}</dd>
                      </div>
                      <div>
                        <dt>{t("Updated")}</dt>
                        <dd>{formatDateTime(dataset?.last_update_ms ?? null)}</dd>
                      </div>
                      <div>
                        <dt>{t("Generation")}</dt>
                        <dd>{dataset?.generation ?? 0}</dd>
                      </div>
                      <div>
                        <dt>SHA-256</dt>
                        <dd>
                          <code title={dataset?.sha256 ?? undefined}>
                            {dataset?.sha256
                              ? `${dataset.sha256.slice(0, 12)}…`
                              : t("Unavailable")}
                          </code>
                        </dd>
                      </div>
                    </dl>
                    <div className="geo-dataset-card__actions">
                      <button
                        className="button button--secondary"
                        disabled={!dataset?.backup_available}
                        onClick={() => void onRollbackGeo(kind)}
                        type="button"
                      >
                        {t("Rollback")}
                      </button>
                      <button
                        className="button button--danger"
                        disabled={!dataset?.installed}
                        onClick={() => {
                          if (
                            window.confirm(
                              t("Remove this local MMDB and its backup?"),
                            )
                          )
                            void onRemoveGeo(kind);
                        }}
                        type="button"
                      >
                        {t("Remove database")}
                      </button>
                    </div>
                    <small>
                      {t(dataset?.attribution ?? geoStatus?.attribution ?? "Unavailable")}
                    </small>
                  </article>
                );
              })}
            </div>
            {geoStatus?.last_operation && (
              <p className="geo-last-operation" role="status">
                {t("Last operation")}: {t(geoStatus.last_operation)}
              </p>
            )}
            <div className="geo-import-panel">
              <div>
                <strong>{t("Offline MMDB import")}</strong>
                <p>
                  {t(
                    "Import a free City or ASN database from this computer. Edition and checksum are verified before the local atomic replacement.",
                  )}
                </p>
              </div>
              <select
                aria-label={t("MMDB kind")}
                onChange={(event) =>
                  setGeoImportKind(event.target.value as "city" | "asn")
                }
                value={geoImportKind}
              >
                <option value="city">City MMDB</option>
                <option value="asn">ASN MMDB</option>
              </select>
              <input
                aria-label={t("MMDB file path")}
                onChange={(event) => setGeoImportPath(event.target.value)}
                placeholder="C:\\Data\\dbip-city-lite.mmdb"
                value={geoImportPath}
              />
              <button
                className="button button--secondary"
                disabled={!geoImportPath.trim()}
                onClick={() =>
                  void onImportGeo(geoImportPath.trim(), geoImportKind)
                }
                type="button"
              >
                {t("Verify and import")}
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
            <label className="setting-row">
              <span>
                <strong>{t("Interface density")}</strong>
                <small>
                  {t(
                    "Compact fits more evidence on screen; Comfortable adds breathing room without hiding fields.",
                  )}
                </small>
              </span>
              <select
                aria-label={t("Interface density")}
                onChange={(event) =>
                  onUpdate({
                    ...settings,
                    ui_density: event.target.value as AppSettings["ui_density"],
                  })
                }
                value={settings.ui_density}
              >
                <option value="comfortable">{t("Comfortable")}</option>
                <option value="compact">{t("Compact")}</option>
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
              <div>
                <small>{t("EVENT SOURCE")}</small>
                <strong>{diagnostics.capture_event_source}</strong>
              </div>
              <div>
                <small>{t("FLOW COUNTERS")}</small>
                <strong>
                  {diagnostics.flow_counters_available
                    ? t("Available")
                    : t("Unavailable")}
                </strong>
              </div>
              <div>
                <small>{t("PACKET EVENTS")}</small>
                <strong>{diagnostics.packet_events_observed}</strong>
              </div>
              <div>
                <small>{t("DNS CORRELATIONS")}</small>
                <strong>{diagnostics.dns_correlations_observed}</strong>
              </div>
              <div>
                <small>{t("CORRELATED")}</small>
                <strong>{diagnostics.correlated_capture_events}</strong>
              </div>
              <div>
                <small>{t("APPROXIMATE")}</small>
                <strong>{diagnostics.approximate_capture_events}</strong>
              </div>
              <div>
                <small>{t("ORPHANED")}</small>
                <strong>{diagnostics.orphan_capture_events}</strong>
              </div>
              <div>
                <small>{t("IPC GAPS / RESYNCS")}</small>
                <strong>
                  {diagnostics.sequence_gaps} / {diagnostics.snapshot_resyncs}
                </strong>
              </div>
            </div>
            <div className="subsystem-grid">
              {diagnostics.subsystems.map((subsystem) => (
                <article
                  className={`subsystem-card is-${subsystem.status}`}
                  key={subsystem.subsystem}
                >
                  <header>
                    <strong>{t(subsystem.subsystem)}</strong>
                    <span>{t(subsystem.status)}</span>
                  </header>
                  <p>{t(subsystem.message)}</p>
                  {subsystem.action && <small>{t(subsystem.action)}</small>}
                  <code>
                    {subsystem.queue_depth}/{subsystem.queue_capacity} · {t("dropped")} {subsystem.dropped_events}
                  </code>
                </article>
              ))}
            </div>
            <div className="self-test-panel">
              <div>
                <strong>{t("Capture self-test")}</strong>
                <p>
                  {t(
                    "Checks adapters, socket tables, process attribution, DNS, GeoIP and storage without requiring a terminal.",
                  )}
                </p>
              </div>
              <button
                className="button button--secondary"
                disabled={selfTestRunning}
                onClick={() => {
                  setSelfTestRunning(true);
                  void onRunSelfTest().then((result) => {
                    setSelfTest(result);
                    setSelfTestRunning(false);
                  });
                }}
                type="button"
              >
                {selfTestRunning ? t("Running checks...") : t("Run self-test")}
              </button>
              {selfTest && (
                <div className="self-test-results">
                  {selfTest.checks.map((check) => (
                    <article className={`is-${check.level}`} key={check.id}>
                      <span>
                        {check.level === "pass"
                          ? "✓"
                          : check.level === "warn"
                            ? "!"
                            : "×"}
                      </span>
                      <div>
                        <strong>{t(check.title)}</strong>
                        <p>{t(check.detail)}</p>
                        {check.diagnostic_code && (
                          <code>{check.diagnostic_code}</code>
                        )}
                        {check.action && <small>{t(check.action)}</small>}
                      </div>
                    </article>
                  ))}
                  <div className="capture-coverage" role="table">
                    <strong>{t("Capability coverage")}</strong>
                    {selfTest.coverage.map((capability) => (
                      <div
                        key={`${capability.protocol}-${capability.family}`}
                        role="row"
                      >
                        <span>
                          {capability.protocol.toUpperCase()} · {capability.family}
                        </span>
                        <span>{t("endpoint")}: {t(capability.endpoint)}</span>
                        <span>{t("process")}: {t(capability.process)}</span>
                        <span>{t("interface")}: {t(capability.interface)}</span>
                        <span>{t("close")}: {t(capability.close_event)}</span>
                        {capability.limitation && (
                          <small>{t(capability.limitation)}</small>
                        )}
                      </div>
                    ))}
                  </div>
                  <details className="redacted-report">
                    <summary>{t("Redacted self-test report")}</summary>
                    <pre>{selfTest.redacted_report}</pre>
                    <button
                      className="button button--secondary"
                      onClick={() =>
                        void navigator.clipboard.writeText(
                          selfTest.redacted_report,
                        )
                      }
                      type="button"
                    >
                      {t("Copy redacted report")}
                    </button>
                  </details>
                </div>
              )}
            </div>
          </section>
        </div>
      </div>
    </section>
  );
}

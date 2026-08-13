import {
  lazy,
  Suspense,
  useCallback,
  useEffect,
  useMemo,
  useRef,
  useState,
} from "react";
import { desktopApi } from "./lib/backend";
import { useSabicheiro } from "./hooks/useSabicheiro";
import { I18nProvider, translate, type Language } from "./i18n";
import { TopBar } from "./components/TopBar";
import { MetricStrip } from "./components/MetricStrip";
import { LiveFeed } from "./components/LiveFeed";
import { ConnectionInspector } from "./components/ConnectionInspector";
import { FirstRun } from "./components/FirstRun";
import { CommandPalette } from "./components/CommandPalette";
import {
  ErrorBanner,
  ExportDialog,
  RecordingDialog,
} from "./components/Dialogs";
import { Icon } from "./components/Icon";
import { FocusBar } from "./components/FocusBar";
import { TopContext } from "./components/TopContext";
import { ProcessView } from "./views/ProcessView";
import { DestinationView } from "./views/DestinationView";
import { HistoryView } from "./views/HistoryView";
import { SettingsView } from "./views/SettingsView";
import { WorkspaceTabs } from "./components/WorkspaceTabs";
import { NotificationCenter } from "./components/NotificationCenter";

const GlobeScene = lazy(async () => {
  const module = await import("./components/visualization/GlobeScene");
  return { default: module.GlobeScene };
});

const MapView = lazy(async () => {
  const module = await import("./components/visualization/MapView");
  return { default: module.MapView };
});

function isEditable(target: EventTarget | null): boolean {
  return (
    target instanceof HTMLInputElement ||
    target instanceof HTMLTextAreaElement ||
    target instanceof HTMLSelectElement ||
    (target instanceof HTMLElement && target.isContentEditable)
  );
}

const localizedFailureMessages: Record<string, string> = {
  capture_unavailable:
    "Network capture is unavailable. Try All interfaces or run the capture self-test.",
  interface_unavailable:
    "The selected network interface is unavailable. Choose All interfaces and try again.",
  history_unavailable:
    "Local history could not be read. Check Diagnostics for the original storage detail.",
  timeline_unavailable:
    "Stored traffic samples could not be read for this time range.",
  storage_unavailable:
    "Local storage is unavailable. Sabicheiro has kept the original diagnostic below.",
  geo_update_failed:
    "The free GeoIP update could not be installed. You can retry or import an MMDB offline.",
  geo_import_failed:
    "The MMDB import failed edition, checksum or filesystem validation.",
  geo_status_unavailable:
    "The status of the local GeoIP datasets could not be read.",
  geo_rollback_failed:
    "The previous local GeoIP dataset could not be restored; the active dataset was kept.",
  geo_remove_failed:
    "The selected local GeoIP dataset could not be removed safely.",
  storage_maintenance_failed:
    "SQLite maintenance could not complete; the database was left intact.",
  storage_busy: "Stop capture before running SQLite maintenance.",
  capture_must_stop:
    "Stop capture before deleting retained sessions or all local data.",
  local_data_remove_failed:
    "The selected local data could not be removed completely.",
  investigation_invalid:
    "The local investigation is missing a valid name or query.",
  investigation_unavailable:
    "Saved investigations could not be read from local storage.",
  asn_profile_invalid: "Choose a valid ASN number greater than zero.",
  asn_profile_unavailable:
    "The local ASN profile could not be reconstructed from retained evidence.",
  timeline_not_found:
    "The selected saved connection is no longer available for its identity timeline.",
  reenrichment_invalid: "Select one retained session to re-enrich.",
  reenrichment_active:
    "This session already has an active re-enrichment job.",
  reenrichment_not_found: "The local re-enrichment job is no longer available.",
  reenrichment_unavailable:
    "The derived GeoIP view could not be loaded; original observations were unchanged.",
  comparison_invalid: "Choose two different retained sessions.",
  comparison_export_failed:
    "The local session comparison could not be exported.",
  recording_failed:
    "Packet recording could not continue; review the original local diagnostic.",
  focus_rules_unavailable: "Local focus rules could not be read.",
  focus_rule_invalid: "The local focus rule is invalid.",
  history_query_invalid: "The local history query is invalid.",
  export_failed:
    "The local export could not be written. Choose a new filename or writable folder.",
  support_bundle_failed: "The redacted support bundle could not be created.",
  comparison_unavailable:
    "The selected sessions could not be compared from local history.",
  comparison_empty: "Both selected sessions need retained connections.",
  output_path_exists: "That output already exists. Choose a new filename.",
  output_path_invalid: "The output filename or extension is invalid.",
  invalid_settings: "One of the local settings is outside its allowed range.",
  operation_failed: "The operation could not be completed.",
};

export default function App() {
  const app = useSabicheiro();
  const language: Language = app.settings.ui_language.startsWith("es")
    ? "es"
    : "en";
  const t = useCallback(
    (source: string, values?: Record<string, string | number>) =>
      translate(language, source, values),
    [language],
  );
  const [exportScope, setExportScope] = useState<{
    sessionId: string | null;
    fromMs: number | null;
    toMs: number | null;
  }>({ sessionId: null, fromMs: null, toMs: null });
  const [mapExpanded, setMapExpanded] = useState(false);
  const mainStageRef = useRef<HTMLElement>(null);
  const home = useMemo<[number, number]>(
    () => [app.settings.home_latitude, app.settings.home_longitude],
    [app.settings.home_latitude, app.settings.home_longitude],
  );
  const recordingActive =
    app.recording?.status === "recording" ||
    app.recording?.status === "starting" ||
    app.recording?.status === "stopping" ||
    (app.recording?.status === "error" && Boolean(app.recording.output_path));
  const webglSupported = useMemo(() => {
    try {
      const canvas = document.createElement("canvas");
      return Boolean(canvas.getContext("webgl2") || canvas.getContext("webgl"));
    } catch {
      return false;
    }
  }, []);

  useEffect(() => {
    document.documentElement.lang = language;
  }, [language]);

  useEffect(() => {
    if (mainStageRef.current) {
      mainStageRef.current.scrollTop = 0;
      mainStageRef.current.scrollLeft = 0;
    }
  }, [app.primaryView]);

  const showFilter = useCallback(
    (query: string) => {
      app.setQuery(query);
      app.setPrimaryView("globe");
    },
    [app.setPrimaryView, app.setQuery],
  );

  const openExport = useCallback(
    (
      sessionId: string | null,
      fromMs: number | null = null,
      toMs: number | null = null,
    ) => {
      setExportScope({ sessionId, fromMs, toMs });
      app.setExportDialogOpen(true);
    },
    [app.setExportDialogOpen],
  );

  const searchPreset = useCallback(
    (query: string) => {
      app.setQuery(query);
      app.setPrimaryView("globe");
      requestAnimationFrame(() => {
        const input =
          document.querySelector<HTMLInputElement>(".query-box input");
        input?.focus();
        input?.setSelectionRange(query.length, query.length);
      });
    },
    [app.setPrimaryView, app.setQuery],
  );

  useEffect(() => {
    const handleKey = (event: KeyboardEvent) => {
      const editable = isEditable(event.target);
      if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === "k") {
        event.preventDefault();
        app.setCommandPaletteOpen(true);
      } else if (
        (event.ctrlKey || event.metaKey) &&
        event.key.toLowerCase() === "f"
      ) {
        event.preventDefault();
        document.querySelector<HTMLInputElement>(".query-box input")?.focus();
      } else if (event.key === "Escape") {
        setMapExpanded(false);
        app.setSelectedId(null);
        app.setCommandPaletteOpen(false);
        app.setRecordDialogOpen(false);
        app.setExportDialogOpen(false);
      } else if (
        !editable &&
        !event.ctrlKey &&
        !event.metaKey &&
        !event.altKey
      ) {
        if (event.code === "Space") {
          event.preventDefault();
          void app.setPaused(!app.paused);
        }
        if (event.key.toLowerCase() === "g") {
          app.setPrimaryView("globe");
          app.setViewMode("globe");
        }
        if (event.key.toLowerCase() === "m") {
          app.setPrimaryView("globe");
          app.setViewMode("map");
        }
        if (event.key.toLowerCase() === "r") {
          if (recordingActive) void app.stopRecording();
          else app.setRecordDialogOpen(true);
        }
      }
    };
    window.addEventListener("keydown", handleKey);
    return () => window.removeEventListener("keydown", handleKey);
  }, [app]);

  const reportFps = useCallback((fps: number) => {
    void desktopApi.reportUiDiagnostics(fps, 0);
  }, []);
  const captureError =
    app.snapshot.capture.status.state === "error"
      ? app.snapshot.capture.status.message
      : null;
  const localizedFailureMessage = captureError
    ? "The capture backend stopped. Run the self-test or select a different interface."
    : (localizedFailureMessages[app.error?.code ?? "operation_failed"] ??
      "The operation could not be completed.");
  const originalFailureDetail = app.error?.message ?? captureError ?? undefined;

  return (
    <I18nProvider value={language}>
      <div
        className={`app-shell density--${app.settings.ui_density}${app.settings.reduce_motion ? " reduce-motion" : ""}`}
      >
        <TopBar
          capture={app.snapshot.capture}
          interfaces={app.interfaces}
          onCapture={app.startCapture}
          onPalette={() => app.setCommandPaletteOpen(true)}
          onQuery={app.setQuery}
          onRecord={() => app.setRecordDialogOpen(true)}
          onView={app.setPrimaryView}
          primaryView={app.primaryView}
          query={app.query}
          queryError={app.queryError}
          recording={recordingActive}
        />
        <main
          className={`main-stage main-stage--${app.primaryView}`}
          ref={mainStageRef}
        >
          {app.primaryView === "globe" && (
            <div className={`observatory${mapExpanded ? " is-map-expanded" : ""}`}>
              <section className="visual-canvas">
                <div className="visual-canvas__heading">
                  <div>
                    <span className="eyebrow">{t("GLOBAL ROUTES")}</span>
                    <strong>
                      {t("{count} active", {
                        count: app.snapshot.stats.active_connections,
                      })}
                    </strong>
                    <small>
                      {" / "}
                      {t("{count} countries", {
                        count: app.snapshot.stats.countries,
                      })}
                    </small>
                  </div>
                  <WorkspaceTabs
                    active={app.settings.active_workspace}
                    counts={app.workspaceCounts}
                    onChange={(active_workspace) =>
                      void app.updateSettings({
                        ...app.settings,
                        active_workspace,
                      })
                    }
                  />
                  <div
                    className="view-switcher"
                    aria-label={t("Visualization mode")}
                  >
                    <button
                      aria-pressed={app.viewMode === "globe"}
                      disabled={!webglSupported}
                      onClick={() => app.setViewMode("globe")}
                      title={
                        webglSupported
                          ? t("3D globe (G)")
                          : t("WebGL unavailable")
                      }
                      type="button"
                    >
                      <Icon name="globe" />
                      3D
                    </button>
                    <button
                      aria-pressed={app.viewMode === "map" || !webglSupported}
                      onClick={() => app.setViewMode("map")}
                      title={t("2D map (M)")}
                      type="button"
                    >
                      <Icon name="map" />
                      2D
                    </button>
                  </div>
                </div>
                <div className="visual-canvas__scene">
                  {app.viewMode === "globe" && webglSupported ? (
                    <Suspense
                      fallback={
                        <div className="scene-loading">
                          <span />
                          {t("Loading local globe...")}
                        </div>
                      }
                    >
                      <GlobeScene
                        connections={app.snapshot.connections}
                        home={home}
                        onFps={reportFps}
                        onSelect={app.setSelectedId}
                        reduceMotion={app.settings.reduce_motion}
                        selectedId={app.selectedId}
                        smartFocus={app.settings.auto_focus}
                      />
                    </Suspense>
                  ) : (
                    <Suspense
                      fallback={
                        <div className="scene-loading">
                          <span />
                          {t("Loading detailed local map...")}
                        </div>
                      }
                    >
                      <MapView
                        connections={app.snapshot.connections}
                        expanded={mapExpanded}
                        home={home}
                        onExpandedChange={setMapExpanded}
                        onSelect={app.setSelectedId}
                        selectedId={app.selectedId}
                      />
                    </Suspense>
                  )}
                  {app.snapshot.capture.mode === "off" && (
                    <FirstRun
                      interfaces={app.interfaces}
                      language={app.settings.ui_language}
                      onDemo={() => void app.startCapture("demo", null)}
                      onLanguage={(ui_language) =>
                        void app.updateSettings({
                          ...app.settings,
                          ui_language,
                        })
                      }
                      onLive={(networkInterface) =>
                        void app.startCapture("live", networkInterface)
                      }
                    />
                  )}
                  {app.snapshot.capture.mode !== "off" &&
                    app.settings.active_workspace === "remote" &&
                    app.snapshot.connections.length > 0 &&
                    !app.snapshot.connections.some(
                      (connection) =>
                        connection.geo?.placement !== "not_plotted" &&
                        connection.geo?.latitude !== null &&
                        connection.geo?.longitude !== null,
                    ) && (
                      <div className="geo-empty-guidance">
                        <strong>{t("Geography is not available yet")}</strong>
                        <p>
                          {t(
                            "Connections are still captured. Install or import a free local City MMDB to place public destinations on the map.",
                          )}
                        </p>
                        <button
                          className="button button--secondary"
                          onClick={() => app.setPrimaryView("settings")}
                          type="button"
                        >
                          {t("Open GeoIP settings")}
                        </button>
                      </div>
                    )}
                  <FocusBar
                    bookmarks={app.focusRules.bookmarks}
                    onFilter={showFilter}
                    onRemove={(id) => void app.removeBookmark(id)}
                  />
                  <TopContext
                    connections={app.snapshot.connections}
                    onFilter={showFilter}
                  />
                  {app.timeMachine && (
                    <div className="time-machine-badge">
                      <span>{t("TIME MACHINE")}</span>
                      <strong>
                        {new Date(app.timeMachine.at_ms).toLocaleString()}
                      </strong>
                      <small>
                        {t("{count} reconstructed routes", {
                          count: app.snapshot.connections.length,
                        })}
                      </small>
                      <button onClick={app.exitTimeMachine} type="button">
                        {t("Return to live")}
                      </button>
                    </div>
                  )}
                  {app.paused && (
                    <div className="paused-badge">
                      <Icon name="pause" />
                      {t("View paused / capture continues")}
                      {app.snapshot.capture.paused_view_events > 0
                        ? ` / ${t("{count} updates queued", {
                            count: app.snapshot.capture.paused_view_events,
                          })}`
                        : ""}
                    </div>
                  )}
                  {app.snapshot.capture.vpn_active && (
                    <div className="vpn-badge">
                      <Icon name="shield" />
                      {t(
                        "VPN interface observed / location reflects visible remote endpoints",
                      )}
                    </div>
                  )}
                </div>
                <div className="canvas-legend">
                  <span>
                    <i className="is-home" />
                    {t("This device")}
                  </span>
                  <span>
                    <i />
                    {t("Active destination")}
                  </span>
                  <span>
                    <i className="is-selected" />
                    {t("Selected route")}
                  </span>
                  <small>{t("Drag to rotate / scroll to zoom")}</small>
                </div>
              </section>
              <LiveFeed
                connections={app.snapshot.connections}
                flowTrafficAvailable={
                  app.snapshot.diagnostics.flow_counters_available
                }
                groupWindowSeconds={app.settings.feed_group_window_seconds}
                onPause={app.setPaused}
                onSelect={app.setSelectedId}
                paused={app.paused}
                queued={app.snapshot.capture.paused_view_events}
                selectedId={app.selectedId}
              />
              {app.selected && (
                <ConnectionInspector
                  connection={app.selected}
                  flowTrafficAvailable={
                    app.snapshot.diagnostics.flow_counters_available
                  }
                  onBookmark={(kind, value) =>
                    void app.addBookmark(kind, value)
                  }
                  onClose={() => app.setSelectedId(null)}
                  onFilterCountry={(value) => showFilter(`country:${value}`)}
                  onFilterDestination={(value) =>
                    showFilter(JSON.stringify(value))
                  }
                  onFilterProcess={(value) =>
                    showFilter(`process:${JSON.stringify(value)}`)
                  }
                  onIgnore={(kind, value) =>
                    void app.addIgnoreRule(kind, value)
                  }
                />
              )}
            </div>
          )}
          {app.primaryView === "processes" && (
            <ProcessView
              connections={app.snapshot.connections}
              flowCountersAvailable={
                app.snapshot.diagnostics.flow_counters_available
              }
              onFilter={showFilter}
              processes={app.snapshot.processes}
              rateHistory={app.processRateHistory}
            />
          )}
          {app.primaryView === "destinations" && (
            <DestinationView
              destinations={app.snapshot.destinations}
              onFilter={showFilter}
            />
          )}
          {app.primaryView === "history" && (
            <HistoryView
              currentQuery={app.query}
              loadHistory={app.loadHistory}
              onExport={openExport}
              onQueryChange={app.setQuery}
              onReconstruct={(atMs, sessionId, preferredId) =>
                void app.enterTimeMachine(atMs, sessionId, preferredId)
              }
            />
          )}
          {app.primaryView === "settings" && (
            <SettingsView
              focusRules={app.focusRules}
              geoStatus={app.geoStatus}
              interfaces={app.interfaces}
              onAddBookmark={(kind, value) => void app.addBookmark(kind, value)}
              onAddIgnoreRule={(kind, value) =>
                void app.addIgnoreRule(kind, value)
              }
              onClearLocalData={app.clearLocalData}
              onClearSession={app.clearSession}
              onCompactStorage={app.compactStorage}
              onExportSupportBundle={app.exportSupportBundle}
              onImportGeo={app.importGeo}
              onRemoveGeo={app.removeGeo}
              onRemoveBookmark={(id) => void app.removeBookmark(id)}
              onRemoveIgnoreRule={(id) => void app.removeIgnoreRule(id)}
              onToggleIgnoreRule={(id, enabled) =>
                void app.setIgnoreRuleEnabled(id, enabled)
              }
              onRollbackGeo={app.rollbackGeo}
              onRunSelfTest={app.runCaptureSelfTest}
              onUpdate={app.updateSettings}
              onUpdateGeo={app.updateGeo}
              settings={app.settings}
              snapshot={app.snapshot}
            />
          )}
        </main>
        <MetricStrip snapshot={app.snapshot} />
        <NotificationCenter
          items={app.notifications}
          onClear={app.clearNotifications}
          onDismiss={app.dismissNotification}
        />
        {(app.error || captureError) && (
          <ErrorBanner
            detail={originalFailureDetail}
            message={localizedFailureMessage}
            onClose={app.clearError}
            title={captureError ? t("Capture stopped") : undefined}
          />
        )}
        {app.loading && (
          <div className="boot-state">
            <span />
            <p>{t("Starting local observability engine...")}</p>
          </div>
        )}
        <CommandPalette
          localTrafficVisible={app.settings.active_workspace === "local"}
          onClearSession={() => void app.clearSession()}
          onClose={() => app.setCommandPaletteOpen(false)}
          onExport={() => openExport(app.snapshot.session?.id ?? null)}
          onPause={() => void app.setPaused(!app.paused)}
          onRecord={() =>
            recordingActive
              ? void app.stopRecording()
              : app.setRecordDialogOpen(true)
          }
          onSearchPreset={searchPreset}
          onToggleLocalTraffic={() =>
            void app.updateSettings({
              ...app.settings,
              active_workspace:
                app.settings.active_workspace === "local" ? "remote" : "local",
            })
          }
          onView={app.setPrimaryView}
          onVisualization={app.setViewMode}
          open={app.commandPaletteOpen}
          paused={app.paused}
          recording={recordingActive}
        />
        <RecordingDialog
          onClose={() => app.setRecordDialogOpen(false)}
          onStart={app.startRecording}
          onStop={() => void app.stopRecording()}
          open={app.recordDialogOpen}
          state={app.recording}
        />
        <ExportDialog
          fromMs={exportScope.fromMs}
          onClose={() => app.setExportDialogOpen(false)}
          onExport={(request) => void app.exportView(request)}
          open={app.exportDialogOpen}
          query={app.query}
          sessionId={exportScope.sessionId}
          toMs={exportScope.toMs}
        />
      </div>
    </I18nProvider>
  );
}

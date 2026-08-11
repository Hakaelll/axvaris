import {
  lazy,
  Suspense,
  useCallback,
  useEffect,
  useMemo,
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

  return (
    <I18nProvider value={language}>
      <div
        className={`app-shell${app.settings.reduce_motion ? " reduce-motion" : ""}`}
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
        <main className={`main-stage main-stage--${app.primaryView}`}>
          {app.primaryView === "globe" && (
            <div className="observatory">
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
                        reduceMotion={
                          app.settings.reduce_motion || !app.settings.auto_focus
                        }
                        selectedId={app.selectedId}
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
                        home={home}
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
                flowTrafficAvailable={app.snapshot.capture.mode === "demo"}
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
                  flowTrafficAvailable={app.snapshot.capture.mode === "demo"}
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
              onFilter={showFilter}
              processes={app.snapshot.processes}
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
              loadHistory={app.loadHistory}
              onExport={openExport}
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
              onClearSession={app.clearSession}
              onRemoveBookmark={(id) => void app.removeBookmark(id)}
              onRemoveIgnoreRule={(id) => void app.removeIgnoreRule(id)}
              onToggleIgnoreRule={(id, enabled) =>
                void app.setIgnoreRuleEnabled(id, enabled)
              }
              onUpdate={app.updateSettings}
              onUpdateGeo={app.updateGeo}
              settings={app.settings}
              snapshot={app.snapshot}
            />
          )}
        </main>
        <MetricStrip snapshot={app.snapshot} />
        {(app.error || captureError) && (
          <ErrorBanner
            message={t(
              app.error?.message ?? captureError ?? "Unknown capture error",
            )}
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
          localTrafficVisible={app.settings.show_local_traffic}
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
              show_local_traffic: !app.settings.show_local_traffic,
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

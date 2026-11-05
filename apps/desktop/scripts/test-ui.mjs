import assert from "node:assert/strict";
import path from "node:path";
import { fileURLToPath } from "node:url";
import React from "react";
import { renderToStaticMarkup } from "react-dom/server";
import { createServer } from "vite";

const root = path.resolve(fileURLToPath(new URL("..", import.meta.url)));
const server = await createServer({
  root,
  configFile: path.join(root, "vite.config.ts"),
  appType: "custom",
  logLevel: "error",
  server: { middlewareMode: true },
});

try {
  const privacy = await server.ssrLoadModule(
    "/src/privacy/PresentationMode.tsx",
  );
  const { I18nProvider } = await server.ssrLoadModule("/src/i18n.ts");
  const { TopBar } = await server.ssrLoadModule("/src/components/TopBar.tsx");
  const { groupDevices } = await server.ssrLoadModule(
    "/src/components/LocalPeers.tsx",
  );
  const { classifyUnlocatedTargets } = await server.ssrLoadModule(
    "/src/lib/unlocated.ts",
  );
  const { LatestRequest } = await server.ssrLoadModule(
    "/src/lib/latestRequest.ts",
  );
  const { findSettingsSections } = await server.ssrLoadModule(
    "/src/lib/settingsSearch.ts",
  );
  const { desktopApi: previewApi } = await server.ssrLoadModule(
    "/src/lib/backend.ts",
  );
  const { ServiceDiagram } = await server.ssrLoadModule("/src/components/ServiceDiagram.tsx");
  const { HistoryInspector } = await server.ssrLoadModule("/src/components/HistoryInspector.tsx");
  const { mapPoint } = await server.ssrLoadModule("/src/lib/world-detail.ts");
  const { buildInvestigationQuery, savedInvestigationWindow, saveInvestigationDraft } = await server.ssrLoadModule("/src/lib/investigation.ts");
  const { isShortcutControl } = await server.ssrLoadModule("/src/lib/shortcuts.ts");

  const domGlobals = ["Element", "HTMLElement", "HTMLInputElement", "HTMLTextAreaElement", "HTMLSelectElement"];
  const previousDomGlobals = domGlobals.map((name) => globalThis[name]);
  class ShortcutElement {
    constructor(tag, parent = null) { this.tag = tag; this.parent = parent; this.isContentEditable = false; }
    closest(selector) {
      return selector.split(",").some((item) => item.trim() === this.tag)
        ? this : this.parent?.closest(selector) ?? null;
    }
  }
  try {
    globalThis.Element = globalThis.HTMLElement = ShortcutElement;
    globalThis.HTMLInputElement = class extends ShortcutElement {};
    globalThis.HTMLTextAreaElement = class extends ShortcutElement {};
    globalThis.HTMLSelectElement = class extends ShortcutElement {};
    const button = new ShortcutElement("button");
    assert.equal(isShortcutControl(button), true, "Space belongs to the focused button");
    assert.equal(isShortcutControl(new ShortcutElement("span", button)), true, "button descendants preserve native keyboard activation");
    assert.equal(isShortcutControl(new ShortcutElement("summary")), true, "Space toggles the investigation editor");
    assert.equal(isShortcutControl(new ShortcutElement("p", new ShortcutElement("dialog"))), true, "workspace shortcuts do not run inside dialogs");
    assert.equal(isShortcutControl(new ShortcutElement("main")), false);
    assert.equal(isShortcutControl(null), false);
  } finally {
    domGlobals.forEach((name, index) => {
      if (previousDomGlobals[index] === undefined) delete globalThis[name];
      else globalThis[name] = previousDomGlobals[index];
    });
  }

  const requestedHistory = {
    from_ms: 1_000, to_ms: null, session_id: null, query: "protocol:tcp", limit: 500, cursor: null,
  };
  const savedQuery = buildInvestigationQuery({
    query: requestedHistory,
    savedAtMs: 2_100,
    pinnedConnectionIds: [], pinnedEntries: [],
  });
  assert.equal(savedQuery.from_ms, 1_000, "saving preserves the requested interval beyond the loaded 500 rows");
  assert.equal(savedQuery.to_ms, 2_100, "an open-ended investigation freezes at save time");
  assert.equal(savedQuery.filter, "protocol:tcp");
  const frozenWindow = savedInvestigationWindow(null, savedQuery);
  const autoSavedQuery = buildInvestigationQuery({
    query: { ...requestedHistory, ...frozenWindow }, savedAtMs: 9_999,
    pinnedConnectionIds: [], pinnedEntries: [], previousQuery: savedQuery,
  });
  assert.equal(autoSavedQuery.to_ms, 2_100, "metadata autosave cannot advance the first saved end");
  assert.equal(savedInvestigationWindow(frozenWindow, autoSavedQuery), frozenWindow, "adopting the same saved window does not restart autosave");
  const saveRequests = new LatestRequest();
  const saveA = Promise.withResolvers();
  const draftA = { id: "A", name: "A", query: savedQuery };
  const pendingA = saveInvestigationDraft({ draft: draftA, requests: saveRequests, persist: () => saveA.promise });
  saveRequests.invalidate();
  saveA.resolve(draftA);
  assert.equal(await pendingA, null, "a delayed A autosave cannot reactivate A after opening B or a new workspace");
  const saveOldRange = Promise.withResolvers();
  const pendingOldRange = saveInvestigationDraft({ draft: draftA, requests: saveRequests, persist: () => saveOldRange.promise });
  saveRequests.invalidate();
  saveOldRange.resolve(draftA);
  assert.equal(await pendingOldRange, null, "a delayed save cannot restore a range the user has changed");
  assert.equal(await saveInvestigationDraft({ draft: draftA, requests: saveRequests, persist: async (draft) => draft }), draftA);
  const savedPins = {
    ...savedQuery, pinned_connection_ids: ["off-page"],
    pinned_destination_ips: ["203.0.113.9"], pinned_process_keys: ["retained-process"], pinned_asns: [64500],
  };
  const metadataOnlyEdit = buildInvestigationQuery({
    query: { ...requestedHistory, from_ms: savedPins.from_ms, to_ms: savedPins.to_ms },
    savedAtMs: 9_999, previousQuery: savedPins,
    pinnedConnectionIds: ["off-page"], pinnedEntries: [],
  });
  assert.deepEqual(metadataOnlyEdit, savedPins, "autosave retains the frozen interval and unloaded pin scope");
  assert.deepEqual(buildInvestigationQuery({
    query: requestedHistory, savedAtMs: 9_999, previousQuery: savedPins,
    pinnedConnectionIds: [], pinnedEntries: [],
  }).pinned_destination_ips, [], "explicitly clearing pins removes their derived selection");
  assert.deepEqual(buildInvestigationQuery({
    query: { ...requestedHistory, from_ms: null, to_ms: 1_900, session_id: "retained-session" },
    savedAtMs: 2_100, pinnedConnectionIds: [], pinnedEntries: [],
  }), {
    filter: "protocol:tcp", from_ms: null, to_ms: 1_900, session_id: "retained-session",
    pinned_connection_ids: [], pinned_destination_ips: [], pinned_process_keys: [], pinned_asns: [],
  }, "explicit windows and session scope remain unchanged");
  const { processNotificationConnections } = await server.ssrLoadModule("/src/lib/notifications.ts");

  const sensitive = {
    process: "Secret Browser",
    domain: "private.example.com",
    publicIp: "93.184.216.34",
    localIp: "192.168.1.20",
    path: "C:\\Users\\alice\\Secret Browser.exe",
    user: "alice",
  };
  const connection = {
    id: "privacy-fixture",
    process: {
      name: sensitive.process,
      executable: sensitive.path,
      executable_path: sensitive.path,
      user_id: sensitive.user,
    },
    domain: sensitive.domain,
    hostname: sensitive.domain,
    key: {
      local: { address: sensitive.localIp },
      remote: { address: sensitive.publicIp, scope: "global" },
    },
  };
  const originalFilter = `process:${JSON.stringify(sensitive.process)} AND domain:${JSON.stringify(sensitive.domain)}`;

  function PrivacyFixture() {
    const presentation = privacy.usePresentation();
    const idempotentDomain = presentation.displayText(
      presentation.displayText(sensitive.domain),
    );
    return React.createElement(
      "section",
      {
        title: presentation.displayText(
          `${sensitive.path} ${sensitive.domain}`,
        ),
        "aria-label": presentation.displayText(
          `${sensitive.process} ${sensitive.publicIp}`,
        ),
      },
      React.createElement(privacy.SensitiveText, {
        kind: "process",
        value: sensitive.process,
      }),
      React.createElement(privacy.SensitiveText, {
        kind: "domain",
        value: sensitive.domain,
      }),
      idempotentDomain,
      presentation.display("user", sensitive.user),
    );
  }

  const privacyMarkup = renderToStaticMarkup(
    React.createElement(
      privacy.PresentationProvider,
      {
        active: true,
        snapshot: { connections: [connection] },
        settings: { selected_interface: null },
        interfaces: [],
      },
      React.createElement(PrivacyFixture),
    ),
  );
  for (const value of Object.values(sensitive)) {
    assert.equal(
      privacyMarkup.includes(value),
      false,
      `presentation markup leaked ${value}`,
    );
  }
  assert.match(privacyMarkup, /Application 1/);
  assert.match(privacyMarkup, /domain-1\.invalid/);
  assert.doesNotMatch(privacyMarkup, /domain-[2-9]\d*\.invalid/);
  assert.equal(
    originalFilter,
    `process:${JSON.stringify(sensitive.process)} AND domain:${JSON.stringify(sensitive.domain)}`,
    "presentation rendering changed the real filter",
  );

  const topBarMarkup = renderToStaticMarkup(
    React.createElement(
      I18nProvider,
      { value: "en" },
      React.createElement(TopBar, {
        primaryView: "today",
        onView() {},
        query: "",
        queryError: null,
        onQuery() {},
        capture: {
          mode: "live",
          status: { state: "capturing" },
          selected_interface: null,
          paused_view_events: 0,
          vpn_active: false,
        },
        interfaces: [],
        onCapture() {},
        onPalette() {},
        onRecord() {},
        recording: false,
        presentationMode: false,
      }),
    ),
  );
  assert.match(topBarMarkup, /topbar-compact-menu/);
  assert.match(topBarMarkup, /Capture mode in compact menu/);
  assert.match(topBarMarkup, /Capture status: Live capture/);
  assert.match(topBarMarkup, /Open capture and actions menu/);

  const unlocated = classifyUnlocatedTargets(
    [
      {
        id: "local",
        key: { remote: { scope: "private" } },
        geo: null,
        last_seen_ms: 1,
      },
      {
        id: "pending",
        key: { remote: { scope: "global" } },
        geo: null,
        last_seen_ms: 9_000,
      },
      {
        id: "plotted",
        key: { remote: { scope: "global" } },
        last_seen_ms: 1,
        geo: { placement: "database_coordinate", latitude: 40, longitude: -3 },
      },
    ],
    10_000,
    true,
  );
  assert.deepEqual(
    unlocated.map((item) => item.reason),
    ["local_or_reserved", "pending"],
  );

  const latest = new LatestRequest();
  const stale = latest.begin();
  const current = latest.begin();
  assert.equal(latest.isCurrent(stale), false);
  assert.equal(latest.isCurrent(current), true);

  assert.equal(findSettingsSections("ubicación")[0]?.section, "geo");
  assert.equal(findSettingsSections("seudónimos")[0]?.section, "display");

  const localNeighbors = [
    { address: "192.168.1.10", mac_address: "02:11:22:33:44:55", interface_id: "wifi", is_router: false, state: "reachable" },
    { address: "fe80::10", mac_address: "02:11:22:33:44:55", interface_id: "wifi", is_router: false, state: "stale" },
    { address: "192.168.1.11", mac_address: "02:11:22:33:44:66", interface_id: "wifi", is_router: false, state: "reachable" },
  ];
  const localConnections = [
    { id: "ipv4", key: { remote: { address: "192.168.1.10", port: 443, scope: "private", zone: null } }, active: true, hostname: null, interface: "Wi-Fi", process: { name: "Browser" }, last_seen_ms: 100 },
    { id: "ipv6", key: { remote: { address: "fe80::10", port: 80, scope: "link_local", zone: null } }, active: false, hostname: null, interface: "Wi-Fi", process: { name: "Browser" }, last_seen_ms: 90 },
    { id: "unknown", key: { remote: { address: "192.168.1.12", port: 22, scope: "private", zone: null } }, active: false, hostname: null, interface: "Wi-Fi", process: { name: "SSH" }, last_seen_ms: 80 },
  ];
  const deviceGroups = groupDevices(localConnections, localNeighbors, "ipv6");
  assert.equal(deviceGroups.length, 3, "different MACs and unverified addresses remain separate");
  const dualStack = deviceGroups.find((device) => device.mac === "02:11:22:33:44:55");
  assert.deepEqual([...dualStack.addresses].sort(), ["192.168.1.10", "fe80::10"]);
  assert.equal(dualStack.connectionCount, 2);
  assert.deepEqual([...dualStack.connectionIds].sort(), ["ipv4", "ipv6"], "device inspection retains only the connections attributed to that group");
  assert.equal(dualStack.selected, true);
  assert.equal(deviceGroups.find((device) => device.addresses.has("192.168.1.12")).mac, null);

  const todayBootstrap = await previewApi.getTodayBootstrap();
  const quickComparison = todayBootstrap.previous_session?.comparison;
  assert.ok(quickComparison, "preview exposes a prior-session comparison");
  assert.equal(
    new Set(quickComparison.processes.map((row) => row.key)).size,
    quickComparison.processes.length,
    "process comparison rows are unique",
  );
  assert.ok(
    quickComparison.countries.every((row) => /^[A-Z]{2}$/.test(row.key)),
    "country comparison rows use country codes",
  );
  await previewApi.setCaptureState("demo", null);
  const demoUpdate = await previewApi.getDashboardUpdate(null);
  const demoConnection = demoUpdate.snapshot?.connections[0] ?? demoUpdate.upserted_connections[0];
  assert.ok(demoConnection, "demo capture supplies an actual connection fixture");
  const notificationSettings = {
    ...await previewApi.getSettings(),
    notification_rules: [
      { id: "port-rule", enabled: true, kind: "port", value: String(demoConnection.key.remote.port), cooldown_seconds: 10 },
    ],
    notification_quiet_hours_enabled: true,
    notification_quiet_start_minute: 1_320,
    notification_quiet_end_minute: 420,
  };
  const freshNotificationTracking = () => ({
    seenConnectionIds: new Set(), seenProcesses: new Set(), seenCountries: new Set(), cooldowns: new Map(),
  });
  const nativeNotifications = [];
  const originalNotification = globalThis.Notification;
  globalThis.Notification = class {
    static permission = "granted";
    constructor(title, options) { nativeNotifications.push({ title, ...options }); }
  };
  try {
    for (const [enabled, start, end, minute, muted] of [
      [true, 1_320, 420, 1_380, true],
      [true, 540, 1_020, 539, false],
      [true, 540, 1_020, 540, true],
      [true, 540, 1_020, 1_019, true],
      [true, 540, 1_020, 1_020, false],
      [true, 1_320, 420, 1_319, false],
      [true, 1_320, 420, 1_320, true],
      [true, 1_320, 420, 1_439, true],
      [true, 1_320, 420, 0, true],
      [true, 1_320, 420, 419, true],
      [true, 1_320, 420, 420, false],
      [false, 1_320, 420, 0, false],
      [true, 540, 540, 540, false],
      [true, 1_440, 420, 0, false],
    ]) {
      nativeNotifications.length = 0;
      const tracking = freshNotificationTracking();
      const settings = {
        ...notificationSettings,
        notification_quiet_hours_enabled: enabled,
        notification_quiet_start_minute: start,
        notification_quiet_end_minute: end,
      };
      const now = new Date(2026, 10, 5, Math.floor(minute / 60), minute % 60).getTime();
      const created = processNotificationConnections([demoConnection], settings, tracking, now);
      assert.equal(created.length, muted ? 0 : 1, `quiet hours ${start}-${end} at minute ${minute}`);
      assert.equal(nativeNotifications.length, muted ? 0 : 1, "native alerts respect the same quiet-hours gate");
      assert.ok(tracking.seenConnectionIds.has(demoConnection.id), "muted connections are still consumed");
      if (muted) assert.equal(tracking.cooldowns.size, 0, "silence does not start an alert cooldown");
    }
    const tracking = freshNotificationTracking();
    const quietTime = new Date(2026, 10, 5, 23).getTime();
    const morningTime = new Date(2026, 10, 6, 7).getTime();
    processNotificationConnections([demoConnection], notificationSettings, tracking, quietTime);
    assert.equal(processNotificationConnections([demoConnection], notificationSettings, tracking, morningTime).length, 0, "quiet-window evidence does not become a delayed alert");
    const nextConnection = { ...demoConnection, id: "after-quiet-hours" };
    assert.equal(processNotificationConnections([nextConnection], notificationSettings, tracking, morningTime).length, 1);
    assert.equal(processNotificationConnections([{ ...nextConnection, id: "during-cooldown" }], notificationSettings, tracking, morningTime + 9_999).length, 0);
    assert.equal(processNotificationConnections([{ ...nextConnection, id: "at-cooldown-end" }], notificationSettings, tracking, morningTime + 10_000).length, 1);
    const newEvidenceTracking = freshNotificationTracking();
    const noveltySettings = {
      ...notificationSettings,
      notification_rules: [
        { id: "new-process", enabled: true, kind: "new_process", value: null, cooldown_seconds: 0 },
        { id: "new-country", enabled: true, kind: "new_country", value: null, cooldown_seconds: 0 },
      ],
    };
    const countryConnection = { ...demoConnection, geo: { ...demoConnection.geo, country_code: "ES" } };
    processNotificationConnections([countryConnection], noveltySettings, newEvidenceTracking, quietTime);
    assert.equal(processNotificationConnections([{ ...countryConnection, id: "same-muted-process-and-country" }], noveltySettings, newEvidenceTracking, morningTime).length, 0, "muted process and country evidence is consumed without a backlog");
    const minimumCooldownTracking = freshNotificationTracking();
    const minimumCooldownSettings = { ...notificationSettings, notification_rules: [{ ...notificationSettings.notification_rules[0], cooldown_seconds: 0 }] };
    assert.equal(processNotificationConnections([demoConnection], minimumCooldownSettings, minimumCooldownTracking, morningTime).length, 1);
    assert.equal(processNotificationConnections([{ ...demoConnection, id: "minimum-cooldown" }], minimumCooldownSettings, minimumCooldownTracking, morningTime + 4_999).length, 0);
    assert.equal(processNotificationConnections([{ ...demoConnection, id: "minimum-cooldown-end" }], minimumCooldownSettings, minimumCooldownTracking, morningTime + 5_000).length, 1);
  } finally {
    if (originalNotification === undefined) delete globalThis.Notification;
    else globalThis.Notification = originalNotification;
  }
  const listener = {
    ...demoConnection,
    id: "test-listener",
    state: "listening",
    counter_source: "unavailable",
    key: { ...demoConnection.key, local: { ...demoConnection.key.local, address: "127.0.0.1", port: 3000 }, remote: { ...demoConnection.key.remote, address: "0.0.0.0", port: 0 } },
    traffic: { bytes_sent: 0, bytes_received: 0, packets_sent: 0, packets_received: 0 },
  };
  const diagramMarkup = renderToStaticMarkup(React.createElement(I18nProvider, { value: "en" }, React.createElement(ServiceDiagram, { connections: [listener], selectedId: listener.id, onSelect() {} })));
  assert.match(diagramMarkup, /127\.0\.0\.1:3000/);
  assert.doesNotMatch(diagramMarkup, /0\.0\.0\.0:0/, "listener diagram must show the bound local endpoint");
  assert.match(diagramMarkup, /aria-pressed="true"/);
  const inspectorMarkup = renderToStaticMarkup(React.createElement(I18nProvider, { value: "en" }, React.createElement(HistoryInspector, { entry: { session_id: "test-session", connection: listener }, onClose() {}, onReconstruct() {} })));
  assert.doesNotMatch(inspectorMarkup, /0 B/, "unavailable counters must remain absent rather than becoming zero");
  assert.match(inspectorMarkup, /—/);
  assert.deepEqual(mapPoint(-180, 90), [0, 0]);
  assert.deepEqual(mapPoint(180, -90), [1000, 500]);
  assert.deepEqual(mapPoint(0, 0), [500, 250], "atlas, markers and relief raster must share the same equirectangular registration");

  console.log(
    "UI behavior passed: investigation windows and frozen autosave, unloaded pins and stale saves, shortcut controls, presentation DOM, compact capture navigation, unlocated classification, local device grouping, stale-response gate, bilingual settings search, quick comparison, listener endpoints, absent counters, relief registration and notification quiet hours with cooldowns.",
  );
} finally {
  await server.close();
}

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

  console.log(
    "UI behavior passed: presentation DOM, compact capture navigation, unlocated classification, stale-response gate, bilingual settings search, quick comparison.",
  );
} finally {
  await server.close();
}

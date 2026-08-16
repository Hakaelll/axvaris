import { useEffect, useMemo, useState } from "react";
import { useI18n } from "../i18n";
import { desktopApi } from "../lib/backend";
import type {
  DashboardSnapshot,
  LocalNotificationRecord,
  LocalRule,
  LocalRuleAction,
  LocalRuleCondition,
  NetworkProfile,
  TodayCardState,
  Connection,
} from "../types";
import { Icon } from "../components/Icon";

interface TodayViewProps {
  snapshot: DashboardSnapshot;
  onOpenEvidence: (connectionId: string | null, query: string) => void;
}

interface DecisionCard {
  id: string;
  category: string;
  scope: "device" | "application" | "profile" | "health";
  title: string;
  detail: string;
  connectionId: string | null;
  query: string;
}

type RuleField = LocalRuleCondition["field"];

const ruleFields: RuleField[] = [
  "process",
  "executable_revision",
  "domain",
  "ip",
  "cidr",
  "asn",
  "country",
  "protocol",
  "port",
  "listener",
  "network_profile",
  "path",
  "first_seen",
  "minimum_bytes",
  "minimum_duration_ms",
];

function conditionFrom(field: RuleField, value: string): LocalRuleCondition | null {
  const cleaned = value.trim();
  if (field === "first_seen") {
    return { field, value: cleaned !== "false" };
  }
  if (field === "asn" || field === "port" || field === "minimum_bytes" || field === "minimum_duration_ms") {
    const parsed = Number(cleaned);
    return Number.isFinite(parsed) && parsed >= 0
      ? ({ field, value: parsed } as LocalRuleCondition)
      : null;
  }
  if (!cleaned) return null;
  return { field, value: cleaned } as LocalRuleCondition;
}

function cardStateFor(states: TodayCardState[], id: string): TodayCardState {
  return (
    states.find((state) => state.id === id) ?? {
      id,
      dismissed: false,
      pinned: false,
      updated_at_ms: 0,
    }
  );
}

function deduplicateCards(cards: DecisionCard[]): DecisionCard[] {
  const seen = new Set<string>();
  return cards.filter((card) => {
    if (seen.has(card.id)) return false;
    seen.add(card.id);
    return true;
  });
}

export function TodayView({ snapshot, onOpenEvidence }: TodayViewProps) {
  const t = useI18n();
  const [states, setStates] = useState<TodayCardState[]>([]);
  const [profiles, setProfiles] = useState<NetworkProfile[]>([]);
  const [rules, setRules] = useState<LocalRule[]>([]);
  const [notifications, setNotifications] = useState<LocalNotificationRecord[]>([]);
  const [comparisonBaseline, setComparisonBaseline] = useState<Connection[]>([]);
  const [ruleName, setRuleName] = useState("");
  const [field, setField] = useState<RuleField>("process");
  const [value, setValue] = useState("");
  const [conditions, setConditions] = useState<LocalRuleCondition[]>([]);
  const [matchAll, setMatchAll] = useState(true);
  const [tag, setTag] = useState("");
  const [pin, setPin] = useState(false);
  const [notify, setNotify] = useState(true);
  const [previewCount, setPreviewCount] = useState<number | null>(null);
  const [previewReasons, setPreviewReasons] = useState<string[]>([]);
  const [status, setStatus] = useState<string | null>(null);

  const refreshLocalState = async () => {
    const [nextStates, nextProfiles, nextRules, nextNotifications, retained] =
      await Promise.all([
        desktopApi.listTodayCardStates(),
        desktopApi.listNetworkProfiles(),
        desktopApi.listLocalRules(),
        desktopApi.listLocalNotifications(),
        desktopApi.queryHistory({
          from_ms: null,
          to_ms: null,
          session_id: null,
          query: null,
          limit: 100_000,
        }),
      ]);
    setStates(nextStates);
    setProfiles(nextProfiles);
    setRules(nextRules);
    setNotifications(nextNotifications);
    setComparisonBaseline(
      retained.entries
        .filter((entry) => entry.session_id !== snapshot.session?.id)
        .map((entry) => entry.connection),
    );
  };

  useEffect(() => {
    void refreshLocalState().catch((error: unknown) =>
      setStatus(error instanceof Error ? error.message : String(error)),
    );
  }, [snapshot.session?.id]);

  const cards = useMemo(() => {
    const result: DecisionCard[] = [];
    const newProcesses = new Map<string, (typeof snapshot.connections)[number]>();
    const newDomains = new Map<string, (typeof snapshot.connections)[number]>();
    const newAsns = new Map<number, (typeof snapshot.connections)[number]>();
    const listeners = snapshot.connections.filter(
      (connection) =>
        connection.evidence.endpoint_role === "listener" ||
        connection.state === "listening",
    );

    const baselineProcesses = new Set(
      comparisonBaseline.map((connection) => connection.process.name),
    );
    const baselineProcessDestinations = new Set(
      comparisonBaseline.map(
        (connection) =>
          `${connection.process.name}\u0000${connection.key.remote.address}`,
      ),
    );
    const baselineDomains = new Set(
      comparisonBaseline.flatMap((connection) =>
        connection.domain ? [connection.domain] : [],
      ),
    );
    const baselineAsns = new Set(
      comparisonBaseline.flatMap((connection) =>
        connection.asn ? [connection.asn.number] : [],
      ),
    );

    for (const connection of snapshot.connections) {
      if (!baselineProcesses.has(connection.process.name)) {
        newProcesses.set(connection.process.name, connection);
      }
      if (connection.domain && !baselineDomains.has(connection.domain)) {
        newDomains.set(connection.domain, connection);
      }
      if (connection.asn && !baselineAsns.has(connection.asn.number)) {
        newAsns.set(connection.asn.number, connection);
      }
    }

    for (const [name, connection] of newProcesses) {
      result.push({
        id: `process:${name}`,
        category: t("New network process"),
        scope: "device",
        title: name,
        detail: t("First observed opening a network connection on this device."),
        connectionId: connection.id,
        query: `process:${JSON.stringify(name)}`,
      });
    }
    for (const connection of snapshot.connections.filter(
      (item) =>
        !baselineProcessDestinations.has(
          `${item.process.name}\u0000${item.key.remote.address}`,
        ),
    )) {
      result.push({
        id: `destination:${connection.process.name}:${connection.key.remote.address}`,
        category: t("New destination"),
        scope: "application",
        title: connection.domain ?? connection.key.remote.address,
        detail: t("New for {process}; open the evidence to inspect source and quality.", {
          process: connection.process.name,
        }),
        connectionId: connection.id,
        query: JSON.stringify(connection.domain ?? connection.key.remote.address),
      });
    }
    for (const [domain, connection] of newDomains) {
      result.push({
        id: `domain:${domain}`,
        category: t("New domain"),
        scope: "profile",
        title: domain,
        detail: t("First observed in the retained local baseline for this network context."),
        connectionId: connection.id,
        query: `domain:${JSON.stringify(domain)}`,
      });
    }
    for (const [number, connection] of newAsns) {
      result.push({
        id: `asn:${number}`,
        category: t("New network owner hint"),
        scope: "profile",
        title: `AS${number} ${connection.asn?.organization ?? ""}`.trim(),
        detail: t("ASN is local dataset context, not a security verdict."),
        connectionId: connection.id,
        query: `asn:${number}`,
      });
    }
    if (listeners.length > 0) {
      result.push({
        id: `listeners:${listeners.map((item) => item.id).sort().join(":")}`,
        category: t("Listener change"),
        scope: "device",
        title: t("{count} local services are listening", { count: listeners.length }),
        detail: t("Review bind scope and firewall evidence in Exposure."),
        connectionId: listeners[0]?.id ?? null,
        query: "state:listening",
      });
    }
    if (snapshot.capture.vpn_active) {
      result.push({
        id: `vpn:${snapshot.diagnostics.capture_generation}`,
        category: t("Network path change"),
        scope: "profile",
        title: t("VPN interface observed"),
        detail: t("Route evidence is tied to the current network generation."),
        connectionId: snapshot.connections[0]?.id ?? null,
        query: "",
      });
    }
    for (const subsystem of snapshot.diagnostics.subsystems.filter(
      (item) => item.status === "degraded" || item.status === "unavailable",
    )) {
      result.push({
        id: `health:${subsystem.subsystem}:${subsystem.since_ms ?? 0}`,
        category: t("Capability degradation"),
        scope: "health",
        title: subsystem.subsystem,
        detail: `${subsystem.message}${subsystem.action ? ` ${subsystem.action}` : ""}`,
        connectionId: null,
        query: "",
      });
    }
    return deduplicateCards(result);
  }, [comparisonBaseline, snapshot, t]);

  const visibleCards = cards
    .filter((card) => !cardStateFor(states, card.id).dismissed)
    .sort(
      (left, right) =>
        Number(cardStateFor(states, right.id).pinned) -
        Number(cardStateFor(states, left.id).pinned),
    );

  const updateCard = async (id: string, patch: Partial<TodayCardState>) => {
    const current = cardStateFor(states, id);
    const next = await desktopApi.saveTodayCardState({
      ...current,
      ...patch,
      id,
      updated_at_ms: Date.now(),
    });
    setStates((items) => [next, ...items.filter((item) => item.id !== id)]);
  };

  const draftRule = (): LocalRule => {
    const existing = rules.find((rule) => rule.name === ruleName.trim());
    const actions: LocalRuleAction[] = [];
    if (tag.trim()) actions.push({ action: "tag", value: tag.trim() });
    if (pin) actions.push({ action: "pin" });
    if (notify) actions.push({ action: "notify" });
    const now = Date.now();
    return {
      id: existing?.id ?? crypto.randomUUID(),
      version: (existing?.version ?? 0) + 1,
      name: ruleName.trim() || t("Untitled local rule"),
      enabled: true,
      match_all: matchAll,
      conditions,
      actions,
      cooldown_seconds: 300,
      created_at_ms: existing?.created_at_ms ?? now,
      updated_at_ms: now,
    };
  };

  const addCondition = () => {
    const condition = conditionFrom(field, value);
    if (!condition) {
      setStatus(t("Enter a valid value for the selected condition."));
      return;
    }
    setConditions((items) => [...items, condition]);
    setValue("");
    setStatus(null);
  };

  const previewRule = async () => {
    if (conditions.length === 0) {
      setStatus(t("Add at least one typed condition."));
      return;
    }
    const matches = await desktopApi.previewLocalRule(draftRule(), {
      from_ms: null,
      to_ms: null,
      session_id: null,
      query: null,
      limit: 100_000,
    });
    setPreviewCount(matches.length);
    setPreviewReasons([...new Set(matches.flatMap((match) => match.reasons))].slice(0, 4));
    setStatus(t("Dry-run completed locally."));
  };

  const saveRule = async () => {
    const draft = draftRule();
    if (draft.conditions.length === 0 || draft.actions.length === 0) {
      setStatus(t("A rule needs a condition and at least one safe action."));
      return;
    }
    const signature = JSON.stringify([draft.match_all, draft.conditions, draft.actions]);
    const duplicate = rules.find(
      (rule) =>
        rule.id !== draft.id &&
        JSON.stringify([rule.match_all, rule.conditions, rule.actions]) === signature,
    );
    if (duplicate) {
      setStatus(t("Duplicate rule: {name}", { name: duplicate.name }));
      return;
    }
    await desktopApi.saveLocalRule(draft);
    setRuleName("");
    setConditions([]);
    setPreviewCount(null);
    setStatus(t("Rule version {version} saved locally.", { version: draft.version }));
    await refreshLocalState();
  };

  const updateNotification = async (
    item: LocalNotificationRecord,
    state: LocalNotificationRecord["state"],
    snoozedUntil: number | null = item.snoozed_until_ms,
  ) => {
    await desktopApi.updateLocalNotification(item.id, state, snoozedUntil);
    await refreshLocalState();
  };

  return (
    <section className="today-view" aria-labelledby="today-title">
      <header className="view-heading">
        <div>
          <span className="eyebrow">{t("LOCAL DECISION SUMMARY")}</span>
          <h1 id="today-title">{t("What changed today")}</h1>
          <p>{t("Changes are observations with traceable evidence, never malware verdicts.")}</p>
        </div>
        <div className="today-profile-summary">
          <strong>{profiles[0]?.display_name ?? t("Current network profile")}</strong>
          <small>{t("{count} local profiles", { count: profiles.length })}</small>
        </div>
      </header>

      <div className="today-layout">
        <section className="today-cards" aria-labelledby="today-changes-title">
          <header>
            <div>
              <h2 id="today-changes-title">{t("Changes worth reviewing")}</h2>
              <p>{t("Open any card to reach its concrete connection evidence in one step.")}</p>
            </div>
            <span>{t("{count} visible", { count: visibleCards.length })}</span>
          </header>
          <div className="today-card-grid">
            {visibleCards.map((card) => {
              const state = cardStateFor(states, card.id);
              return (
                <article className={`today-card${state.pinned ? " is-pinned" : ""}`} key={card.id}>
                  <header>
                    <span>{card.category}</span>
                    <small>{t(`scope_${card.scope}`)}</small>
                  </header>
                  <h3>{card.title}</h3>
                  <p>{card.detail}</p>
                  <footer>
                    <button
                      className="button button--primary"
                      disabled={!card.connectionId && !card.query}
                      onClick={() => onOpenEvidence(card.connectionId, card.query)}
                      type="button"
                    >
                      {t("Open evidence")}
                    </button>
                    <button
                      aria-pressed={state.pinned}
                      className="button button--secondary"
                      onClick={() => void updateCard(card.id, { pinned: !state.pinned })}
                      type="button"
                    >
                      {state.pinned ? t("Unpin") : t("Pin")}
                    </button>
                    <button
                      className="button button--secondary"
                      onClick={() => void updateCard(card.id, { dismissed: true })}
                      type="button"
                    >
                      {t("Dismiss")}
                    </button>
                  </footer>
                </article>
              );
            })}
            {visibleCards.length === 0 && (
              <div className="today-empty">
                <Icon name="activity" />
                <strong>{t("No new retained changes to review")}</strong>
                <p>{t("Start capture or use Demo; provider limitations remain visible in Data & Health.")}</p>
              </div>
            )}
          </div>
        </section>

        <aside className="today-notifications" aria-labelledby="today-notifications-title">
          <header>
            <h2 id="today-notifications-title">{t("Local notification inbox")}</h2>
            <span>{notifications.filter((item) => item.state === "unread").length}</span>
          </header>
          {notifications.slice(0, 12).map((item) => (
            <article key={item.id}>
              <div>
                <strong>{item.title}</strong>
                <small>{t("{count} grouped events", { count: item.count })}</small>
              </div>
              <p>{item.detail}</p>
              <div className="notification-actions">
                <button onClick={() => onOpenEvidence(item.evidence_ids[0] ?? null, "")} type="button">
                  {t("Evidence")}
                </button>
                <button onClick={() => void updateNotification(item, item.state === "pinned" ? "read" : "pinned")} type="button">
                  {item.state === "pinned" ? t("Unpin") : t("Pin")}
                </button>
                <button onClick={() => void updateNotification(item, "read", Date.now() + 60 * 60 * 1000)} type="button">
                  {t("Snooze 1 h")}
                </button>
                <button onClick={() => void updateNotification(item, "resolved", null)} type="button">
                  {t("Resolve")}
                </button>
              </div>
            </article>
          ))}
          {notifications.length === 0 && <p className="list-empty">{t("No local notifications.")}</p>}
        </aside>
      </div>

      <section className="rule-studio" aria-labelledby="rule-studio-title">
        <header>
          <div>
            <span className="eyebrow">{t("SAFE LOCAL AUTOMATION")}</span>
            <h2 id="rule-studio-title">{t("Rules and labels")}</h2>
            <p>{t("Rules may tag, pin, add to an investigation or notify. They cannot block traffic, run commands or start packet capture.")}</p>
          </div>
          <span>{t("{count} versioned rules", { count: rules.length })}</span>
        </header>
        <div className="rule-studio__editor">
          <label>
            <span>{t("Rule name")}</span>
            <input onChange={(event) => setRuleName(event.target.value)} value={ruleName} />
          </label>
          <label>
            <span>{t("Typed field")}</span>
            <select onChange={(event) => setField(event.target.value as RuleField)} value={field}>
              {ruleFields.map((item) => <option key={item} value={item}>{t(item)}</option>)}
            </select>
          </label>
          <label>
            <span>{t("Value")}</span>
            <input
              onChange={(event) => setValue(event.target.value)}
              placeholder={field === "first_seen" ? "true" : field === "protocol" ? "tcp" : ""}
              value={value}
            />
          </label>
          <button className="button button--secondary" onClick={addCondition} type="button">{t("Add condition")}</button>
        </div>
        <div className="rule-condition-list" aria-live="polite">
          {conditions.map((condition, index) => (
            <button key={`${condition.field}-${index}`} onClick={() => setConditions((items) => items.filter((_, itemIndex) => itemIndex !== index))} type="button">
              <strong>{t(condition.field)}</strong>
              <span>{String(condition.value)}</span>
              <Icon name="close" />
            </button>
          ))}
          <label><input checked={matchAll} onChange={(event) => setMatchAll(event.target.checked)} type="checkbox" />{t("Require every condition (AND)")}</label>
        </div>
        <fieldset className="rule-actions">
          <legend>{t("Safe actions")}</legend>
          <label><input checked={notify} onChange={(event) => setNotify(event.target.checked)} type="checkbox" />{t("Notify locally")}</label>
          <label><input checked={pin} onChange={(event) => setPin(event.target.checked)} type="checkbox" />{t("Pin matching evidence")}</label>
          <label><span>{t("Tag")}</span><input onChange={(event) => setTag(event.target.value)} value={tag} /></label>
        </fieldset>
        <div className="rule-studio__controls">
          <button className="button button--secondary" onClick={() => void previewRule()} type="button">{t("Preview on retained history")}</button>
          <button className="button button--primary" onClick={() => void saveRule()} type="button">{t("Save new version")}</button>
          {previewCount !== null && <span>{t("{count} dry-run matches", { count: previewCount })}</span>}
          {status && <span role="status">{status}</span>}
        </div>
        {previewReasons.length > 0 && <ul>{previewReasons.map((reason) => <li key={reason}>{reason}</li>)}</ul>}
        <div className="rule-library">
          {rules.map((rule) => (
            <article key={rule.id}>
              <div><strong>{rule.name}</strong><small>v{rule.version} · {rule.conditions.length} {t("conditions")}</small></div>
              <span>{rule.enabled ? t("Enabled") : t("Disabled")}</span>
              <button className="button button--secondary" onClick={() => { setRuleName(rule.name); setConditions(rule.conditions); setMatchAll(rule.match_all); }} type="button">{t("Edit as next version")}</button>
              <button className="button button--secondary" onClick={() => void desktopApi.deleteLocalRule(rule.id).then(refreshLocalState)} type="button">{t("Delete")}</button>
            </article>
          ))}
        </div>
      </section>
    </section>
  );
}

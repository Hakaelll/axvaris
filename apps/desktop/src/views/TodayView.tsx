import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { useI18n } from "../i18n";
import { desktopApi } from "#backend";
import type {
  DashboardSnapshot,
  LocalNotificationRecord,
  LocalRule,
  LocalRuleAction,
  LocalRuleCondition,
  GeoDatabaseStatus,
  TodayBootstrap,
  TodayCardState,
  TodayChangeExample,
  TodayChangeKind,
  TodayEvidenceTarget,
  TrafficTrendPoint,
} from "../types";
import { Icon } from "../components/Icon";
import { TodaySessionStrip } from "../components/TodaySessionStrip";
import { TodayComparison } from "../components/TodayComparison";
import { UnlocatedTargets } from "../components/UnlocatedTargets";
import {
  usePresentation,
  type SensitiveKind,
} from "../privacy/PresentationMode";

interface TodayViewProps {
  snapshot: DashboardSnapshot;
  geoStatus: GeoDatabaseStatus | null;
  trafficTrend: TrafficTrendPoint[];
  onOpenEvidence: (
    target: TodayEvidenceTarget,
    connectionId: string | null,
    query: string,
  ) => void;
}

interface DecisionCard {
  id: string;
  category: string;
  scope: "device" | "application" | "profile" | "health";
  title: string;
  detail: string;
  connectionId: string | null;
  query: string;
  target: TodayEvidenceTarget;
  sensitiveKind: SensitiveKind | null;
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

function conditionFrom(
  field: RuleField,
  value: string,
): LocalRuleCondition | null {
  const cleaned = value.trim();
  if (field === "first_seen") {
    return { field, value: cleaned !== "false" };
  }
  if (
    field === "asn" ||
    field === "port" ||
    field === "minimum_bytes" ||
    field === "minimum_duration_ms"
  ) {
    const parsed = Number(cleaned);
    return Number.isFinite(parsed) && parsed >= 0
      ? ({ field, value: parsed } as LocalRuleCondition)
      : null;
  }
  if (!cleaned) return null;
  return { field, value: cleaned } as LocalRuleCondition;
}

function ruleSensitiveKind(field: RuleField): SensitiveKind | null {
  if (field === "process") return "process";
  if (field === "domain") return "domain";
  if (field === "ip" || field === "cidr") return "public_ip";
  if (field === "network_profile") return "profile";
  return null;
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

const categoryLabels: Record<TodayChangeKind, string> = {
  process: "New network process",
  application_destination: "New destination for application",
  domain: "New domain",
  asn: "New network owner hint",
  country: "New country",
  executable_revision: "New executable revision",
  network_profile: "Network profile change",
  listener_added: "Listener added",
  listener_removed: "Listener removed",
  listener_widened: "Listener scope widened",
  provider_degradation: "Capability degradation",
};

function scopeFor(kind: TodayChangeKind): DecisionCard["scope"] {
  if (kind === "application_destination" || kind === "executable_revision") {
    return "application";
  }
  if (
    kind === "domain" ||
    kind === "asn" ||
    kind === "country" ||
    kind === "network_profile"
  ) {
    return "profile";
  }
  if (kind === "provider_degradation") return "health";
  return "device";
}

function sensitiveKindFor(kind: TodayChangeKind): SensitiveKind | null {
  if (kind === "process") return "process";
  if (kind === "application_destination" || kind === "domain") return "domain";
  if (kind === "network_profile") return "profile";
  if (
    kind === "listener_added" ||
    kind === "listener_removed" ||
    kind === "listener_widened"
  )
    return "local_ip";
  return null;
}

function detailFor(
  t: ReturnType<typeof useI18n>,
  example: TodayChangeExample,
  count: number,
): string {
  const base = (() => {
    switch (example.kind) {
      case "process":
        return t("First observed opening a network connection on this device.");
      case "application_destination":
        return t(
          "New for {process}; open the retained evidence to inspect source and quality.",
          {
            process: example.secondary ?? t("this application"),
          },
        );
      case "domain":
        return t(
          "First observed domain in the complete retained local baseline.",
        );
      case "asn":
        return t("ASN is local dataset context, not a security verdict.");
      case "country":
        return t(
          "First observed country estimate in retained evidence; location is approximate.",
        );
      case "executable_revision":
        return t(
          "This application revision was not present in retained network evidence.",
        );
      case "network_profile":
        return t(
          "The pseudonymous local network profile changed during this session.",
        );
      case "listener_added":
        return t(
          "A local listening endpoint appeared; review bind scope and firewall evidence.",
        );
      case "listener_removed":
        return t(
          "A listening endpoint from the previous snapshot is no longer present.",
        );
      case "listener_widened":
        return t("A listener changed to a broader bind scope.");
      case "provider_degradation":
        return (
          example.secondary ??
          t("A capture or enrichment capability is degraded.")
        );
    }
  })();
  return count > 1
    ? `${base} ${t("{count} in this category.", { count })}`
    : base;
}

export function TodayView({
  snapshot,
  geoStatus,
  trafficTrend,
  onOpenEvidence,
}: TodayViewProps) {
  const t = useI18n();
  const { display, displayText } = usePresentation();
  const [bootstrap, setBootstrap] = useState<TodayBootstrap | null>(null);
  const refreshSequence = useRef(0);
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

  const refreshLocalState = useCallback(async () => {
    const sequence = refreshSequence.current + 1;
    refreshSequence.current = sequence;
    try {
      const next = await desktopApi.getTodayBootstrap();
      if (refreshSequence.current !== sequence) return;
      setBootstrap(next);
      setStatus(null);
    } catch (error: unknown) {
      if (refreshSequence.current !== sequence) return;
      setStatus(error instanceof Error ? error.message : String(error));
    }
  }, []);

  useEffect(() => {
    let secondFrame = 0;
    const firstFrame = window.requestAnimationFrame(() => {
      secondFrame = window.requestAnimationFrame(() => {
        void refreshLocalState();
      });
    });
    return () => {
      window.cancelAnimationFrame(firstFrame);
      window.cancelAnimationFrame(secondFrame);
      refreshSequence.current += 1;
    };
  }, [refreshLocalState, snapshot.session?.id]);

  const cards = useMemo(() => {
    return (bootstrap?.changes ?? []).flatMap((group) =>
      group.examples.map(
        (example): DecisionCard => ({
          id: example.id,
          category: t(categoryLabels[example.kind]),
          scope: scopeFor(example.kind),
          title: example.primary,
          detail: detailFor(t, example, group.count),
          connectionId: example.connection_id,
          query: example.query ?? "",
          target: example.target,
          sensitiveKind: sensitiveKindFor(example.kind),
        }),
      ),
    );
  }, [bootstrap?.changes, t]);

  const states = bootstrap?.card_states ?? [];
  const profiles = bootstrap?.profiles ?? [];
  const rules = bootstrap?.rules ?? [];
  const notifications = bootstrap?.notifications ?? [];

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
    setBootstrap((currentBootstrap) =>
      currentBootstrap
        ? {
            ...currentBootstrap,
            card_states: [
              next,
              ...currentBootstrap.card_states.filter((item) => item.id !== id),
            ],
          }
        : currentBootstrap,
    );
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
    setPreviewReasons(
      [...new Set(matches.flatMap((match) => match.reasons))].slice(0, 4),
    );
    setStatus(t("Dry-run completed locally."));
  };

  const saveRule = async () => {
    const draft = draftRule();
    if (draft.conditions.length === 0 || draft.actions.length === 0) {
      setStatus(t("A rule needs a condition and at least one safe action."));
      return;
    }
    const signature = JSON.stringify([
      draft.match_all,
      draft.conditions,
      draft.actions,
    ]);
    const duplicate = rules.find(
      (rule) =>
        rule.id !== draft.id &&
        JSON.stringify([rule.match_all, rule.conditions, rule.actions]) ===
          signature,
    );
    if (duplicate) {
      setStatus(t("Duplicate rule: {name}", { name: duplicate.name }));
      return;
    }
    await desktopApi.saveLocalRule(draft);
    setRuleName("");
    setConditions([]);
    setPreviewCount(null);
    setStatus(
      t("Rule version {version} saved locally.", { version: draft.version }),
    );
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
          <p>
            {t(
              "Changes are observations with traceable evidence, never malware verdicts.",
            )}
          </p>
        </div>
        <div className="today-profile-summary">
          <strong>
            {profiles[0]?.display_name
              ? display("profile", profiles[0].display_name)
              : t("Current network profile")}
          </strong>
          <small>
            {t("{count} local profiles", { count: profiles.length })}
          </small>
        </div>
      </header>

      <TodaySessionStrip
        activeProfileId={bootstrap?.active_profile_id ?? null}
        profiles={profiles}
        snapshot={snapshot}
        trend={trafficTrend}
      />

      <div className="today-layout">
        <section className="today-cards" aria-labelledby="today-changes-title">
          <header>
            <div>
              <h2 id="today-changes-title">{t("Changes worth reviewing")}</h2>
              <p>
                {t(
                  "Open any card to reach its concrete connection evidence in one step.",
                )}
              </p>
            </div>
            <span>
              {t("{count} retained changes", {
                count: (bootstrap?.changes ?? []).reduce(
                  (total, group) => total + group.count,
                  0,
                ),
              })}
            </span>
          </header>
          <div className="today-card-grid">
            {visibleCards.map((card) => {
              const state = cardStateFor(states, card.id);
              return (
                <article
                  className={`today-card${state.pinned ? " is-pinned" : ""}`}
                  key={card.id}
                >
                  <header>
                    <span>{card.category}</span>
                    <small>{t(`scope_${card.scope}`)}</small>
                  </header>
                  <h3>
                    {card.sensitiveKind
                      ? display(card.sensitiveKind, card.title)
                      : card.title}
                  </h3>
                  <p>{displayText(card.detail)}</p>
                  <footer>
                    <button
                      className="button button--primary"
                      onClick={() =>
                        onOpenEvidence(
                          card.target,
                          card.connectionId,
                          card.query,
                        )
                      }
                      type="button"
                    >
                      {t("Open evidence")}
                    </button>
                    <button
                      aria-pressed={state.pinned}
                      className="button button--secondary"
                      onClick={() =>
                        void updateCard(card.id, { pinned: !state.pinned })
                      }
                      type="button"
                    >
                      {state.pinned ? t("Unpin") : t("Pin")}
                    </button>
                    <button
                      className="button button--secondary"
                      onClick={() =>
                        void updateCard(card.id, { dismissed: true })
                      }
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
                <p>
                  {t(
                    "Start capture or use Demo; provider limitations remain visible in Data & Health.",
                  )}
                </p>
              </div>
            )}
          </div>
        </section>

        <aside
          className="today-notifications"
          aria-labelledby="today-notifications-title"
        >
          <header>
            <h2 id="today-notifications-title">
              {t("Local notification inbox")}
            </h2>
            <span>
              {notifications.filter((item) => item.state === "unread").length}
            </span>
          </header>
          {notifications.slice(0, 12).map((item) => (
            <article key={item.id}>
              <div>
                <strong>{displayText(item.title)}</strong>
                <small>
                  {t("{count} grouped events", { count: item.count })}
                </small>
              </div>
              <p>{displayText(item.detail)}</p>
              <div className="notification-actions">
                <button
                  onClick={() =>
                    onOpenEvidence("live", item.evidence_ids[0] ?? null, "")
                  }
                  type="button"
                >
                  {t("Evidence")}
                </button>
                <button
                  onClick={() =>
                    void updateNotification(
                      item,
                      item.state === "pinned" ? "read" : "pinned",
                    )
                  }
                  type="button"
                >
                  {item.state === "pinned" ? t("Unpin") : t("Pin")}
                </button>
                <button
                  onClick={() =>
                    void updateNotification(
                      item,
                      "read",
                      Date.now() + 60 * 60 * 1000,
                    )
                  }
                  type="button"
                >
                  {t("Snooze 1 h")}
                </button>
                <button
                  onClick={() =>
                    void updateNotification(item, "resolved", null)
                  }
                  type="button"
                >
                  {t("Resolve")}
                </button>
              </div>
            </article>
          ))}
          {notifications.length === 0 && (
            <p className="list-empty">{t("No local notifications.")}</p>
          )}
        </aside>
      </div>

      <div className="today-evidence-grid">
        <TodayComparison
          comparison={bootstrap?.previous_session ?? null}
          onNavigate={onOpenEvidence}
        />
        <UnlocatedTargets
          geoIpInstalled={geoStatus?.city.installed ?? false}
          onNavigate={onOpenEvidence}
          snapshot={snapshot}
        />
      </div>

      <section className="rule-studio" aria-labelledby="rule-studio-title">
        <header>
          <div>
            <span className="eyebrow">{t("SAFE LOCAL AUTOMATION")}</span>
            <h2 id="rule-studio-title">{t("Rules and labels")}</h2>
            <p>
              {t(
                "Rules may tag, pin, add to an investigation or notify. They cannot block traffic, run commands or start packet capture.",
              )}
            </p>
          </div>
          <span>{t("{count} versioned rules", { count: rules.length })}</span>
        </header>
        <div className="rule-studio__editor">
          <label>
            <span>{t("Rule name")}</span>
            <input
              onChange={(event) => setRuleName(event.target.value)}
              value={ruleName}
            />
          </label>
          <label>
            <span>{t("Typed field")}</span>
            <select
              onChange={(event) => setField(event.target.value as RuleField)}
              value={field}
            >
              {ruleFields.map((item) => (
                <option key={item} value={item}>
                  {t(item)}
                </option>
              ))}
            </select>
          </label>
          <label>
            <span>{t("Value")}</span>
            <input
              onChange={(event) => setValue(event.target.value)}
              placeholder={
                field === "first_seen"
                  ? "true"
                  : field === "protocol"
                    ? "tcp"
                    : ""
              }
              value={value}
            />
          </label>
          <button
            className="button button--secondary"
            onClick={addCondition}
            type="button"
          >
            {t("Add condition")}
          </button>
        </div>
        <div className="rule-condition-list" aria-live="polite">
          {conditions.map((condition, index) => (
            <button
              key={`${condition.field}-${index}`}
              onClick={() =>
                setConditions((items) =>
                  items.filter((_, itemIndex) => itemIndex !== index),
                )
              }
              type="button"
            >
              <strong>{t(condition.field)}</strong>
              <span>
                {ruleSensitiveKind(condition.field)
                  ? display(
                      ruleSensitiveKind(condition.field)!,
                      String(condition.value),
                    )
                  : String(condition.value)}
              </span>
              <Icon name="close" />
            </button>
          ))}
          <label>
            <input
              checked={matchAll}
              onChange={(event) => setMatchAll(event.target.checked)}
              type="checkbox"
            />
            {t("Require every condition (AND)")}
          </label>
        </div>
        <fieldset className="rule-actions">
          <legend>{t("Safe actions")}</legend>
          <label>
            <input
              checked={notify}
              onChange={(event) => setNotify(event.target.checked)}
              type="checkbox"
            />
            {t("Notify locally")}
          </label>
          <label>
            <input
              checked={pin}
              onChange={(event) => setPin(event.target.checked)}
              type="checkbox"
            />
            {t("Pin matching evidence")}
          </label>
          <label>
            <span>{t("Tag")}</span>
            <input
              onChange={(event) => setTag(event.target.value)}
              value={tag}
            />
          </label>
        </fieldset>
        <div className="rule-studio__controls">
          <button
            className="button button--secondary"
            onClick={() => void previewRule()}
            type="button"
          >
            {t("Preview on retained history")}
          </button>
          <button
            className="button button--primary"
            onClick={() => void saveRule()}
            type="button"
          >
            {t("Save new version")}
          </button>
          {previewCount !== null && (
            <span>{t("{count} dry-run matches", { count: previewCount })}</span>
          )}
          {status && <span role="status">{status}</span>}
        </div>
        {previewReasons.length > 0 && (
          <ul>
            {previewReasons.map((reason) => (
              <li key={reason}>{reason}</li>
            ))}
          </ul>
        )}
        <div className="rule-library">
          {rules.map((rule) => (
            <article key={rule.id}>
              <div>
                <strong>{rule.name}</strong>
                <small>
                  v{rule.version} · {rule.conditions.length} {t("conditions")}
                </small>
              </div>
              <span>{rule.enabled ? t("Enabled") : t("Disabled")}</span>
              <button
                className="button button--secondary"
                onClick={() => {
                  setRuleName(rule.name);
                  setConditions(rule.conditions);
                  setMatchAll(rule.match_all);
                }}
                type="button"
              >
                {t("Edit as next version")}
              </button>
              <button
                className="button button--secondary"
                onClick={() =>
                  void desktopApi
                    .deleteLocalRule(rule.id)
                    .then(refreshLocalState)
                }
                type="button"
              >
                {t("Delete")}
              </button>
            </article>
          ))}
        </div>
      </section>
    </section>
  );
}

import { useMemo, useState } from "react";
import { useI18n } from "../i18n";
import {
  parseQuery,
  serializeQueryExpression,
  serializeQueryTerm,
  type QueryExpression,
} from "../lib/filter";
import type { Connection } from "../types";
import { Icon } from "./Icon";

interface QueryBuilderProps {
  query: string;
  connections: Connection[];
  rangeLabel: string;
  onChange: (query: string) => void;
}

const fields = [
  "process",
  "domain",
  "ip",
  "cidr",
  "asn",
  "country",
  "protocol",
  "port",
  "direction",
  "state",
  "firstseen",
  "scope",
  "geo",
  "source",
] as const;

function readHistory(): string[] {
  try {
    const value = JSON.parse(localStorage.getItem("sabicheiro:query-history:v1") ?? "[]");
    return Array.isArray(value) ? value.filter((item): item is string => typeof item === "string").slice(0, 20) : [];
  } catch {
    return [];
  }
}

function removeTerm(expression: QueryExpression, target: number): QueryExpression | null {
  let cursor = 0;
  const visit = (item: QueryExpression): QueryExpression | null => {
    if (item.kind === "term") {
      const remove = cursor === target;
      cursor += 1;
      return remove ? null : item;
    }
    if (item.kind === "not") {
      const child = visit(item.item);
      return child ? { kind: "not", item: child } : null;
    }
    const items = item.items.map(visit).filter((child): child is QueryExpression => child !== null);
    if (items.length === 0) return null;
    if (items.length === 1) return items[0]!;
    return { kind: item.kind, items };
  };
  return visit(expression);
}

function quote(value: string): string {
  return /[\s()]/.test(value) ? `"${value}"` : value;
}

export function QueryBuilder({ query, connections, rangeLabel, onChange }: QueryBuilderProps) {
  const t = useI18n();
  const parsed = useMemo(() => parseQuery(query), [query]);
  const [field, setField] = useState<(typeof fields)[number]>("process");
  const [value, setValue] = useState("");
  const [negated, setNegated] = useState(false);
  const [join, setJoin] = useState<"AND" | "OR">("AND");
  const [history, setHistory] = useState(readHistory);

  const suggestions = useMemo(() => {
    const values = new Set<string>();
    for (const connection of connections) {
      if (field === "process") values.add(connection.process.name);
      if (field === "domain" && connection.domain) values.add(connection.domain);
      if (field === "ip") values.add(connection.key.remote.address);
      if (field === "asn" && connection.asn) values.add(String(connection.asn.number));
      if (field === "country" && connection.geo?.country_code) values.add(connection.geo.country_code);
      if (field === "protocol") values.add(connection.protocol);
      if (field === "port") values.add(String(connection.key.remote.port));
      if (field === "direction") values.add(connection.direction);
      if (field === "state") values.add(connection.state);
      if (field === "scope") values.add(connection.key.remote.scope);
      if (field === "source") values.add(connection.evidence.observation_source);
    }
    if (field === "firstseen") return ["true", "false"];
    return [...values].sort().slice(0, 200);
  }, [connections, field]);

  const remember = (next: string) => {
    if (!next.trim()) return;
    const nextHistory = [next, ...history.filter((item) => item !== next)].slice(0, 20);
    setHistory(nextHistory);
    localStorage.setItem("sabicheiro:query-history:v1", JSON.stringify(nextHistory));
  };

  const add = (asGroup: boolean) => {
    const cleaned = value.trim();
    if (!cleaned) return;
    const term = `${negated ? "-" : ""}${field}:${quote(cleaned)}`;
    const next = query.trim()
      ? `${asGroup ? `(${query.trim()})` : query.trim()} ${join} ${asGroup ? `(${term})` : term}`
      : term;
    onChange(next);
    remember(next);
    setValue("");
  };

  const remove = (index: number) => {
    if (!parsed.expression) return;
    const next = removeTerm(parsed.expression, index);
    onChange(next ? serializeQueryExpression(next) : "");
  };

  return (
    <section className="query-builder" aria-labelledby="query-builder-title">
      <header>
        <div><span className="eyebrow">{t("LOCAL QUERY BUILDER")}</span><h2 id="query-builder-title">{t("Build an evidence query")}</h2><p>{t("Fields, operators and observed values are completed entirely on this device.")}</p></div>
        <code>{rangeLabel}</code>
      </header>
      <div className="query-builder__text">
        <Icon name="search" />
        <input aria-label={t("Build an evidence query")} aria-invalid={Boolean(parsed.error)} onChange={(event) => onChange(event.target.value)} onKeyDown={(event) => event.key === "Enter" && remember(query)} value={query} />
        <button disabled={!parsed.expression || Boolean(parsed.error)} onClick={() => parsed.expression && onChange(serializeQueryExpression(parsed.expression))} type="button">{t("Canonicalize")}</button>
      </div>
      {parsed.error && <p className="query-builder__error" role="alert">{parsed.error}</p>}
      <div className="query-builder__chips" aria-label={t("Editable query chips")}>
        {parsed.terms.map((term, index) => (
          <button key={`${serializeQueryTerm(term)}-${index}`} onClick={() => remove(index)} title={t("Remove this condition")} type="button">
            {term.negated && <em>{t("NOT")}</em>}<strong>{term.field ?? t("text")}</strong><span>{term.value}</span><Icon name="close" />
          </button>
        ))}
        {parsed.terms.length === 0 && <span>{t("No conditions yet")}</span>}
      </div>
      <div className="query-builder__editor">
        <label><span>{t("Join")}</span><select onChange={(event) => setJoin(event.target.value as "AND" | "OR")} value={join}><option>AND</option><option>OR</option></select></label>
        <label><span>{t("Field")}</span><select onChange={(event) => setField(event.target.value as (typeof fields)[number])} value={field}>{fields.map((item) => <option key={item}>{item}</option>)}</select></label>
        <label className="query-builder__value"><span>{t("Observed value")}</span><input list="observed-query-values" onChange={(event) => setValue(event.target.value)} value={value} /><datalist id="observed-query-values">{suggestions.map((item) => <option key={item} value={item} />)}</datalist></label>
        <label className="query-builder__negate"><input checked={negated} onChange={(event) => setNegated(event.target.checked)} type="checkbox" />{t("Negate")}</label>
        <button className="button button--primary" disabled={!value.trim()} onClick={() => add(false)} type="button">{t("Add chip")}</button>
        <button className="button button--secondary" disabled={!value.trim()} onClick={() => add(true)} type="button">{t("Add grouped")}</button>
      </div>
      <footer>
        <div><strong>{t("Meaning")}</strong><span>{parsed.error ? t("Fix the positional error before applying this query.") : t("AND requires every condition; OR accepts either group. Negation excludes matching evidence.")}</span></div>
        <label><span>{t("Local history")}</span><select onChange={(event) => onChange(event.target.value)} value=""><option value="">{t("Choose a previous query")}</option>{history.map((item) => <option key={item} value={item}>{item}</option>)}</select></label>
      </footer>
    </section>
  );
}

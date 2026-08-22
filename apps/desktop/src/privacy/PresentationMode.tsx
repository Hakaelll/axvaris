import {
  createContext,
  useCallback,
  useContext,
  useLayoutEffect,
  useMemo,
  useRef,
  type ReactNode,
} from "react";
import type {
  AppSettings,
  DashboardSnapshot,
  NetworkInterface,
} from "../types";

export type SensitiveKind =
  | "public_ip"
  | "local_ip"
  | "domain"
  | "process"
  | "path"
  | "user"
  | "gateway"
  | "profile"
  | "interface"
  | "generic";

interface KnownSecret {
  kind: SensitiveKind;
  value: string;
}

const labels: Record<SensitiveKind, string> = {
  public_ip: "Public IP",
  local_ip: "Local IP",
  domain: "domain",
  process: "Application",
  path: "Executable path",
  user: "User",
  gateway: "Gateway",
  profile: "Network",
  interface: "Interface",
  generic: "Sensitive value",
};

class StablePresentationRedactor {
  private readonly forward = new Map<string, string>();
  private readonly reverse = new Map<string, string>();
  private readonly counts = new Map<SensitiveKind, number>();
  private knownSecrets: KnownSecret[] = [];

  setKnownSecrets(secrets: KnownSecret[]) {
    this.knownSecrets = [...secrets]
      .filter(({ value }) => value.trim().length >= 2)
      .sort((left, right) => right.value.length - left.value.length);
  }

  redact(kind: SensitiveKind, value: string | null | undefined): string {
    if (!value) return value ?? "";
    const key = `${kind}\0${value}`;
    const existing = this.forward.get(key);
    if (existing) return existing;
    const next = (this.counts.get(kind) ?? 0) + 1;
    this.counts.set(kind, next);
    const pseudonym =
      kind === "domain" ? `domain-${next}.invalid` : `${labels[kind]} ${next}`;
    this.forward.set(key, pseudonym);
    this.reverse.set(pseudonym, value);
    return pseudonym;
  }

  redactText(value: string): string {
    let redacted = value;
    const protectedPseudonyms = [...this.reverse.keys()].sort(
      (left, right) => right.length - left.length,
    );
    for (const [index, pseudonym] of protectedPseudonyms.entries()) {
      redacted = redacted.split(pseudonym).join(`\u{e000}${index}\u{e001}`);
    }
    for (const secret of this.knownSecrets) {
      if (redacted.includes(secret.value)) {
        redacted = redacted
          .split(secret.value)
          .join(this.redact(secret.kind, secret.value));
      }
    }
    redacted = redacted.replace(/\b(?:\d{1,3}\.){3}\d{1,3}\b/g, (match) =>
      this.redact("public_ip", match),
    );
    redacted = redacted.replace(
      /\b(?:[0-9a-f]{1,4}:){2,}[0-9a-f:]*\b/gi,
      (match) => this.redact("public_ip", match),
    );
    redacted = redacted.replace(
      /\b(?:[a-z0-9-]+\.)+[a-z]{2,63}\b/gi,
      (match) =>
        this.reverse.has(match) ? match : this.redact("domain", match),
    );
    redacted = redacted.replace(
      /(?:[a-z]:\\|\/(?:home|users)\/)[^\s"'<>]+/gi,
      (match) => this.redact("path", match),
    );
    return redacted.replace(
      /\u{e000}(\d+)\u{e001}/gu,
      (_match, index: string) => protectedPseudonyms[Number(index)] ?? "",
    );
  }

  restoreText(value: string): string {
    let restored = value;
    for (const [pseudonym, original] of [...this.reverse.entries()].sort(
      (left, right) => right[0].length - left[0].length,
    )) {
      restored = restored.split(pseudonym).join(original);
    }
    return restored;
  }
}

interface PresentationContextValue {
  active: boolean;
  display: (kind: SensitiveKind, value: string | null | undefined) => string;
  displayText: (value: string) => string;
  restoreText: (value: string) => string;
}

const PresentationContext = createContext<PresentationContextValue>({
  active: false,
  display: (_kind, value) => value ?? "",
  displayText: (value) => value,
  restoreText: (value) => value,
});

function collectKnownSecrets(
  snapshot: DashboardSnapshot,
  settings: AppSettings,
  interfaces: NetworkInterface[],
): KnownSecret[] {
  const secrets: KnownSecret[] = [];
  const add = (kind: SensitiveKind, value: string | null | undefined) => {
    if (value?.trim()) secrets.push({ kind, value });
  };
  for (const connection of snapshot.connections) {
    add("process", connection.process.name);
    add("process", connection.process.executable);
    add("path", connection.process.executable_path);
    add("user", connection.process.user_id);
    add("domain", connection.domain);
    add("domain", connection.hostname);
    add("local_ip", connection.key.local.address);
    add(
      connection.key.remote.scope === "global" ? "public_ip" : "local_ip",
      connection.key.remote.address,
    );
  }
  for (const networkInterface of interfaces) {
    add("interface", networkInterface.id);
    add("interface", networkInterface.name);
    for (const address of networkInterface.addresses) add("local_ip", address);
  }
  add("interface", settings.selected_interface);
  return secrets;
}

function redactDom(
  root: HTMLElement,
  redactor: { redactText: (value: string) => string },
) {
  const walker = document.createTreeWalker(root, NodeFilter.SHOW_TEXT);
  const textNodes: Text[] = [];
  while (walker.nextNode()) textNodes.push(walker.currentNode as Text);
  for (const node of textNodes) {
    const parent = node.parentElement;
    if (!parent || parent.closest("script, style, [data-presentation-safe]"))
      continue;
    const original = node.nodeValue ?? "";
    const redacted = redactor.redactText(original);
    if (redacted !== original) node.nodeValue = redacted;
  }
  for (const element of root.querySelectorAll<HTMLElement>(
    "[title], [aria-label]",
  )) {
    for (const attribute of ["title", "aria-label"] as const) {
      const original = element.getAttribute(attribute);
      if (!original) continue;
      const redacted = redactor.redactText(original);
      if (redacted !== original) element.setAttribute(attribute, redacted);
    }
  }
}

interface PresentationProviderProps {
  active: boolean;
  snapshot: DashboardSnapshot;
  settings: AppSettings;
  interfaces: NetworkInterface[];
  children: ReactNode;
}

export function PresentationProvider({
  active,
  snapshot,
  settings,
  interfaces,
  children,
}: PresentationProviderProps) {
  const redactorRef = useRef(new StablePresentationRedactor());
  redactorRef.current.setKnownSecrets(
    collectKnownSecrets(snapshot, settings, interfaces),
  );
  const value = useMemo<PresentationContextValue>(
    () => ({
      active,
      display: (kind, sensitiveValue) =>
        active
          ? redactorRef.current.redact(kind, sensitiveValue)
          : (sensitiveValue ?? ""),
      displayText: (text) =>
        active ? redactorRef.current.redactText(text) : text,
      restoreText: (text) =>
        active ? redactorRef.current.restoreText(text) : text,
    }),
    [active],
  );
  return (
    <PresentationContext.Provider value={value}>
      {children}
    </PresentationContext.Provider>
  );
}

export function PresentationBoundary({
  active,
  children,
}: {
  active: boolean;
  children: ReactNode;
}) {
  const rootRef = useRef<HTMLDivElement>(null);
  const { displayText } = useContext(PresentationContext);
  const redact = useCallback(
    (root: HTMLElement) => {
      if (!active) return;
      // `displayText` closes over the same stable redactor used by explicit values.
      redactDom(root, { redactText: displayText });
    },
    [active, displayText],
  );

  useLayoutEffect(() => {
    const root = rootRef.current;
    if (!active || !root) return;
    redact(root);
    const options: MutationObserverInit = {
      subtree: true,
      childList: true,
      characterData: true,
      attributes: true,
      attributeFilter: ["title", "aria-label"],
    };
    const observer = new MutationObserver((mutations) => {
      observer.disconnect();
      const targets = new Set<HTMLElement>();
      for (const mutation of mutations) {
        if (mutation.type === "childList") {
          for (const node of mutation.addedNodes) {
            if (node instanceof HTMLElement) targets.add(node);
            else if (node.parentElement) targets.add(node.parentElement);
          }
        } else if (mutation.target instanceof HTMLElement) {
          targets.add(mutation.target);
        } else if (mutation.target.parentElement) {
          targets.add(mutation.target.parentElement);
        }
      }
      for (const target of targets) redact(target);
      observer.observe(root, options);
    });
    observer.observe(root, options);
    return () => observer.disconnect();
  }, [active, redact]);

  return (
    <div className="presentation-boundary" ref={rootRef}>
      {children}
    </div>
  );
}

export function usePresentation() {
  return useContext(PresentationContext);
}

export function SensitiveText({
  kind,
  value,
}: {
  kind: SensitiveKind;
  value: string | null | undefined;
}) {
  const { display } = usePresentation();
  return <>{display(kind, value)}</>;
}

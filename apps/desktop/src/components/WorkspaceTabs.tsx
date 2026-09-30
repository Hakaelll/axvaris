import { useI18n } from "../i18n";
import type { ConnectionWorkspace } from "../types";

interface WorkspaceTabsProps {
  active: ConnectionWorkspace;
  counts: Record<ConnectionWorkspace, number>;
  onChange: (workspace: ConnectionWorkspace) => void;
}

const tabs: { id: ConnectionWorkspace; label: string }[] = [
  { id: "remote", label: "Remote" },
  { id: "local", label: "Devices" },
  { id: "listeners", label: "Listeners" },
];

export function WorkspaceTabs({
  active,
  counts,
  onChange,
}: WorkspaceTabsProps) {
  const t = useI18n();
  return (
    <div
      aria-label={t("Connection workspace")}
      className="workspace-tabs"
      role="tablist"
    >
      {tabs.map((tab) => (
        <button
          aria-selected={active === tab.id}
          className={active === tab.id ? "is-active" : undefined}
          key={tab.id}
          onClick={() => onChange(tab.id)}
          role="tab"
          type="button"
        >
          {t(tab.label)} {tab.id !== "local" && <span>{counts[tab.id]}</span>}
        </button>
      ))}
    </div>
  );
}

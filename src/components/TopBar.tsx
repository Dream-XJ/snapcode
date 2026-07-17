import { History, Settings as SettingsIcon, type LucideIcon } from "lucide-react";

import { cn, statusMeta } from "@/lib/utils";
import type { ListenerState, Tab } from "@/types";

interface TopBarProps {
  status: ListenerState | null;
  tab: Tab;
  onTabChange: (tab: Tab) => void;
}

const TABS: { value: Tab; label: string; icon: LucideIcon }[] = [
  { value: "history", label: "历史", icon: History },
  { value: "settings", label: "设置", icon: SettingsIcon },
];

export function TopBar({ status, tab, onTabChange }: TopBarProps) {
  const meta = statusMeta(status?.state ?? "starting");
  return (
    <header className="flex h-12 shrink-0 items-center justify-between border-b px-4">
      <div className="flex items-center gap-2.5">
        <span className="text-sm font-semibold tracking-tight">SnapCode</span>
        <span
          className="flex items-center gap-1.5 text-xs text-muted-foreground"
          title={status?.message ?? meta.text}
        >
          <span className={cn("h-1.5 w-1.5 rounded-full", meta.dot)} />
          {meta.text}
        </span>
      </div>
      <nav className="flex rounded-lg bg-muted p-0.5">
        {TABS.map((t) => {
          const Icon = t.icon;
          const active = tab === t.value;
          return (
            <button
              key={t.value}
              type="button"
              onClick={() => onTabChange(t.value)}
              className={cn(
                "flex h-7 items-center gap-1.5 rounded-md px-3 text-xs transition-colors",
                active
                  ? "bg-background font-medium shadow-sm"
                  : "text-muted-foreground hover:text-foreground",
              )}
            >
              <Icon className="h-3.5 w-3.5" />
              {t.label}
            </button>
          );
        })}
      </nav>
    </header>
  );
}

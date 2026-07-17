import { History, Settings as SettingsIcon, type LucideIcon } from "lucide-react";

import { useI18n, type MessageKey } from "@/lib/i18n";
import { cn, statusMeta } from "@/lib/utils";
import type { ListenerState, Tab } from "@/types";

interface TopBarProps {
  status: ListenerState | null;
  tab: Tab;
  onTabChange: (tab: Tab) => void;
}

const TABS: { value: Tab; labelKey: MessageKey; icon: LucideIcon }[] = [
  { value: "history", labelKey: "tabs.history", icon: History },
  { value: "settings", labelKey: "tabs.settings", icon: SettingsIcon },
];

export function TopBar({ status, tab, onTabChange }: TopBarProps) {
  const { t, lang } = useI18n();
  const meta = statusMeta(status?.state ?? "starting", lang);
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
        {TABS.map((tab_) => {
          const Icon = tab_.icon;
          const active = tab === tab_.value;
          return (
            <button
              key={tab_.value}
              type="button"
              onClick={() => onTabChange(tab_.value)}
              className={cn(
                "flex h-7 items-center gap-1.5 rounded-md px-3 text-xs transition-colors",
                active
                  ? "bg-background font-medium shadow-sm"
                  : "text-muted-foreground hover:text-foreground",
              )}
            >
              <Icon className="h-3.5 w-3.5" />
              {t(tab_.labelKey)}
            </button>
          );
        })}
      </nav>
    </header>
  );
}

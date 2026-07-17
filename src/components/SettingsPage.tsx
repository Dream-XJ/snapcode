import { useState, type ReactNode } from "react";
import { toast } from "sonner";
import { Monitor, Moon, Plus, RotateCcw, Sun, Trash2 } from "lucide-react";

import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Switch } from "@/components/ui/switch";
import { ShortcutRecorder } from "@/components/ShortcutRecorder";
import { toLang, useI18n, type Lang, type MessageKey } from "@/lib/i18n";
import {
  clearHistory,
  dumpNotifications,
  openNotificationSettings,
  retryListener,
  simulateNotification,
  updateSettings,
} from "@/lib/tauri";
import type { Theme } from "@/lib/theme";
import { cn, DEFAULT_AUMID, sourceDisplayName, statusMeta } from "@/lib/utils";
import type { ListenerState, Settings, ToastInfo } from "@/types";

interface SettingsPageProps {
  settings: Settings;
  onSettingsChange: (s: Settings) => void;
  status: ListenerState | null;
  shortcutError: string | null;
  theme: Theme;
  onThemeChange: (t: Theme) => void;
  onClearHistory: () => void;
}

function Section({ title, children }: { title: string; children: ReactNode }) {
  return (
    <section className="overflow-hidden rounded-xl border bg-card">
      <h2 className="border-b bg-muted/40 px-4 py-2 text-xs font-medium text-muted-foreground">
        {title}
      </h2>
      <div className="divide-y px-4">{children}</div>
    </section>
  );
}

function Row({ label, desc, control }: { label: string; desc?: string; control?: ReactNode }) {
  return (
    <div className="flex items-center justify-between gap-4 py-3">
      <div className="min-w-0">
        <p className="text-sm">{label}</p>
        {desc ? (
          <p className="mt-0.5 text-xs leading-relaxed text-muted-foreground">{desc}</p>
        ) : null}
      </div>
      {control ? <div className="shrink-0">{control}</div> : null}
    </div>
  );
}

const THEME_OPTIONS: { value: Theme; labelKey: MessageKey; icon: typeof Sun }[] = [
  { value: "light", labelKey: "theme.light", icon: Sun },
  { value: "dark", labelKey: "theme.dark", icon: Moon },
  { value: "system", labelKey: "theme.system", icon: Monitor },
];

/** 语言选项标签自命名，不随界面语言变化 */
const LANG_OPTIONS: { value: Lang; label: string }[] = [
  { value: "zh-CN", label: "中文" },
  { value: "en", label: "English" },
];

export function SettingsPage({
  settings,
  onSettingsChange,
  status,
  shortcutError,
  theme,
  onThemeChange,
  onClearHistory,
}: SettingsPageProps) {
  const { t, lang } = useI18n();
  const [newAumid, setNewAumid] = useState("");
  const [simText, setSimText] = useState("");
  const [simBusy, setSimBusy] = useState(false);
  const [dumpList, setDumpList] = useState<ToastInfo[] | null>(null);
  const [dumpBusy, setDumpBusy] = useState(false);
  const [dumpError, setDumpError] = useState<string | null>(null);

  /** 拉取当前系统 Toast 列表，用于诊断来源过滤 */
  async function handleDump() {
    if (dumpBusy) return;
    setDumpBusy(true);
    setDumpError(null);
    try {
      setDumpList(await dumpNotifications());
    } catch (e) {
      setDumpList(null);
      setDumpError(String(e));
    } finally {
      setDumpBusy(false);
    }
  }

  function addDumpedSource(aumid: string) {
    if (!aumid || settings.aumids.includes(aumid)) return;
    void save({ aumids: [...settings.aumids, aumid] });
  }

  /** 整体替换式保存：乐观更新，失败回滚并提示后端返回的错误字符串 */
  async function save(patch: Partial<Settings>): Promise<void> {
    const prev = settings;
    const next = { ...settings, ...patch };
    onSettingsChange(next);
    try {
      onSettingsChange(await updateSettings(next));
    } catch (e) {
      onSettingsChange(prev);
      toast.error(String(e));
      throw e;
    }
  }

  const meta = statusMeta(status?.state ?? "starting", lang);

  function addAumid() {
    const v = newAumid.trim();
    if (!v) return;
    if (settings.aumids.includes(v)) {
      toast.error(t("settings.sourceExists"));
      return;
    }
    setNewAumid("");
    void save({ aumids: [...settings.aumids, v] });
  }

  async function handleSimulate() {
    const text = simText.trim();
    if (!text || simBusy) return;
    setSimBusy(true);
    try {
      const code = await simulateNotification(text);
      if (code) toast.success(t("settings.simFound", { code }));
      else toast.info(t("settings.simNotFound"));
      setSimText("");
    } catch (e) {
      toast.error(String(e));
    } finally {
      setSimBusy(false);
    }
  }

  async function handleClear() {
    if (!window.confirm(t("settings.clearConfirm"))) return;
    try {
      await clearHistory();
      onClearHistory();
      toast.success(t("settings.cleared"));
    } catch (e) {
      toast.error(String(e));
    }
  }

  return (
    <div className="h-full space-y-3 overflow-y-auto p-3">
      <Section title={t("settings.sectionListener")}>
        <div className="py-3">
          <div className="flex items-center justify-between gap-3">
            <div className="flex min-w-0 items-center gap-2">
              <span className={cn("h-2 w-2 shrink-0 rounded-full", meta.dot)} />
              <p className="truncate text-sm">{meta.text}</p>
            </div>
            {status?.state === "access_denied" && (
              <div className="flex shrink-0 gap-2">
                <Button size="sm" variant="outline" onClick={() => void openNotificationSettings()}>
                  {t("common.openSystemSettings")}
                </Button>
                <Button size="sm" variant="ghost" onClick={() => void retryListener()}>
                  {t("common.retry")}
                </Button>
              </div>
            )}
            {status?.state === "error" && (
              <Button
                size="sm"
                variant="outline"
                className="shrink-0"
                onClick={() => void retryListener()}
              >
                {t("common.retry")}
              </Button>
            )}
          </div>
          <p className="mt-1.5 text-xs leading-relaxed text-muted-foreground">
            {status?.message ?? t("settings.listenerDefaultDesc")}
          </p>
        </div>
      </Section>

      <Section title={t("settings.sectionAppearance")}>
        <Row
          label={t("settings.theme")}
          control={
            <div className="flex rounded-lg bg-muted p-0.5">
              {THEME_OPTIONS.map((o) => {
                const Icon = o.icon;
                const active = theme === o.value;
                return (
                  <button
                    key={o.value}
                    type="button"
                    onClick={() => onThemeChange(o.value)}
                    className={cn(
                      "flex h-7 items-center gap-1 rounded-md px-2.5 text-xs transition-colors",
                      active
                        ? "bg-background shadow-sm"
                        : "text-muted-foreground hover:text-foreground",
                    )}
                  >
                    <Icon className="h-3.5 w-3.5" />
                    {t(o.labelKey)}
                  </button>
                );
              })}
            </div>
          }
        />
        <Row
          label={t("settings.language")}
          control={
            <div className="flex rounded-lg bg-muted p-0.5">
              {LANG_OPTIONS.map((o) => {
                const active = toLang(settings.language) === o.value;
                return (
                  <button
                    key={o.value}
                    type="button"
                    onClick={() => void save({ language: o.value })}
                    className={cn(
                      "flex h-7 items-center rounded-md px-2.5 text-xs transition-colors",
                      active
                        ? "bg-background shadow-sm"
                        : "text-muted-foreground hover:text-foreground",
                    )}
                  >
                    {o.label}
                  </button>
                );
              })}
            </div>
          }
        />
      </Section>

      <Section title={t("settings.sectionShortcut")}>
        <Row
          label={t("settings.shortcutLabel")}
          desc={t("settings.shortcutDesc")}
          control={
            <ShortcutRecorder
              value={settings.shortcut}
              error={shortcutError}
              onSave={(shortcut) => save({ shortcut })}
            />
          }
        />
      </Section>

      <Section title={t("settings.sectionBehavior")}>
        <Row
          label={t("settings.autoCopy")}
          desc={t("settings.autoCopyDesc")}
          control={
            <Switch
              checked={settings.auto_copy}
              onCheckedChange={(v) => void save({ auto_copy: v })}
            />
          }
        />
        <Row
          label={t("settings.autostart")}
          desc={t("settings.autostartDesc")}
          control={
            <Switch
              checked={settings.autostart}
              onCheckedChange={(v) => void save({ autostart: v })}
            />
          }
        />
      </Section>

      <Section title={t("settings.sectionSources")}>
        <div className="py-3">
          <p className="text-xs text-muted-foreground">{t("settings.sourcesDesc")}</p>
          <div className="mt-2 space-y-1">
            {settings.aumids.map((a) => (
              <div key={a} className="flex items-center gap-2 rounded-md py-1">
                <div className="min-w-0 flex-1">
                  <p className="text-sm">{sourceDisplayName(a, lang)}</p>
                  <p className="truncate font-mono text-[11px] text-muted-foreground">{a}</p>
                </div>
                <Button
                  variant="ghost"
                  size="icon"
                  className="h-7 w-7 shrink-0 text-muted-foreground hover:text-destructive"
                  title={t("common.remove")}
                  onClick={() => void save({ aumids: settings.aumids.filter((x) => x !== a) })}
                >
                  <Trash2 className="h-3.5 w-3.5" />
                </Button>
              </div>
            ))}
          </div>
          <div className="mt-2 flex gap-2">
            <Input
              value={newAumid}
              onChange={(e) => setNewAumid(e.target.value)}
              onKeyDown={(e) => {
                if (e.key === "Enter") addAumid();
              }}
              placeholder={t("settings.aumidPlaceholder")}
              className="h-8 font-mono text-xs"
            />
            <Button size="sm" variant="outline" className="h-8 shrink-0" onClick={addAumid}>
              {t("settings.add")}
            </Button>
          </div>
          {!settings.aumids.includes(DEFAULT_AUMID) && (
            <button
              type="button"
              onClick={() => void save({ aumids: [...settings.aumids, DEFAULT_AUMID] })}
              className="mt-2 flex items-center gap-1 text-xs text-primary hover:underline"
            >
              <RotateCcw className="h-3 w-3" />
              {t("settings.restoreDefaultSource")}
            </button>
          )}
        </div>
      </Section>

      <Section title={t("settings.sectionHistory")}>
        <Row
          label={t("settings.retention")}
          desc={t("settings.retentionDesc")}
          control={
            <select
              value={settings.retention_days}
              onChange={(e) => void save({ retention_days: Number(e.target.value) })}
              className="h-8 rounded-md border border-input bg-background px-2 text-sm shadow-sm focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring"
            >
              <option value={1}>{t("settings.retentionDay1")}</option>
              <option value={3}>{t("settings.retentionDays", { n: 3 })}</option>
              <option value={7}>{t("settings.retentionDays", { n: 7 })}</option>
              <option value={30}>{t("settings.retentionDays", { n: 30 })}</option>
              <option value={0}>{t("settings.retentionForever")}</option>
            </select>
          }
        />
        <Row
          label={t("settings.clearLabel")}
          desc={t("settings.clearDesc")}
          control={
            <Button
              size="sm"
              variant="outline"
              className="text-destructive hover:text-destructive"
              onClick={() => void handleClear()}
            >
              {t("settings.clearAll")}
            </Button>
          }
        />
      </Section>

      <Section title={t("settings.sectionDebug")}>
        <div className="space-y-2 py-3">
          <p className="text-xs text-muted-foreground">{t("settings.simDesc")}</p>
          <textarea
            value={simText}
            onChange={(e) => setSimText(e.target.value)}
            rows={3}
            placeholder={t("settings.simPlaceholder")}
            className="w-full resize-none rounded-md border border-input bg-transparent px-3 py-2 text-sm shadow-sm placeholder:text-muted-foreground focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring"
          />
          <div className="flex justify-end">
            <Button
              size="sm"
              variant="outline"
              disabled={!simText.trim() || simBusy}
              onClick={() => void handleSimulate()}
            >
              {t("settings.simulate")}
            </Button>
          </div>
        </div>
        <div className="space-y-2 py-3">
          <p className="text-xs text-muted-foreground">{t("settings.dumpDesc")}</p>
          <div className="flex justify-end">
            <Button
              size="sm"
              variant="outline"
              disabled={dumpBusy}
              onClick={() => void handleDump()}
            >
              {t("settings.dump")}
            </Button>
          </div>
          {dumpError ? <p className="text-xs text-destructive">{dumpError}</p> : null}
          {dumpList && dumpList.length === 0 ? (
            <p className="text-xs text-muted-foreground">{t("settings.dumpEmpty")}</p>
          ) : null}
          {dumpList?.map((info, i) => (
            <div key={i} className="space-y-1 rounded-md border p-2">
              <div className="flex items-start justify-between gap-2">
                <p className="break-all font-mono text-[11px] leading-relaxed text-muted-foreground">
                  {info.aumid || t("settings.dumpNoAumid")}
                </p>
                {info.aumid && !settings.aumids.includes(info.aumid) ? (
                  <Button
                    size="sm"
                    variant="ghost"
                    className="h-6 shrink-0 px-2 text-xs"
                    onClick={() => addDumpedSource(info.aumid)}
                  >
                    <Plus className="mr-1 h-3 w-3" />
                    {t("settings.dumpAdd")}
                  </Button>
                ) : null}
              </div>
              {info.title ? <p className="truncate text-xs">{info.title}</p> : null}
              {info.body ? (
                <p className="truncate text-xs text-muted-foreground">{info.body}</p>
              ) : null}
            </div>
          ))}
        </div>
      </Section>

      <Section title={t("settings.sectionAbout")}>
        <Row label={t("app.name")} desc={t("settings.aboutDesc")} />
        <Row label={t("settings.dataLabel")} desc={t("settings.dataDesc")} />
      </Section>
    </div>
  );
}

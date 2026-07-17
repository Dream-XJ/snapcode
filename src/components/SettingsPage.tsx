import { useState, type ReactNode } from "react";
import { toast } from "sonner";
import { Monitor, Moon, Plus, RotateCcw, Sun, Trash2 } from "lucide-react";

import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Switch } from "@/components/ui/switch";
import { ShortcutRecorder } from "@/components/ShortcutRecorder";
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

const THEME_OPTIONS: { value: Theme; label: string; icon: typeof Sun }[] = [
  { value: "light", label: "浅色", icon: Sun },
  { value: "dark", label: "深色", icon: Moon },
  { value: "system", label: "跟随系统", icon: Monitor },
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

  const meta = statusMeta(status?.state ?? "starting");

  function addAumid() {
    const v = newAumid.trim();
    if (!v) return;
    if (settings.aumids.includes(v)) {
      toast.error("该来源已存在");
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
      if (code) toast.success(`识别到验证码 ${code}`);
      else toast.info("未识别到验证码");
      setSimText("");
    } catch (e) {
      toast.error(String(e));
    } finally {
      setSimBusy(false);
    }
  }

  async function handleClear() {
    if (!window.confirm("确定要清空全部历史记录吗？此操作不可撤销。")) return;
    try {
      await clearHistory();
      onClearHistory();
      toast.success("历史记录已清空");
    } catch (e) {
      toast.error(String(e));
    }
  }

  return (
    <div className="h-full space-y-3 overflow-y-auto p-3">
      <Section title="通知监听">
        <div className="py-3">
          <div className="flex items-center justify-between gap-3">
            <div className="flex min-w-0 items-center gap-2">
              <span className={cn("h-2 w-2 shrink-0 rounded-full", meta.dot)} />
              <p className="truncate text-sm">{meta.text}</p>
            </div>
            {status?.state === "access_denied" && (
              <div className="flex shrink-0 gap-2">
                <Button size="sm" variant="outline" onClick={() => void openNotificationSettings()}>
                  打开系统设置
                </Button>
                <Button size="sm" variant="ghost" onClick={() => void retryListener()}>
                  重新检测
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
                重新检测
              </Button>
            )}
          </div>
          <p className="mt-1.5 text-xs leading-relaxed text-muted-foreground">
            {status?.message ?? "SnapCode 通过读取 Windows 通知识别短信验证码。"}
          </p>
        </div>
      </Section>

      <Section title="外观">
        <Row
          label="主题"
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
                    {o.label}
                  </button>
                );
              })}
            </div>
          }
        />
      </Section>

      <Section title="全局快捷键">
        <Row
          label="粘贴快捷键"
          desc="在任意应用中按下，即可粘贴最新验证码"
          control={
            <ShortcutRecorder
              value={settings.shortcut}
              error={shortcutError}
              onSave={(shortcut) => save({ shortcut })}
            />
          }
        />
      </Section>

      <Section title="行为">
        <Row
          label="自动复制"
          desc="识别到验证码后立即写入剪贴板"
          control={
            <Switch
              checked={settings.auto_copy}
              onCheckedChange={(v) => void save({ auto_copy: v })}
            />
          }
        />
        <Row
          label="开机自启"
          desc="登录 Windows 后自动启动 SnapCode"
          control={
            <Switch
              checked={settings.autostart}
              onCheckedChange={(v) => void save({ autostart: v })}
            />
          }
        />
      </Section>

      <Section title="通知来源">
        <div className="py-3">
          <p className="text-xs text-muted-foreground">仅监听以下应用（AUMID）的通知</p>
          <div className="mt-2 space-y-1">
            {settings.aumids.map((a) => (
              <div key={a} className="flex items-center gap-2 rounded-md py-1">
                <div className="min-w-0 flex-1">
                  <p className="text-sm">{sourceDisplayName(a)}</p>
                  <p className="truncate font-mono text-[11px] text-muted-foreground">{a}</p>
                </div>
                <Button
                  variant="ghost"
                  size="icon"
                  className="h-7 w-7 shrink-0 text-muted-foreground hover:text-destructive"
                  title="移除"
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
              placeholder="添加 AUMID…"
              className="h-8 font-mono text-xs"
            />
            <Button size="sm" variant="outline" className="h-8 shrink-0" onClick={addAumid}>
              添加
            </Button>
          </div>
          {!settings.aumids.includes(DEFAULT_AUMID) && (
            <button
              type="button"
              onClick={() => void save({ aumids: [...settings.aumids, DEFAULT_AUMID] })}
              className="mt-2 flex items-center gap-1 text-xs text-primary hover:underline"
            >
              <RotateCcw className="h-3 w-3" />
              恢复默认来源（手机连接）
            </button>
          )}
        </div>
      </Section>

      <Section title="历史记录">
        <Row
          label="保留策略"
          desc="过期记录将自动清理"
          control={
            <select
              value={settings.retention_days}
              onChange={(e) => void save({ retention_days: Number(e.target.value) })}
              className="h-8 rounded-md border border-input bg-background px-2 text-sm shadow-sm focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring"
            >
              <option value={1}>1 天</option>
              <option value={3}>3 天</option>
              <option value={7}>7 天</option>
              <option value={30}>30 天</option>
              <option value={0}>永久保留</option>
            </select>
          }
        />
        <Row
          label="清空历史"
          desc="删除全部已捕获的验证码记录"
          control={
            <Button
              size="sm"
              variant="outline"
              className="text-destructive hover:text-destructive"
              onClick={() => void handleClear()}
            >
              清空全部
            </Button>
          }
        />
      </Section>

      <Section title="调试">
        <div className="space-y-2 py-3">
          <p className="text-xs text-muted-foreground">
            模拟收到一条短信通知，走完整的识别与入库流程
          </p>
          <textarea
            value={simText}
            onChange={(e) => setSimText(e.target.value)}
            rows={3}
            placeholder="例如：【微信】您的验证码是 482913，5 分钟内有效。"
            className="w-full resize-none rounded-md border border-input bg-transparent px-3 py-2 text-sm shadow-sm placeholder:text-muted-foreground focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring"
          />
          <div className="flex justify-end">
            <Button
              size="sm"
              variant="outline"
              disabled={!simText.trim() || simBusy}
              onClick={() => void handleSimulate()}
            >
              模拟通知
            </Button>
          </div>
        </div>
        <div className="space-y-2 py-3">
          <p className="text-xs text-muted-foreground">
            抓取不到验证码时，列出当前系统中的 Toast 通知，确认「手机连接」的真实来源
            AUMID 并一键加入监听列表
          </p>
          <div className="flex justify-end">
            <Button
              size="sm"
              variant="outline"
              disabled={dumpBusy}
              onClick={() => void handleDump()}
            >
              列出系统通知
            </Button>
          </div>
          {dumpError ? <p className="text-xs text-destructive">{dumpError}</p> : null}
          {dumpList && dumpList.length === 0 ? (
            <p className="text-xs text-muted-foreground">当前没有系统通知</p>
          ) : null}
          {dumpList?.map((t, i) => (
            <div key={i} className="space-y-1 rounded-md border p-2">
              <div className="flex items-start justify-between gap-2">
                <p className="break-all font-mono text-[11px] leading-relaxed text-muted-foreground">
                  {t.aumid || "(无 AUMID)"}
                </p>
                {t.aumid && !settings.aumids.includes(t.aumid) ? (
                  <Button
                    size="sm"
                    variant="ghost"
                    className="h-6 shrink-0 px-2 text-xs"
                    onClick={() => addDumpedSource(t.aumid)}
                  >
                    <Plus className="mr-1 h-3 w-3" />
                    加为来源
                  </Button>
                ) : null}
              </div>
              {t.title ? <p className="truncate text-xs">{t.title}</p> : null}
              {t.body ? (
                <p className="truncate text-xs text-muted-foreground">{t.body}</p>
              ) : null}
            </div>
          ))}
        </div>
      </Section>

      <Section title="关于">
        <Row label="SnapCode 闪码" desc="v0.1.0 · Windows 短信验证码捕获工具" />
        <Row label="数据存储" desc="全部数据仅保存在本机，不会上传" />
      </Section>
    </div>
  );
}

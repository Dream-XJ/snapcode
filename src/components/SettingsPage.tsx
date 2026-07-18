import { useEffect, useState, type ReactNode } from "react";
import { toast } from "sonner";
import { ChevronDown, Monitor, Moon, Plus, RotateCcw, Sun, Trash2 } from "lucide-react";

import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Switch } from "@/components/ui/switch";
import { ShortcutRecorder } from "@/components/ShortcutRecorder";
import { toLang, useI18n, type Lang, type MessageKey } from "@/lib/i18n";
import {
  APP_VERSION,
  clearHistory,
  dumpNotifications,
  getEmailStatus,
  newEmailAccount,
  onEmailStatus,
  openNotificationSettings,
  retryListener,
  simulateNotification,
  testEmailConnection,
  updateSettings,
} from "@/lib/tauri";
import type { Theme } from "@/lib/theme";
import { cn, DEFAULT_AUMID, sourceDisplayName, statusMeta } from "@/lib/utils";
import type {
  EmailAccount,
  EmailAccountStatus,
  EmailProtocol,
  EmailStateName,
  ListenerState,
  Settings,
  ToastInfo,
} from "@/types";

interface SettingsPageProps {
  settings: Settings;
  onSettingsChange: (s: Settings) => void;
  status: ListenerState | null;
  shortcutError: string | null;
  theme: Theme;
  onThemeChange: (t: Theme) => void;
  onClearHistory: () => void;
  /** 手动检查更新（关于区按钮）；发现新版本由 App 弹窗，错误以 reject 抛出 */
  onCheckUpdate: () => Promise<void>;
}

function Section({
  title,
  action,
  children,
}: {
  title: string;
  /** 标题栏右侧操作区（如「添加账户」按钮） */
  action?: ReactNode;
  children: ReactNode;
}) {
  return (
    <section className="overflow-hidden rounded-xl border bg-card">
      <div className="flex items-center justify-between gap-2 border-b bg-muted/40 px-4 py-2">
        <h2 className="text-xs font-medium text-muted-foreground">{title}</h2>
        {action}
      </div>
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

/**
 * 单个邮箱账户的编辑表单。
 * 文本字段草稿本地管理（失焦才保存，避免每次击键都触发整体设置更新）；
 * 父组件必须带 key={account.id}，保证切换账户时草稿不串。
 */
function EmailAccountEditor({
  account,
  onSave,
}: {
  account: EmailAccount;
  onSave: (patch: Partial<EmailAccount>) => void;
}) {
  const { t } = useI18n();
  const [draft, setDraft] = useState({
    name: account.name,
    host: account.host,
    username: account.username,
    password: account.password,
  });
  const [portText, setPortText] = useState(String(account.port));
  const [testBusy, setTestBusy] = useState(false);

  // 持久化值外部变化（保存回写 / 其他途径更新）后重置草稿，避免旧草稿覆盖新值
  useEffect(() => {
    setDraft({
      name: account.name,
      host: account.host,
      username: account.username,
      password: account.password,
    });
    setPortText(String(account.port));
  }, [account]);

  /** 失焦保存：与已持久化值比较，未变化不发请求 */
  function blurSave(patch: Partial<EmailAccount>) {
    const next = { ...account, ...patch };
    if (JSON.stringify(next) === JSON.stringify(account)) return;
    onSave(patch);
  }

  function blurPort() {
    const port = Number(portText.trim());
    if (!Number.isInteger(port) || port <= 0 || port > 65535) {
      toast.error(t("settings.emailPortInvalid"));
      setPortText(String(account.port));
      return;
    }
    blurSave({ port });
  }

  /** 端口恰为另一协议的 TLS 默认端口（995↔993）时随协议联动切换，免去手动改端口 */
  function handleProtocol(protocol: EmailProtocol) {
    const patch: Partial<EmailAccount> = { protocol };
    if (protocol === "imap" && account.port === 995) patch.port = 993;
    else if (protocol === "pop3" && account.port === 993) patch.port = 995;
    if (patch.port) setPortText(String(patch.port));
    onSave(patch);
  }

  /** 测试连接用表单当前草稿值（可能尚未保存） */
  async function handleTest() {
    if (testBusy) return;
    setTestBusy(true);
    try {
      const cfg: EmailAccount = {
        ...account,
        ...draft,
        name: draft.name.trim(),
        host: draft.host.trim(),
        username: draft.username.trim(),
        port: Number(portText.trim()) || 0,
      };
      const n = await testEmailConnection(cfg);
      toast.success(t("settings.emailTestOk", { n }));
    } catch (e) {
      toast.error(String(e));
    } finally {
      setTestBusy(false);
    }
  }

  return (
    <div className="space-y-2">
      <div className="flex gap-2">
        <div className="min-w-0 flex-1">
          <p className="mb-1 text-xs text-muted-foreground">{t("settings.emailName")}</p>
          <Input
            value={draft.name}
            onChange={(e) => setDraft({ ...draft, name: e.target.value })}
            onBlur={() => blurSave({ name: draft.name.trim() })}
            placeholder={t("settings.emailNamePlaceholder")}
            className="h-8 text-xs"
          />
        </div>
        <div className="w-36 shrink-0">
          <p className="mb-1 text-xs text-muted-foreground">{t("settings.emailProtocol")}</p>
          <select
            value={account.protocol}
            onChange={(e) => handleProtocol(e.target.value as EmailProtocol)}
            className="h-8 w-full rounded-md border border-input bg-background px-2 text-sm shadow-sm focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring"
          >
            <option value="pop3">{t("settings.emailProtocolPop3")}</option>
            <option value="imap">{t("settings.emailProtocolImap")}</option>
          </select>
        </div>
      </div>
      <div className="flex gap-2">
        <div className="min-w-0 flex-1">
          <p className="mb-1 text-xs text-muted-foreground">{t("settings.emailHost")}</p>
          <Input
            value={draft.host}
            onChange={(e) => setDraft({ ...draft, host: e.target.value })}
            onBlur={() => blurSave({ host: draft.host.trim() })}
            placeholder={t("settings.emailHostPlaceholder")}
            className="h-8 font-mono text-xs"
          />
        </div>
        <div className="w-20 shrink-0">
          <p className="mb-1 text-xs text-muted-foreground">{t("settings.emailPort")}</p>
          <Input
            value={portText}
            onChange={(e) => setPortText(e.target.value)}
            onBlur={blurPort}
            className="h-8 font-mono text-xs"
          />
        </div>
      </div>
      <div>
        <p className="mb-1 text-xs text-muted-foreground">{t("settings.emailUser")}</p>
        <Input
          value={draft.username}
          onChange={(e) => setDraft({ ...draft, username: e.target.value })}
          onBlur={() => blurSave({ username: draft.username.trim() })}
          placeholder={t("settings.emailUserPlaceholder")}
          className="h-8 font-mono text-xs"
        />
      </div>
      <div>
        <p className="mb-1 text-xs text-muted-foreground">{t("settings.emailPass")}</p>
        <Input
          type="password"
          value={draft.password}
          onChange={(e) => setDraft({ ...draft, password: e.target.value })}
          onBlur={() => blurSave({ password: draft.password })}
          placeholder={t("settings.emailPassPlaceholder")}
          autoComplete="off"
          className="h-8 font-mono text-xs"
        />
      </div>
      <div className="flex items-center justify-between gap-3 pt-1">
        <div className="flex items-center gap-2">
          <p className="text-xs text-muted-foreground">{t("settings.emailInterval")}</p>
          <select
            value={account.poll_interval_secs}
            onChange={(e) => onSave({ poll_interval_secs: Number(e.target.value) })}
            className="h-8 rounded-md border border-input bg-background px-2 text-sm shadow-sm focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring"
          >
            <option value={30}>{t("settings.intervalSeconds", { n: 30 })}</option>
            <option value={60}>{t("settings.intervalMinutes", { n: 1 })}</option>
            <option value={120}>{t("settings.intervalMinutes", { n: 2 })}</option>
            <option value={300}>{t("settings.intervalMinutes", { n: 5 })}</option>
          </select>
        </div>
        <div className="flex items-center gap-2">
          <p className="text-xs text-muted-foreground">{t("settings.emailTls")}</p>
          <Switch checked={account.use_tls} onCheckedChange={(v) => onSave({ use_tls: v })} />
        </div>
      </div>
      <p className="text-[11px] leading-relaxed text-muted-foreground">
        {t("settings.emailIntervalHint")}
      </p>
      <div className="flex items-center justify-between gap-3 pt-1">
        <p className="text-xs leading-relaxed text-muted-foreground">
          {t("settings.emailBaselineHint")}
        </p>
        <Button
          size="sm"
          variant="outline"
          className="shrink-0"
          disabled={testBusy}
          onClick={() => void handleTest()}
        >
          {testBusy ? t("settings.emailTesting") : t("settings.emailTest")}
        </Button>
      </div>
    </div>
  );
}

export function SettingsPage({
  settings,
  onSettingsChange,
  status,
  shortcutError,
  theme,
  onThemeChange,
  onClearHistory,
  onCheckUpdate,
}: SettingsPageProps) {
  const { t, lang } = useI18n();
  const [newAumid, setNewAumid] = useState("");
  const [simText, setSimText] = useState("");
  const [simBusy, setSimBusy] = useState(false);
  const [dumpList, setDumpList] = useState<ToastInfo[] | null>(null);
  const [dumpBusy, setDumpBusy] = useState(false);
  const [dumpError, setDumpError] = useState<string | null>(null);
  const [checkBusy, setCheckBusy] = useState(false);
  /** 各邮箱账户的轮询状态（email-status 事件 payload，按 account_id 排序） */
  const [emailStatusList, setEmailStatusList] = useState<EmailAccountStatus[]>([]);
  /** 展开编辑表单的账户 id 集合；卡片默认收起 */
  const [expandedEmailIds, setExpandedEmailIds] = useState<ReadonlySet<string>>(new Set());

  // 订阅邮箱轮询状态（初始拉取 + email-status 事件）
  useEffect(() => {
    getEmailStatus()
      .then(setEmailStatusList)
      .catch(() => undefined);
    return onEmailStatus(setEmailStatusList);
  }, []);

  /** 账户列表整体替换保存 */
  function saveAccounts(accounts: EmailAccount[]) {
    void save({ email: { accounts } });
  }

  function updateAccount(id: string, patch: Partial<EmailAccount>) {
    saveAccounts(settings.email.accounts.map((a) => (a.id === id ? { ...a, ...patch } : a)));
  }

  function addAccount() {
    saveAccounts([...settings.email.accounts, newEmailAccount()]);
  }

  function removeAccount(id: string) {
    saveAccounts(settings.email.accounts.filter((a) => a.id !== id));
  }

  function toggleEmailExpanded(id: string) {
    setExpandedEmailIds((prev) => {
      const next = new Set(prev);
      if (next.has(id)) next.delete(id);
      else next.add(id);
      return next;
    });
  }

  /** 单账户状态点颜色与文案；无状态记录（后端从未上报）按未启用显示 */
  function emailMeta(state: EmailStateName | undefined) {
    switch (state) {
      case "running":
        return { dot: "bg-emerald-500", text: t("email.status.running") };
      case "paused":
        return { dot: "bg-amber-500", text: t("email.status.paused") };
      case "error":
        return { dot: "bg-red-500", text: t("email.status.error") };
      default:
        return { dot: "bg-zinc-400", text: t("email.status.disabled") };
    }
  }

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

  /** 手动检查更新：新版本/已最新由 App 处理，这里只兜底错误提示 */
  async function handleCheck() {
    if (checkBusy) return;
    setCheckBusy(true);
    try {
      await onCheckUpdate();
    } catch (e) {
      toast.error(t("update.checkFailed", { err: String(e) }));
    } finally {
      setCheckBusy(false);
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

      <Section
        title={t("settings.sectionEmail")}
        action={
          <Button size="sm" variant="outline" className="h-7" onClick={addAccount}>
            <Plus className="mr-1 h-3 w-3" />
            {t("settings.emailAdd")}
          </Button>
        }
      >
        <div className="space-y-2 py-3">
          <p className="text-xs leading-relaxed text-muted-foreground">
            {t("settings.emailDesc")}
          </p>
          {settings.email.accounts.map((account) => {
            const st = emailStatusList.find((s) => s.account_id === account.id);
            const meta = emailMeta(st?.state);
            const expanded = expandedEmailIds.has(account.id);
            return (
              <div key={account.id} className="rounded-lg border px-3">
                <div className="flex items-center gap-2 py-2.5">
                  <span
                    className={cn("h-2 w-2 shrink-0 rounded-full", meta.dot)}
                    title={meta.text}
                  />
                  <p className="min-w-0 truncate text-sm">
                    {account.name.trim() || account.username.trim() || t("settings.emailUntitled")}
                  </p>
                  <span className="shrink-0 rounded bg-muted px-1.5 py-0.5 text-[10px] font-medium uppercase text-muted-foreground">
                    {account.protocol}
                  </span>
                  <div className="flex-1" />
                  <Switch
                    checked={account.enabled}
                    onCheckedChange={(v) => updateAccount(account.id, { enabled: v })}
                  />
                  <Button
                    variant="ghost"
                    size="icon"
                    className="h-7 w-7 shrink-0 text-muted-foreground"
                    onClick={() => toggleEmailExpanded(account.id)}
                  >
                    <ChevronDown
                      className={cn("h-3.5 w-3.5 transition-transform", expanded && "rotate-180")}
                    />
                  </Button>
                  <Button
                    variant="ghost"
                    size="icon"
                    className="h-7 w-7 shrink-0 text-muted-foreground hover:text-destructive"
                    title={t("settings.emailDelete")}
                    onClick={() => removeAccount(account.id)}
                  >
                    <Trash2 className="h-3.5 w-3.5" />
                  </Button>
                </div>
                {st?.state === "error" && st.message ? (
                  <p className="pb-2 text-xs leading-relaxed text-destructive">{st.message}</p>
                ) : null}
                {expanded ? (
                  <div className="border-t py-3">
                    <EmailAccountEditor
                      key={account.id}
                      account={account}
                      onSave={(patch) => updateAccount(account.id, patch)}
                    />
                  </div>
                ) : null}
              </div>
            );
          })}
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
        <Row label={t("app.name")} desc={t("settings.aboutDesc", { v: APP_VERSION })} />
        <Row
          label={t("update.label")}
          desc={t("update.labelDesc")}
          control={
            <Button
              size="sm"
              variant="outline"
              disabled={checkBusy}
              onClick={() => void handleCheck()}
            >
              {checkBusy ? t("update.checking") : t("update.check")}
            </Button>
          }
        />
        <Row label={t("settings.dataLabel")} desc={t("settings.dataDesc")} />
      </Section>
    </div>
  );
}

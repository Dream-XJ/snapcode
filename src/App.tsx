import { useEffect, useRef, useState } from "react";
import { Toaster, toast } from "sonner";

import { HistoryPage } from "@/components/HistoryPage";
import { Onboarding } from "@/components/Onboarding";
import { SettingsPage } from "@/components/SettingsPage";
import { TitleBar } from "@/components/TitleBar";
import { TopBar } from "@/components/TopBar";
import { UpdateDialog } from "@/components/UpdateDialog";
import {
  checkUpdate,
  completeOnboarding,
  getHistory,
  getListenerStatus,
  getSettings,
  getShortcutError,
  onCodeAdded,
  onListenerStatus,
  onShortcutError,
} from "@/lib/tauri";
import { detectLang, I18nProvider, toLang, translate } from "@/lib/i18n";
import {
  applyTheme,
  getStoredTheme,
  setStoredTheme,
  watchSystemTheme,
  type Theme,
} from "@/lib/theme";
import type { CodeRecord, ListenerState, Settings, Tab, UpdateInfo } from "@/types";

export default function App() {
  const [settings, setSettings] = useState<Settings | null>(null);
  const [status, setStatus] = useState<ListenerState | null>(null);
  const [records, setRecords] = useState<CodeRecord[]>([]);
  const [shortcutError, setShortcutError] = useState<string | null>(null);
  const [tab, setTab] = useState<Tab>("history");
  const [theme, setTheme] = useState<Theme>(getStoredTheme);
  const [loaded, setLoaded] = useState(false);
  const [updateInfo, setUpdateInfo] = useState<UpdateInfo | null>(null);

  // 事件回调里读取最新 settings，避免闭包过期
  const settingsRef = useRef<Settings | null>(null);
  useEffect(() => {
    settingsRef.current = settings;
  }, [settings]);

  /** 事件回调里取当前界面语言（设置未加载时跟随系统语言） */
  function currentLang() {
    return settingsRef.current ? toLang(settingsRef.current.language) : detectLang();
  }

  // 主题应用；「跟随系统」时订阅系统主题变化
  useEffect(() => {
    applyTheme(theme);
    if (theme !== "system") return;
    return watchSystemTheme(() => applyTheme("system"));
  }, [theme]);

  // 初始并行加载
  useEffect(() => {
    Promise.all([getSettings(), getListenerStatus(), getHistory(null)])
      .then(([s, st, h]) => {
        setSettings(s);
        setStatus(st);
        setRecords(h);
      })
      .catch((e: unknown) =>
        toast.error(translate(currentLang(), "app.initFailed", { err: String(e) })),
      )
      .finally(() => setLoaded(true));
    getShortcutError()
      .then(setShortcutError)
      .catch(() => undefined);
  }, []);

  // 启动后自动检查一次更新；离线等失败静默忽略，手动检查走设置页
  const onboarded = settings?.onboarded;
  useEffect(() => {
    if (!loaded || !onboarded) return;
    checkUpdate()
      .then(setUpdateInfo)
      .catch(() => undefined);
  }, [loaded, onboarded]);

  // 全局事件订阅
  useEffect(() => {
    const offCode = onCodeAdded((rec) => {
      setRecords((prev) => [rec, ...prev]);
      if (settingsRef.current?.auto_copy)
        toast.success(translate(currentLang(), "app.codeCopied", { code: rec.code }));
      else toast.success(translate(currentLang(), "app.codeReceived", { code: rec.code }));
    });
    const offStatus = onListenerStatus((st) => {
      setStatus(st);
      if (st.state === "access_denied") {
        toast.error(translate(currentLang(), "app.accessDeniedToast"));
      }
    });
    const offShortcut = onShortcutError(setShortcutError);
    return () => {
      offCode();
      offStatus();
      offShortcut();
    };
  }, []);

  function handleThemeChange(t: Theme) {
    setStoredTheme(t);
    setTheme(t);
  }

  async function handleCompleteOnboarding() {
    try {
      await completeOnboarding();
      setSettings((s) => (s ? { ...s, onboarded: true } : s));
    } catch (e) {
      toast.error(String(e));
    }
  }

  /** 手动检查更新：有新版本则弹窗，无则提示已是最新；出错 reject 给调用方 */
  async function handleCheckUpdate() {
    const info = await checkUpdate();
    if (info) setUpdateInfo(info);
    else toast.success(translate(currentLang(), "update.latest"));
  }

  const lang = settings ? toLang(settings.language) : detectLang();

  if (!loaded || !settings) {
    return (
      <I18nProvider lang={lang}>
        <div className="flex h-screen flex-col bg-background text-foreground">
          <TitleBar />
          <div className="flex flex-1 items-center justify-center text-sm text-muted-foreground">
            {translate(lang, "app.loading")}
          </div>
        </div>
      </I18nProvider>
    );
  }

  return (
    <I18nProvider lang={lang}>
      <div className="flex h-screen flex-col bg-background text-foreground">
        <TitleBar />
        {settings.onboarded ? (
          <div className="flex min-h-0 flex-1 flex-col">
            <TopBar status={status} tab={tab} onTabChange={setTab} />
            <main className="min-h-0 flex-1">
              {tab === "history" ? (
                <HistoryPage records={records} onRecordsChange={setRecords} />
              ) : (
                <SettingsPage
                  settings={settings}
                  onSettingsChange={setSettings}
                  status={status}
                  shortcutError={shortcutError}
                  theme={theme}
                  onThemeChange={handleThemeChange}
                  onClearHistory={() => setRecords([])}
                  onCheckUpdate={handleCheckUpdate}
                />
              )}
            </main>
          </div>
        ) : (
          <div className="min-h-0 flex-1">
            <Onboarding
              status={status}
              shortcut={settings.shortcut}
              onComplete={() => void handleCompleteOnboarding()}
            />
          </div>
        )}
        <Toaster position="bottom-center" theme={theme} richColors />
        {updateInfo && <UpdateDialog info={updateInfo} onClose={() => setUpdateInfo(null)} />}
      </div>
    </I18nProvider>
  );
}

import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type { CodeRecord, EmailSettings, EmailState, ListenerState, Settings, ToastInfo, UpdateInfo, UpdateProgress } from "@/types";
import pkg from "../../package.json";

/**
 * 按前后端契约封装的 typed invoke 与事件订阅。
 * 在纯浏览器环境（vite dev 预览、无 Tauri 运行时）下自动切换为内存 Mock，
 * 真实 Tauri 环境中全部走 invoke / listen。
 */

/** 版本号取自 package.json，由 bump-version.mjs 与 tauri.conf.json 保持同步 */
export const APP_VERSION: string = pkg.version;

const isTauri = typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

function call<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  if (isTauri) return invoke<T>(cmd, args);
  return mockInvoke(cmd, args) as Promise<T>;
}

/* ---------- 命令 ---------- */

export const getHistory = (query: string | null) =>
  call<CodeRecord[]>("get_history", { query });

export const clearHistory = () => call<void>("clear_history");

export const deleteRecord = (id: number) => call<void>("delete_record", { id });

/** 后端写剪贴板并标记已用，返回验证码 */
export const copyCode = (id: number) => call<string>("copy_code", { id });

export const getSettings = () => call<Settings>("get_settings");

/** 整体替换；快捷键被占用等副作用失败时以错误字符串 reject */
export const updateSettings = (settings: Settings) =>
  call<Settings>("update_settings", { settings });

export const getListenerStatus = () => call<ListenerState>("get_listener_status");

export const retryListener = () => call<void>("retry_listener");

export const openNotificationSettings = () => call<void>("open_notification_settings");

export const setPaused = (paused: boolean) => call<void>("set_paused", { paused });

/** 走完整解析+入库流程，返回识别到的验证码（未识别为 null） */
export const simulateNotification = (text: string) =>
  call<string | null>("simulate_notification", { text });

export const completeOnboarding = () => call<void>("complete_onboarding");

export const getShortcutError = () => call<string | null>("get_shortcut_error");

/** 列出当前系统 Toast 通知（诊断来源过滤用） */
export const dumpNotifications = () => call<ToastInfo[]>("dump_notifications");

/** 检查更新；已是最新返回 null */
export const checkUpdate = () => call<UpdateInfo | null>("check_update");

/** 下载并安装更新；成功后进程由安装器接管退出，Promise 通常不会 resolve */
export const installUpdate = () => call<void>("install_update");

export const getEmailStatus = () => call<EmailState>("get_email_status");

/** 测试邮箱连接（表单当前值，可能未保存），成功返回邮箱中的邮件总数 */
export const testEmailConnection = (config: EmailSettings) =>
  call<number>("test_email_connection", { config });

/* ---------- 事件 ---------- */

type Unlisten = () => void;

function subscribe<T>(event: string, cb: (payload: T) => void): Unlisten {
  if (isTauri) {
    const pending = listen<T>(event, (e) => cb(e.payload));
    return () => {
      void pending.then((unlisten) => unlisten()).catch(() => undefined);
    };
  }
  return mockOn(event, (payload) => cb(payload as T));
}

export const onCodeAdded = (cb: (record: CodeRecord) => void): Unlisten =>
  subscribe("code-added", cb);

export const onListenerStatus = (cb: (state: ListenerState) => void): Unlisten =>
  subscribe("listener-status", cb);

export const onShortcutError = (cb: (error: string | null) => void): Unlisten =>
  subscribe("shortcut-error", cb);

export const onUpdateProgress = (cb: (progress: UpdateProgress) => void): Unlisten =>
  subscribe("update-download-progress", cb);

export const onEmailStatus = (cb: (state: EmailState) => void): Unlisten =>
  subscribe("email-status", cb);

/* ---------- 浏览器 Mock（仅在无 Tauri 运行时时使用） ---------- */

type MockHandler = (payload: unknown) => void;
const mockHandlers = new Map<string, Set<MockHandler>>();

function mockOn(event: string, handler: MockHandler): Unlisten {
  let set = mockHandlers.get(event);
  if (!set) {
    set = new Set();
    mockHandlers.set(event, set);
  }
  set.add(handler);
  return () => {
    set.delete(handler);
  };
}

function mockEmit(event: string, payload: unknown): void {
  mockHandlers.get(event)?.forEach((h) => h(payload));
}

const bootTime = Date.now();

let mockRecords: CodeRecord[] = [
  {
    id: 3,
    source: "Microsoft.YourPhone_8wekyb3d8bbwe",
    sender: "10690000",
    body: "【阿里云】您的验证码为 482913，5 分钟内有效，请勿泄露。",
    code: "482913",
    received_at: bootTime - 3 * 60_000,
    used: false,
  },
  {
    id: 2,
    source: "Microsoft.YourPhone_8wekyb3d8bbwe",
    sender: "95588",
    body: "您正在登录网上银行，验证码 730251，工作人员不会索取。",
    code: "730251",
    received_at: bootTime - 42 * 60_000,
    used: true,
  },
  {
    id: 1,
    source: "Microsoft.YourPhone_8wekyb3d8bbwe",
    sender: null,
    body: "【微信】验证码：159357。请勿转发给他人。",
    code: "159357",
    received_at: bootTime - 26 * 3600_000,
    used: false,
  },
];

let mockSettings: Settings = {
  auto_copy: true,
  shortcut: "Ctrl+Shift+V",
  autostart: true,
  retention_days: 7,
  aumids: ["Microsoft.YourPhone_8wekyb3d8bbwe"],
  onboarded: true,
  language: "zh-CN",
  // 与 Rust 侧 EmailSettings::default() 对齐
  email: {
    enabled: false,
    host: "",
    port: 995,
    username: "",
    password: "",
    use_tls: true,
    poll_interval_secs: 60,
  },
};

let mockStatus: ListenerState = { state: "running", message: null };
let mockEmailStatus: EmailState = { state: "disabled", message: null };
let mockNextId = 100;

async function mockInvoke(cmd: string, args?: Record<string, unknown>): Promise<unknown> {
  await new Promise((r) => setTimeout(r, 50));
  switch (cmd) {
    case "get_history": {
      const q = ((args?.query as string | null) ?? "").trim().toLowerCase();
      const list = q
        ? mockRecords.filter(
            (r) =>
              r.code.includes(q) ||
              r.body.toLowerCase().includes(q) ||
              (r.sender ?? "").toLowerCase().includes(q) ||
              r.source.toLowerCase().includes(q),
          )
        : mockRecords;
      return [...list].sort((a, b) => b.received_at - a.received_at);
    }
    case "clear_history":
      mockRecords = [];
      return undefined;
    case "delete_record":
      mockRecords = mockRecords.filter((r) => r.id !== (args?.id as number));
      return undefined;
    case "copy_code": {
      const rec = mockRecords.find((r) => r.id === (args?.id as number));
      if (!rec) throw "记录不存在";
      rec.used = true;
      try {
        await navigator.clipboard.writeText(rec.code);
      } catch {
        /* 非安全上下文下剪贴板不可用，忽略 */
      }
      return rec.code;
    }
    case "get_settings":
      return { ...mockSettings, aumids: [...mockSettings.aumids], email: { ...mockSettings.email } };
    case "update_settings":
      mockSettings = args?.settings as Settings;
      return { ...mockSettings, aumids: [...mockSettings.aumids], email: { ...mockSettings.email } };
    case "get_listener_status":
      return mockStatus;
    case "retry_listener":
      mockStatus = { state: "running", message: null };
      mockEmit("listener-status", mockStatus);
      return undefined;
    case "open_notification_settings":
      return undefined;
    case "set_paused":
      mockStatus = { state: (args?.paused as boolean) ? "paused" : "running", message: null };
      mockEmit("listener-status", mockStatus);
      return undefined;
    case "simulate_notification": {
      const text = (args?.text as string) ?? "";
      const m = text.match(/\d{4,8}/);
      if (!m) return null;
      const rec: CodeRecord = {
        id: ++mockNextId,
        source: "Microsoft.YourPhone_8wekyb3d8bbwe",
        sender: "模拟号码",
        body: text,
        code: m[0],
        received_at: Date.now(),
        used: false,
      };
      mockRecords = [rec, ...mockRecords];
      mockEmit("code-added", rec);
      return rec.code;
    }
    case "complete_onboarding":
      mockSettings = { ...mockSettings, onboarded: true };
      return undefined;
    case "get_shortcut_error":
      return null;
    case "dump_notifications":
      return [
        {
          aumid: "Microsoft.YourPhone_8wekyb3d8bbwe!App",
          title: "10690000",
          body: "【阿里云】您的验证码为 482913，5 分钟内有效。",
        },
        {
          aumid: "Microsoft.Windows.Explorer",
          title: "iPhone 已连接",
          body: "手机连接已就绪。",
        },
      ];
    case "check_update":
      // 预览模式不模拟新版本
      return null;
    case "install_update":
      throw "浏览器预览模式不支持安装更新";
    case "get_email_status":
      return mockEmailStatus;
    case "test_email_connection": {
      const cfg = args?.config as EmailSettings;
      if (!cfg.host.trim() || !cfg.username.trim() || !cfg.password) {
        throw "请先在设置中填写邮箱配置";
      }
      // 预览模式模拟一次成功连接，并把状态切到轮询中便于预览 UI
      mockEmailStatus = { state: "running", message: null };
      mockEmit("email-status", mockEmailStatus);
      return 128;
    }
    default:
      throw `未知命令: ${cmd}`;
  }
}

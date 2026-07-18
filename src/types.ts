/** 与 Rust 侧 serde 对应的前后端契约类型（TS 侧 number = Rust i64） */

export interface CodeRecord {
  id: number;
  /** 来源应用 AUMID */
  source: string;
  sender: string | null;
  body: string;
  code: string;
  /** unix 毫秒时间戳 */
  received_at: number;
  used: boolean;
}

export interface Settings {
  auto_copy: boolean;
  shortcut: string;
  autostart: boolean;
  retention_days: number;
  aumids: string[];
  onboarded: boolean;
  /** 界面语言："zh-CN" | "en"（Rust 侧为 String，非 "en" 一律按中文） */
  language: string;
  email: EmailSettings;
}

export type EmailProtocol = "pop3" | "imap";

/** 单个邮箱账户配置（对应 Rust EmailAccount） */
export interface EmailAccount {
  /** 账户唯一 id，由前端生成 */
  id: string;
  /** 备注名；空时显示账号地址 */
  name: string;
  protocol: EmailProtocol;
  enabled: boolean;
  /** 服务器，如 pop.qq.com / imap.qq.com */
  host: string;
  /** SSL/TLS 默认端口：POP3 995 / IMAP 993 */
  port: number;
  /** 邮箱账号（一般即完整地址） */
  username: string;
  /** 密码或客户端授权码（QQ/163 等需用授权码） */
  password: string;
  /** true = 隐式 TLS */
  use_tls: boolean;
  /** 轮询间隔秒数，最小 15；IMAP 仅在不支持 IDLE 时生效 */
  poll_interval_secs: number;
}

/** 邮箱验证码监听配置：多账户列表（对应 Rust EmailSettings） */
export interface EmailSettings {
  accounts: EmailAccount[];
}

export type EmailStateName = "disabled" | "running" | "paused" | "error";

/** 单个账户的轮询状态（get_email_status 返回元素 / email-status 事件 payload） */
export interface EmailAccountStatus {
  account_id: string;
  state: EmailStateName;
  message: string | null;
}

export type ListenerStateName =
  | "starting"
  | "running"
  | "paused"
  | "access_denied"
  | "unsupported"
  | "error";

export interface ListenerState {
  state: ListenerStateName;
  message: string | null;
}

/** 诊断用：一条系统 Toast 通知的来源与文本 */
export interface ToastInfo {
  aumid: string;
  title: string;
  body: string;
}

export type Tab = "history" | "settings";

/** 可用更新信息（Rust check_update 返回；已是最新时为 null） */
export interface UpdateInfo {
  version: string;
  current_version: string;
  /** Release 说明（Markdown 文本），可能为空 */
  body: string | null;
  /** 发布日期 YYYY-MM-DD，可能为空 */
  date: string | null;
}

/** 更新下载进度（update-download-progress 事件） */
export interface UpdateProgress {
  downloaded: number;
  total: number | null;
}

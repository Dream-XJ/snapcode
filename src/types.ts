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

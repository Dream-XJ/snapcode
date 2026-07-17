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

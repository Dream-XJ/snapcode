import { clsx, type ClassValue } from "clsx";
import { twMerge } from "tailwind-merge";

export function cn(...inputs: ClassValue[]) {
  return twMerge(clsx(inputs));
}

/** 默认监听来源（手机连接）。与 Rust 侧 settings.rs 的默认值保持一致 */
export const DEFAULT_AUMID = "Microsoft.YourPhone_8wekyb3d8bbwe";

/** AUMID → 来源显示名：含 YourPhone 显示为「手机连接」，否则原样展示 */
export function sourceDisplayName(aumid: string): string {
  return aumid.includes("YourPhone") ? "手机连接" : aumid;
}

/** 监听状态 → 状态点颜色与文案 */
export function statusMeta(state: string): { dot: string; text: string } {
  switch (state) {
    case "running":
      return { dot: "bg-emerald-500", text: "监听中" };
    case "paused":
      return { dot: "bg-amber-500", text: "已暂停" };
    case "access_denied":
      return { dot: "bg-red-500", text: "权限被拒" };
    case "error":
      return { dot: "bg-red-500", text: "监听出错" };
    case "starting":
      return { dot: "bg-zinc-400", text: "启动中" };
    case "unsupported":
      return { dot: "bg-zinc-400", text: "系统不支持" };
    default:
      return { dot: "bg-zinc-400", text: state };
  }
}

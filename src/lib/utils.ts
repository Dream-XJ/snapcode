import { clsx, type ClassValue } from "clsx";
import { twMerge } from "tailwind-merge";

import { translate, type Lang } from "@/lib/i18n";

export function cn(...inputs: ClassValue[]) {
  return twMerge(clsx(inputs));
}

/** 默认监听来源（手机连接）。与 Rust 侧 settings.rs 的默认值保持一致 */
export const DEFAULT_AUMID = "Microsoft.YourPhone_8wekyb3d8bbwe";

/** AUMID → 来源显示名：含 YourPhone 显示为「手机连接 / Phone Link」，否则原样展示 */
export function sourceDisplayName(aumid: string, lang: Lang): string {
  return aumid.includes("YourPhone") ? translate(lang, "source.phoneLink") : aumid;
}

/** 监听状态 → 状态点颜色与文案 */
export function statusMeta(state: string, lang: Lang): { dot: string; text: string } {
  switch (state) {
    case "running":
      return { dot: "bg-emerald-500", text: translate(lang, "status.running") };
    case "paused":
      return { dot: "bg-amber-500", text: translate(lang, "status.paused") };
    case "access_denied":
      return { dot: "bg-red-500", text: translate(lang, "status.accessDenied") };
    case "error":
      return { dot: "bg-red-500", text: translate(lang, "status.error") };
    case "starting":
      return { dot: "bg-zinc-400", text: translate(lang, "status.starting") };
    case "unsupported":
      return { dot: "bg-zinc-400", text: translate(lang, "status.unsupported") };
    default:
      return { dot: "bg-zinc-400", text: state };
  }
}

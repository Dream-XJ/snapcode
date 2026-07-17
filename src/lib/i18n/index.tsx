import { createContext, useContext, useMemo, type ReactNode } from "react";

import { en } from "./en";
import { zhCN, type Messages } from "./zh-CN";

/** 界面语言。与 Rust 侧 Settings.language 对应；非 "en" 一律按中文处理 */
export type Lang = "zh-CN" | "en";

export type MessageKey = keyof Messages;

const dicts: Record<Lang, Messages> = { "zh-CN": zhCN, en };

/** 任意字符串 → Lang（Settings.language 在 Rust 侧是自由 String） */
export function toLang(value: string | null | undefined): Lang {
  return value === "en" ? "en" : "zh-CN";
}

/** 首帧语言（设置加载完成前）：跟随系统/浏览器语言 */
export function detectLang(): Lang {
  return navigator.language.toLowerCase().startsWith("zh") ? "zh-CN" : "en";
}

/** 查词典并做 {name} 插值。纯函数，非 React 模块（time.ts / utils.ts）也用 */
export function translate(
  lang: Lang,
  key: MessageKey,
  vars?: Record<string, string | number>,
): string {
  let s: string = dicts[lang][key];
  if (vars) {
    for (const [k, v] of Object.entries(vars)) {
      // tsconfig lib 低于 ES2021，无 replaceAll，用 split/join 等价替换
      s = s.split(`{${k}}`).join(String(v));
    }
  }
  return s;
}

interface I18nValue {
  lang: Lang;
  t: (key: MessageKey, vars?: Record<string, string | number>) => string;
}

const I18nContext = createContext<I18nValue>({
  lang: "zh-CN",
  t: (key) => zhCN[key],
});

export function I18nProvider({ lang, children }: { lang: Lang; children: ReactNode }) {
  const value = useMemo<I18nValue>(
    () => ({ lang, t: (key, vars) => translate(lang, key, vars) }),
    [lang],
  );
  return <I18nContext.Provider value={value}>{children}</I18nContext.Provider>;
}

export function useI18n(): I18nValue {
  return useContext(I18nContext);
}

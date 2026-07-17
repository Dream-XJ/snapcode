export type Theme = "light" | "dark" | "system";

const STORAGE_KEY = "snapcode-theme";

export function getStoredTheme(): Theme {
  const v = localStorage.getItem(STORAGE_KEY);
  return v === "light" || v === "dark" ? v : "system";
}

function resolved(t: Theme): "light" | "dark" {
  if (t !== "system") return t;
  return window.matchMedia("(prefers-color-scheme: dark)").matches ? "dark" : "light";
}

/** 切换 <html> 的 .dark class */
export function applyTheme(t: Theme): void {
  document.documentElement.classList.toggle("dark", resolved(t) === "dark");
}

export function setStoredTheme(t: Theme): void {
  localStorage.setItem(STORAGE_KEY, t);
  applyTheme(t);
}

/** 监听系统主题变化（仅在「跟随系统」时使用），返回取消函数 */
export function watchSystemTheme(cb: () => void): () => void {
  const mq = window.matchMedia("(prefers-color-scheme: dark)");
  mq.addEventListener("change", cb);
  return () => mq.removeEventListener("change", cb);
}

import { getCurrentWindow } from "@tauri-apps/api/window";
import { Minus, X } from "lucide-react";

import icon from "@/assets/icon.png";
import { useI18n } from "@/lib/i18n";

/** 自定义 Windows 风格标题栏：左侧图标与名称，中间留白为拖拽区，右侧最小化/关闭 */
export function TitleBar() {
  const appWindow = getCurrentWindow();
  const { t } = useI18n();

  return (
    <header className="flex h-9 shrink-0 items-center border-b">
      <div className="flex items-center gap-2 pl-3">
        <img src={icon} alt="" draggable={false} className="h-4 w-4" />
        <span className="text-xs text-muted-foreground">{t("app.name")}</span>
      </div>

      {/* 拖拽区：勿放入文字或按钮，避免拖拽吞掉点击 */}
      <div className="h-full flex-1" data-tauri-drag-region />

      <button
        type="button"
        title={t("common.minimize")}
        onClick={() => void appWindow.minimize()}
        className="flex h-9 w-11 items-center justify-center transition-colors hover:bg-muted"
      >
        <Minus className="h-4 w-4" />
      </button>
      <button
        type="button"
        title={t("common.close")}
        onClick={() => void appWindow.close()}
        className="flex h-9 w-11 items-center justify-center transition-colors hover:bg-[#e81123] hover:text-white"
      >
        <X className="h-4 w-4" />
      </button>
    </header>
  );
}

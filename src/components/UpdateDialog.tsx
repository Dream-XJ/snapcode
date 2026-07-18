import { useEffect, useState } from "react";
import { Download, X } from "lucide-react";
import { toast } from "sonner";

import { Button } from "@/components/ui/button";
import { useI18n } from "@/lib/i18n";
import { installUpdate, onUpdateProgress } from "@/lib/tauri";
import { cn } from "@/lib/utils";
import type { UpdateInfo, UpdateProgress } from "@/types";

type Phase = "idle" | "busy" | "error";

interface UpdateDialogProps {
  info: UpdateInfo;
  /** 「暂不更新」或出错后关闭对话框（本次跳过，下次启动还会再提醒） */
  onClose: () => void;
}

/** 新版本提示弹窗：展示版本号与更新说明，一键下载安装；是否更新由用户决定 */
export function UpdateDialog({ info, onClose }: UpdateDialogProps) {
  const { t } = useI18n();
  const [phase, setPhase] = useState<Phase>("idle");
  const [progress, setProgress] = useState<UpdateProgress | null>(null);

  // 下载进度事件：install_update 期间由 Rust 侧持续广播
  useEffect(() => onUpdateProgress(setProgress), []);

  const percent =
    progress?.total && progress.total > 0
      ? Math.min(100, Math.round((progress.downloaded / progress.total) * 100))
      : null;

  async function handleInstall() {
    if (phase === "busy") return;
    setProgress(null);
    setPhase("busy");
    try {
      // 正常情况下安装器接管后本进程即退出，Promise 不会 resolve
      await installUpdate();
    } catch (e) {
      setPhase("error");
      toast.error(t("update.failed", { err: String(e) }));
    }
  }

  const busy = phase === "busy";

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/40 p-4">
      <div className="w-full max-w-sm rounded-xl border bg-card p-4 shadow-xl">
        <div className="flex items-start justify-between gap-2">
          <div>
            <h2 className="text-sm font-semibold">{t("update.title")}</h2>
            <p className="mt-0.5 font-mono text-xs text-muted-foreground">
              {t("update.versionLine", { from: info.current_version, to: info.version })}
            </p>
          </div>
          {!busy && (
            <button
              type="button"
              onClick={onClose}
              title={t("common.close")}
              className="rounded-md p-1 text-muted-foreground transition-colors hover:bg-accent hover:text-foreground"
            >
              <X className="h-4 w-4" />
            </button>
          )}
        </div>

        <div className="mt-3 max-h-48 overflow-y-auto rounded-md bg-muted/40 p-3">
          <p className="text-xs font-medium">{t("update.notesTitle")}</p>
          <p className="mt-1 whitespace-pre-wrap text-xs leading-relaxed text-muted-foreground">
            {info.body?.trim() || t("update.noNotes")}
          </p>
        </div>

        {busy ? (
          <div className="mt-4 space-y-2">
            <div className="h-1.5 overflow-hidden rounded-full bg-muted">
              <div
                className={cn(
                  "h-full rounded-full bg-primary transition-[width]",
                  percent === null && "w-1/3 animate-pulse",
                )}
                style={percent !== null ? { width: `${percent}%` } : undefined}
              />
            </div>
            <p className="text-center text-xs text-muted-foreground">
              {percent === null
                ? t("update.downloadingNoTotal")
                : percent >= 100
                  ? t("update.installing")
                  : t("update.downloading", { p: percent })}
            </p>
          </div>
        ) : (
          <div className="mt-4 flex justify-end gap-2">
            <Button variant="ghost" size="sm" onClick={onClose}>
              {t("update.later")}
            </Button>
            <Button size="sm" onClick={() => void handleInstall()}>
              <Download className="mr-1 h-3.5 w-3.5" />
              {phase === "error" ? t("update.retry") : t("update.install")}
            </Button>
          </div>
        )}
      </div>
    </div>
  );
}

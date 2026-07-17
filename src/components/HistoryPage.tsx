import { useEffect, useState } from "react";
import { toast } from "sonner";
import { Copy, Inbox, Search, Trash2 } from "lucide-react";

import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { useI18n } from "@/lib/i18n";
import { copyCode, deleteRecord, getHistory } from "@/lib/tauri";
import { groupByDay, relativeTime } from "@/lib/time";
import { cn, sourceDisplayName } from "@/lib/utils";
import type { CodeRecord } from "@/types";

interface HistoryPageProps {
  records: CodeRecord[];
  onRecordsChange: (records: CodeRecord[]) => void;
}

export function HistoryPage({ records, onRecordsChange }: HistoryPageProps) {
  const { t, lang } = useI18n();
  const [query, setQuery] = useState("");
  /** 搜索结果；null 表示未在搜索，直接展示完整列表 */
  const [results, setResults] = useState<CodeRecord[] | null>(null);

  // 实时搜索（轻防抖）；关键字为空时回退到完整列表
  useEffect(() => {
    const q = query.trim();
    if (!q) {
      setResults(null);
      return;
    }
    const timer = window.setTimeout(() => {
      getHistory(q)
        .then(setResults)
        .catch((e: unknown) => toast.error(String(e)));
    }, 200);
    return () => window.clearTimeout(timer);
  }, [query]);

  const shown = results ?? records;
  const groups = groupByDay(shown, (r) => r.received_at, lang);

  function patchEverywhere(id: number, patch: Partial<CodeRecord>) {
    const apply = (list: CodeRecord[]) =>
      list.map((r) => (r.id === id ? { ...r, ...patch } : r));
    onRecordsChange(apply(records));
    setResults((prev) => (prev ? apply(prev) : prev));
  }

  function removeEverywhere(id: number) {
    onRecordsChange(records.filter((r) => r.id !== id));
    setResults((prev) => (prev ? prev.filter((r) => r.id !== id) : prev));
  }

  async function handleCopy(id: number) {
    try {
      await copyCode(id);
      patchEverywhere(id, { used: true });
      toast.success(t("history.copied"));
    } catch (e) {
      toast.error(String(e));
    }
  }

  async function handleDelete(id: number) {
    try {
      await deleteRecord(id);
      removeEverywhere(id);
      toast.success(t("history.deleted"));
    } catch (e) {
      toast.error(String(e));
    }
  }

  return (
    <div className="flex h-full flex-col">
      <div className="border-b px-3 py-2.5">
        <div className="relative">
          <Search className="pointer-events-none absolute left-2.5 top-1/2 h-4 w-4 -translate-y-1/2 text-muted-foreground" />
          <Input
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            placeholder={t("history.searchPlaceholder")}
            className="h-8 pl-8"
          />
        </div>
      </div>

      <div className="min-h-0 flex-1 overflow-y-auto">
        {shown.length === 0 ? (
          query.trim() ? (
            <div className="flex flex-col items-center gap-2 px-8 py-16 text-center">
              <p className="text-sm font-medium">{t("history.noMatch")}</p>
              <p className="text-xs text-muted-foreground">{t("history.noMatchHint")}</p>
            </div>
          ) : (
            <div className="flex flex-col items-center gap-3 px-8 py-16 text-center">
              <div className="rounded-full bg-muted p-4">
                <Inbox className="h-6 w-6 text-muted-foreground" />
              </div>
              <p className="text-sm font-medium">{t("history.emptyTitle")}</p>
              <p className="max-w-[300px] text-xs leading-relaxed text-muted-foreground">
                {t("history.emptyDesc")}
              </p>
            </div>
          )
        ) : (
          groups.map((g) => (
            <section key={g.label}>
              <h3 className="sticky top-0 z-10 border-b bg-background/95 px-4 py-1.5 text-xs font-medium text-muted-foreground backdrop-blur">
                {g.label}
              </h3>
              <div className="divide-y">
                {g.items.map((r) => (
                  <div
                    key={r.id}
                    className="group flex items-center gap-2 px-4 py-3 transition-colors hover:bg-accent/50"
                  >
                    <div className="min-w-0 flex-1">
                      <p
                        className={cn(
                          "select-text font-mono text-2xl font-semibold tracking-widest",
                          r.used && "text-muted-foreground",
                        )}
                      >
                        {r.code}
                      </p>
                      <div className="mt-1 flex items-center gap-1.5 text-xs text-muted-foreground">
                        <span className="truncate">{r.sender ?? t("history.unknownSender")}</span>
                        <span className="shrink-0">·</span>
                        <span className="shrink-0">{relativeTime(r.received_at, lang)}</span>
                        <span className="shrink-0">·</span>
                        <span className="shrink-0">{sourceDisplayName(r.source, lang)}</span>
                        {r.used && (
                          <span className="shrink-0 rounded bg-muted px-1 py-0.5 text-[10px] leading-none">
                            {t("history.used")}
                          </span>
                        )}
                      </div>
                    </div>
                    <div className="flex shrink-0 items-center gap-0.5 opacity-0 transition-opacity focus-within:opacity-100 group-hover:opacity-100">
                      <Button
                        variant="ghost"
                        size="icon"
                        className="h-8 w-8"
                        title={t("common.copy")}
                        onClick={() => void handleCopy(r.id)}
                      >
                        <Copy className="h-4 w-4" />
                      </Button>
                      <Button
                        variant="ghost"
                        size="icon"
                        className="h-8 w-8 text-muted-foreground hover:text-destructive"
                        title={t("common.delete")}
                        onClick={() => void handleDelete(r.id)}
                      >
                        <Trash2 className="h-4 w-4" />
                      </Button>
                    </div>
                  </div>
                ))}
              </div>
            </section>
          ))
        )}
      </div>
    </div>
  );
}

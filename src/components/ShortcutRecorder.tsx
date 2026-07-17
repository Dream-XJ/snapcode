import { useEffect, useRef, useState, type KeyboardEvent } from "react";

import { Input } from "@/components/ui/input";
import { cn } from "@/lib/utils";

interface ShortcutRecorderProps {
  value: string;
  /** 后端报告的错误（如快捷键被占用），红字展示 */
  error: string | null;
  onSave: (shortcut: string) => Promise<void>;
}

const MODIFIER_KEYS = new Set(["Control", "Shift", "Alt", "Meta"]);

/**
 * 把按键事件转成契约格式「Ctrl+Shift+V」。
 * 纯修饰键、无修饰键均返回 null（忽略）。
 */
function formatCombo(e: KeyboardEvent): string | null {
  const key = e.key;
  if (MODIFIER_KEYS.has(key)) return null;
  const parts: string[] = [];
  if (e.ctrlKey) parts.push("Ctrl");
  if (e.altKey) parts.push("Alt");
  if (e.shiftKey) parts.push("Shift");
  if (e.metaKey) parts.push("Meta");
  if (parts.length === 0) return null;
  let name: string;
  if (key === " ") name = "Space";
  else if (key.startsWith("Arrow")) name = key.slice(5); // Up / Down / Left / Right
  else if (key.length === 1) name = key.toUpperCase();
  else name = key; // Enter / Tab / Escape / Backspace / Delete / F1-F24 ...
  return [...parts, name].join("+");
}

export function ShortcutRecorder({ value, error, onSave }: ShortcutRecorderProps) {
  const [recording, setRecording] = useState(false);
  const [draft, setDraft] = useState(value);
  const [saving, setSaving] = useState(false);
  /** 避免回车保存后紧接着的 blur 触发重复保存 */
  const lastSubmitted = useRef<string | null>(null);

  // 外部值变化（保存成功 / 设置加载完成）且不在录制时，同步草稿
  useEffect(() => {
    if (!recording) setDraft(value);
  }, [value, recording]);

  async function save(shortcut: string) {
    if (!shortcut || shortcut === value || shortcut === lastSubmitted.current) return;
    lastSubmitted.current = shortcut;
    setSaving(true);
    try {
      await onSave(shortcut);
    } catch {
      // 保存失败（如被占用）：还原显示，错误提示由调用方展示
      lastSubmitted.current = null;
      setDraft(value);
    } finally {
      setSaving(false);
    }
  }

  return (
    <div className="w-44">
      <Input
        readOnly
        value={recording ? draft : value}
        placeholder={saving ? "保存中…" : recording ? "按下组合键…" : "点击录入"}
        onFocus={() => {
          setRecording(true);
          setDraft("");
          lastSubmitted.current = null;
        }}
        onBlur={() => {
          setRecording(false);
          if (draft && draft !== value && draft !== lastSubmitted.current) {
            void save(draft);
          } else {
            setDraft(value);
          }
        }}
        onKeyDown={(e) => {
          // 放行无修饰键的 Tab，保证可以移出焦点
          if (e.key === "Tab" && !e.ctrlKey && !e.altKey && !e.shiftKey && !e.metaKey) return;
          e.preventDefault();
          const combo = formatCombo(e);
          if (combo) {
            setDraft(combo);
            return;
          }
          if (e.key === "Enter" && draft) {
            void save(draft);
            e.currentTarget.blur();
          } else if (e.key === "Escape") {
            setDraft(value);
            e.currentTarget.blur();
          }
        }}
        className={cn(
          "h-8 cursor-pointer text-center font-mono text-xs",
          recording && "border-primary ring-1 ring-ring",
        )}
      />
      {error ? <p className="mt-1.5 text-xs leading-relaxed text-destructive">{error}</p> : null}
    </div>
  );
}

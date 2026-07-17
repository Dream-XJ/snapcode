import { translate, type Lang } from "@/lib/i18n";

/** 英文短月份名（zh 走「M月d日」数字格式，无需名称表） */
const EN_MONTHS = [
  "Jan",
  "Feb",
  "Mar",
  "Apr",
  "May",
  "Jun",
  "Jul",
  "Aug",
  "Sep",
  "Oct",
  "Nov",
  "Dec",
];

function isSameDay(a: Date, b: Date): boolean {
  return (
    a.getFullYear() === b.getFullYear() &&
    a.getMonth() === b.getMonth() &&
    a.getDate() === b.getDate()
  );
}

/** 相对时间，如「刚刚」「2 分钟前」/ "just now" "2 min ago" */
export function relativeTime(ms: number, lang: Lang): string {
  const diff = Date.now() - ms;
  if (diff < 45_000) return translate(lang, "time.justNow");
  const minutes = Math.floor(diff / 60_000);
  if (minutes < 60) return translate(lang, "time.minutesAgo", { n: minutes });
  const hours = Math.floor(minutes / 60);
  if (hours < 24 && isSameDay(new Date(ms), new Date()))
    return translate(lang, "time.hoursAgo", { n: hours });
  const days = Math.floor(hours / 24);
  if (days < 30) return translate(lang, "time.daysAgo", { n: days });
  const d = new Date(ms);
  return lang === "en"
    ? `${EN_MONTHS[d.getMonth()]} ${d.getDate()}`
    : `${d.getMonth() + 1}月${d.getDate()}日`;
}

/** 分组标题：今天 / 昨天 / M月d日（跨年补全年份）；英文 Today / Yesterday / Jul 5 */
export function dayLabel(ms: number, lang: Lang): string {
  const d = new Date(ms);
  const now = new Date();
  if (isSameDay(d, now)) return translate(lang, "time.today");
  const yesterday = new Date(now);
  yesterday.setDate(now.getDate() - 1);
  if (isSameDay(d, yesterday)) return translate(lang, "time.yesterday");
  if (d.getFullYear() === now.getFullYear()) {
    return lang === "en"
      ? `${EN_MONTHS[d.getMonth()]} ${d.getDate()}`
      : `${d.getMonth() + 1}月${d.getDate()}日`;
  }
  return lang === "en"
    ? `${EN_MONTHS[d.getMonth()]} ${d.getDate()}, ${d.getFullYear()}`
    : `${d.getFullYear()}年${d.getMonth() + 1}月${d.getDate()}日`;
}

export interface DayGroup<T> {
  label: string;
  items: T[];
}

/** 按天分组（输入需已按时间倒序排列，分组为连续合并） */
export function groupByDay<T>(
  items: T[],
  getTime: (item: T) => number,
  lang: Lang,
): DayGroup<T>[] {
  const groups: DayGroup<T>[] = [];
  for (const item of items) {
    const label = dayLabel(getTime(item), lang);
    const last = groups[groups.length - 1];
    if (last && last.label === label) last.items.push(item);
    else groups.push({ label, items: [item] });
  }
  return groups;
}

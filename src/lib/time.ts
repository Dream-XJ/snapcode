function isSameDay(a: Date, b: Date): boolean {
  return (
    a.getFullYear() === b.getFullYear() &&
    a.getMonth() === b.getMonth() &&
    a.getDate() === b.getDate()
  );
}

/** 相对时间，如「刚刚」「2 分钟前」「3 天前」 */
export function relativeTime(ms: number): string {
  const diff = Date.now() - ms;
  if (diff < 45_000) return "刚刚";
  const minutes = Math.floor(diff / 60_000);
  if (minutes < 60) return `${minutes} 分钟前`;
  const hours = Math.floor(minutes / 60);
  if (hours < 24 && isSameDay(new Date(ms), new Date())) return `${hours} 小时前`;
  const days = Math.floor(hours / 24);
  if (days < 30) return `${days} 天前`;
  const d = new Date(ms);
  return `${d.getMonth() + 1}月${d.getDate()}日`;
}

/** 分组标题：今天 / 昨天 / M月d日（跨年补全年份） */
export function dayLabel(ms: number): string {
  const d = new Date(ms);
  const now = new Date();
  if (isSameDay(d, now)) return "今天";
  const yesterday = new Date(now);
  yesterday.setDate(now.getDate() - 1);
  if (isSameDay(d, yesterday)) return "昨天";
  if (d.getFullYear() === now.getFullYear()) return `${d.getMonth() + 1}月${d.getDate()}日`;
  return `${d.getFullYear()}年${d.getMonth() + 1}月${d.getDate()}日`;
}

export interface DayGroup<T> {
  label: string;
  items: T[];
}

/** 按天分组（输入需已按时间倒序排列，分组为连续合并） */
export function groupByDay<T>(items: T[], getTime: (item: T) => number): DayGroup<T>[] {
  const groups: DayGroup<T>[] = [];
  for (const item of items) {
    const label = dayLabel(getTime(item));
    const last = groups[groups.length - 1];
    if (last && last.label === label) last.items.push(item);
    else groups.push({ label, items: [item] });
  }
  return groups;
}

// 每版那段话在这里生成，不在控制面。
//
// 控制面只给数（DESIGN §3.5：只有计数和中位数，没有比例），句子是呈现：
// 手机上要短、`--json` 要原始数字、`playtest mcp` 要另一种说法。措辞钉在服务端的话，
// 每一处都得先把句子拆回数字。规则集中在这个文件，改文案只改这里。

import type { VersionResults, WorkKind } from "./api";

export function workKind(kind: WorkKind | undefined): string {
  if (kind === "article") return "Article";
  if (kind === "video") return "Video";
  return "Web";
}

/** 没留名字的人在反馈里叫什么：「A playtester」「A reader」「A viewer」。 */
export function audience(kind: WorkKind | undefined): string {
  if (kind === "article") return "A reader";
  if (kind === "video") return "A viewer";
  return "A playtester";
}

/** Seconds → human words. "45s", "3m 20s", "1h 2m". */
export function seconds(value: number): string {
  if (value < 60) return `${value}s`;
  if (value < 3600) {
    const rest = value % 60;
    return rest === 0 ? `${Math.floor(value / 60)}m` : `${Math.floor(value / 60)}m ${rest}s`;
  }
  const hours = Math.floor(value / 3600);
  const minutes = Math.floor((value % 3600) / 60);
  return minutes === 0 ? `${hours}h` : `${hours}h ${minutes}m`;
}

/** RFC 3339 → "Sep 5 14:20", in viewer's local timezone. */
export function moment(iso: string | undefined): string {
  if (!iso) return "";
  const at = new Date(iso);
  if (Number.isNaN(at.getTime())) return iso;
  const clock = `${pad(at.getHours())}:${pad(at.getMinutes())}`;
  const months = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];
  return `${months[at.getMonth()]} ${at.getDate()} ${clock}`;
}

/** RFC 3339 → "Sep 12". */
export function day(iso: string | undefined): string {
  if (!iso) return "";
  const at = new Date(iso);
  if (Number.isNaN(at.getTime())) return iso;
  const months = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];
  return `${months[at.getMonth()]} ${at.getDate()}`;
}

/** RFC 3339 → "14:20:05". */
export function clock(iso: string | undefined): string {
  if (!iso) return "";
  const at = new Date(iso);
  if (Number.isNaN(at.getTime())) return iso;
  return `${pad(at.getHours())}:${pad(at.getMinutes())}:${pad(at.getSeconds())}`;
}

function pad(value: number): string {
  return value.toString().padStart(2, "0");
}

/** Relative time. "just now", "12m ago", "3h ago", falls back to date after a day. */
export function ago(iso: string | undefined, now = Date.now()): string {
  if (!iso) return "";
  const at = new Date(iso).getTime();
  if (Number.isNaN(at)) return iso;
  const delta = Math.max(0, Math.floor((now - at) / 1000));
  if (delta < 60) return "just now";
  if (delta < 3600) return `${Math.floor(delta / 60)}m ago`;
  if (delta < 86400) return `${Math.floor(delta / 3600)}h ago`;
  return moment(iso);
}

/** Time left until expiry. "5h left", "40m left"; null if expired. */
export function left(iso: string | undefined, now = Date.now()): string | null {
  if (!iso) return null;
  const at = new Date(iso).getTime();
  if (Number.isNaN(at)) return null;
  const delta = Math.floor((at - now) / 1000);
  if (delta <= 0) return null;
  if (delta < 3600) return `${Math.max(1, Math.floor(delta / 60))}m left`;
  if (delta < 86400 * 2) return `${Math.floor(delta / 3600)}h left`;
  return `${Math.floor(delta / 86400)}d left`;
}

/** Byte size → "1.2 MB". */
export function bytes(value: number): string {
  if (value < 1024) return `${value} B`;
  if (value < 1024 * 1024) return `${Math.round(value / 1024)} KB`;
  if (value < 1024 * 1024 * 1024) return `${(value / 1024 / 1024).toFixed(1)} MB`;
  return `${(value / 1024 / 1024 / 1024).toFixed(2)} GB`;
}

/** 截断字符串，按 Unicode 字符数算。 */
export function clip(text: string, max: number): string {
  const chars = Array.from(text);
  return chars.length > max ? `${chars.slice(0, max).join("")}…` : text;
}

/** 事实行里的一项：一个数加一个词（`4 left name`），或者只有词（一条堆栈指纹）。 */
export type FactItem = {
  n?: number | string;
  text: string;
  /** 等宽排：堆栈指纹这种要逐字看的。 */
  code?: boolean;
};

export type Fact = {
  label: string;
  items: FactItem[];
  tone?: "warn" | "plain";
  /** 上一版的数，只在变了的时候给；数字旁边的差是好事，这里的差多半是坏事，所以写「was 0」。 */
  was?: number;
};

/**
 * 每版那段话，排成带标签的事实行（DESIGN §3.13）。
 * 打开 / 进到 / 5 分钟 / 反馈头上已经排大了，掉在加载里的那几个挂在数字下面，这里都不再说。
 * 0 的行不出现；`opened === 0` 时一行也没有，由调用方说「还没人来」。
 */
export function facts(version: VersionResults, kind: WorkKind | undefined = "web", previous?: VersionResults): Fact[] {
  if (version.opened === 0) return [];
  const rows: Fact[] = [];

  const stayed: FactItem[] = [];
  if ((version.named ?? 0) > 0) stayed.push({ n: version.named, text: "left name" });
  if (kind !== "article" && kind !== "video" && (version.returned ?? 0) > 0) {
    stayed.push({ n: version.returned, text: "returned" });
  }
  if (version.dwell_median_s !== null && version.dwell_median_s !== undefined) {
    stayed.push({ n: seconds(version.dwell_median_s), text: "median dwell" });
  }
  if (stayed.length > 0) rows.push({ label: "Stayed", items: stayed });

  const from = (version.sources ?? []).filter((one) => one.count > 0);
  if (from.length > 0) {
    rows.push({ label: "From", items: from.map((one) => ({ n: one.count, text: sourceLabel(one.kind) })) });
  }

  const errors = version.errors;
  if (errors.total > 0) {
    // 只有一种错误时，「1 distinct」和下面那条指纹说的是同一件事，省掉。
    const items: FactItem[] = [{ n: errors.total, text: errors.total === 1 ? "hit" : "hits" }];
    if (errors.distinct > 1) items.push({ n: errors.distinct, text: "distinct" });
    for (const one of errors.top ?? []) items.push({ n: `×${one.count}`, text: one.fingerprint, code: true });
    rows.push({ label: "Errors", items, tone: "warn", was: changed(errors.total, previous?.errors.total) });
  } else if (previous && previous.errors.total > 0) {
    rows.push({ label: "Errors", items: [{ n: 0, text: "hits" }], was: previous.errors.total });
  }

  if (version.load_failures > 0) {
    rows.push({
      label: "Load failed",
      items: [{ n: version.load_failures, text: version.load_failures === 1 ? "resource" : "resources" }],
      tone: "warn",
      was: changed(version.load_failures, previous?.load_failures),
    });
  } else if (previous && previous.load_failures > 0) {
    rows.push({ label: "Load failed", items: [{ n: 0, text: "resources" }], was: previous.load_failures });
  }

  return rows;
}

function changed(now: number, before: number | undefined): number | undefined {
  return before !== undefined && before !== now ? before : undefined;
}

const NAMES: Record<string, string> = {
  phone: "Mobile",
  tablet: "Tablet",
  desktop: "Desktop",
  chrome: "Chrome",
  safari: "Safari",
  firefox: "Firefox",
  wechat: "WeChat",
  ios: "iOS",
  android: "Android",
  windows: "Windows",
  macos: "macOS",
  linux: "Linux",
  discord: "Discord",
  plaza: "Plaza",
  direct: "Direct",
  other: "Other",
};

export function label(value: string | undefined): string {
  if (!value) return "Unknown";
  return NAMES[value] ?? value;
}

const SOURCES: Record<string, string> = {
  card: "Card",
  notice: "Notice",
  collection: "Collection",
  plaza: "Plaza",
  wechat: "WeChat",
  discord: "Discord",
  direct: "Direct",
};

export function sourceLabel(kind: string): string {
  return SOURCES[kind] ?? "Other";
}

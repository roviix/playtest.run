// 每版那段话在这里生成，不在控制面。
//
// 控制面只给数（DESIGN §3.5：只有计数和中位数，没有比例），句子是呈现：
// 手机上要短、`--json` 要原始数字、`playtest mcp` 要另一种说法。措辞钉在服务端的话，
// 每一处都得先把句子拆回数字。规则集中在这个文件，改文案只改这里。

import type { SourceTally, VersionResults, WorkKind } from "./api";

export function workKind(kind: WorkKind | undefined): string {
  if (kind === "article") return "Article";
  if (kind === "video") return "Video";
  return "Web";
}

export function audience(kind: WorkKind | undefined): string {
  if (kind === "article") return "Readers";
  if (kind === "video") return "Viewers";
  return "Playtesters";
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

export function sentences(version: VersionResults, kind: WorkKind | undefined = "web"): string[] {
  if (version.opened === 0) {
    return ["No visits yet."];
  }

  const lines: string[] = [];
  const dropped = version.dropped_before_first_frame;
  if (kind === "article") {
    lines.push(`${version.opened} visited the article. Visiting does not imply full read.`);
  } else if (kind === "video") {
    lines.push(`${version.opened} visited the video page. Opening page is not counted as full play.`);
  } else if (dropped === null || dropped === undefined) {
    lines.push(`${version.opened} opened, ${version.entered} clicked start.`);
    lines.push("playtest.js not integrated; unable to track drop-offs during load.");
  } else {
    const reached = Math.max(version.entered - dropped, 0);
    lines.push(
      dropped > 0
        ? `${version.opened} opened, ${reached} entered, ${dropped} dropped off during load.`
        : `${version.opened} opened, ${reached} entered.`,
    );
  }

  const stayed: string[] = [];
  if ((version.named ?? 0) > 0) stayed.push(`${version.named} left name`);
  if (kind === "web" && (version.played_5min_plus ?? 0) > 0) stayed.push(`${version.played_5min_plus} played 5m+`);
  if ((version.returned ?? 0) > 0) stayed.push(`${version.returned} returned`);
  if (version.dwell_median_s !== null && version.dwell_median_s !== undefined) {
    stayed.push(`median dwell ${seconds(version.dwell_median_s)}`);
  }
  if (stayed.length > 0) lines.push(`${stayed.join(", ")}.`);

  const from = sources(version.sources);
  if (from) lines.push(from);

  const errors = version.errors;
  if (errors.total > 0) {
    const counted =
      errors.distinct === 1 ? `1 error occurred ${errors.total} times` : `${errors.distinct} errors, ${errors.total} total`;
    const worst = errors.top?.[0]?.fingerprint;
    lines.push(worst ? `${counted}: ${worst}` : `${counted}.`);
  }

  if (version.load_failures > 0) lines.push(`${version.load_failures} load failure${version.load_failures > 1 ? "s" : ""}.`);
  if (version.feedback_count > 0) lines.push(`${version.feedback_count} feedback item${version.feedback_count > 1 ? "s" : ""}.`);

  return lines;
}

function sources(tally: SourceTally[] | undefined): string | null {
  const counted = (tally ?? []).filter((one) => one.count > 0);
  if (counted.length === 0) return null;
  return `From ${counted.map((one) => `${sourceLabel(one.kind)} ${one.count}`).join(" · ")}`;
}

export type VersusDelta = {
  label: string;
  kind: "better" | "worse" | "neutral";
};

export function versusDeltas(version: VersionResults, previous: VersionResults): {
  previousVersion: number;
  deltas: VersusDelta[];
} | null {
  const deltas: VersusDelta[] = [];

  const opened = version.opened - previous.opened;
  if (opened !== 0) {
    deltas.push({
      label: `opened ${opened > 0 ? "+" : "−"}${Math.abs(opened)}`,
      kind: opened > 0 ? "better" : "neutral",
    });
  }

  const entered = version.entered - previous.entered;
  if (entered !== 0) {
    deltas.push({
      label: `entered ${entered > 0 ? "+" : "−"}${Math.abs(entered)}`,
      kind: entered > 0 ? "better" : "neutral",
    });
  }

  if (version.played_5min_plus !== previous.played_5min_plus) {
    const playDiff = version.played_5min_plus - previous.played_5min_plus;
    deltas.push({
      label: `5m+ play ${playDiff > 0 ? "+" : "−"}${Math.abs(playDiff)}`,
      kind: playDiff > 0 ? "better" : "neutral",
    });
  }

  if (version.load_failures !== previous.load_failures) {
    const worse = version.load_failures > previous.load_failures;
    deltas.push({
      label: `load failures ${previous.load_failures} → ${version.load_failures}`,
      kind: worse ? "worse" : "better",
    });
  }

  if (version.errors.total !== previous.errors.total) {
    const worse = version.errors.total > previous.errors.total;
    deltas.push({
      label: `errors ${previous.errors.total} → ${version.errors.total}`,
      kind: worse ? "worse" : "better",
    });
  }

  if (version.feedback_count !== previous.feedback_count) {
    const fbDiff = version.feedback_count - previous.feedback_count;
    deltas.push({
      label: `feedback ${previous.feedback_count} → ${version.feedback_count}`,
      kind: fbDiff > 0 ? "better" : "neutral",
    });
  }

  if (deltas.length === 0) return null;
  return { previousVersion: previous.version, deltas };
}

export function versus(version: VersionResults, previous: VersionResults): string | null {
  const res = versusDeltas(version, previous);
  if (!res) return null;
  return `vs v${res.previousVersion}: ${res.deltas.map((d) => d.label).join(", ")}`;
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

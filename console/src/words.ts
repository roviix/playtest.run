// 每版那段话在这里生成，不在控制面。
//
// 控制面只给数（DESIGN §3.5：只有计数和中位数，没有比例），句子是呈现：
// 手机上要短、`--json` 要原始数字、`playtest mcp` 要另一种说法。措辞钉在服务端的话，
// 每一处都得先把句子拆回数字。规则集中在这个文件，改文案只改这里。

import type { SourceTally, VersionResults } from "./api";

/** 秒 → 人话。「45 秒」「3 分 20 秒」「1 小时 2 分」。 */
export function seconds(value: number): string {
  if (value < 60) return `${value} 秒`;
  if (value < 3600) {
    const rest = value % 60;
    return rest === 0 ? `${Math.floor(value / 60)} 分` : `${Math.floor(value / 60)} 分 ${rest} 秒`;
  }
  const hours = Math.floor(value / 3600);
  const minutes = Math.floor((value % 3600) / 60);
  return minutes === 0 ? `${hours} 小时` : `${hours} 小时 ${minutes} 分`;
}

/** RFC 3339 → 「9 月 5 日 14:20」，按看的人自己的时区。 */
export function moment(iso: string | undefined): string {
  if (!iso) return "";
  const at = new Date(iso);
  if (Number.isNaN(at.getTime())) return iso;
  const clock = `${pad(at.getHours())}:${pad(at.getMinutes())}`;
  return `${at.getMonth() + 1} 月 ${at.getDate()} 日 ${clock}`;
}

/** RFC 3339 → 「9 月 12 日」。推广的起止那种只关心哪一天的地方用。 */
export function day(iso: string | undefined): string {
  if (!iso) return "";
  const at = new Date(iso);
  if (Number.isNaN(at.getTime())) return iso;
  return `${at.getMonth() + 1} 月 ${at.getDate()} 日`;
}

/** RFC 3339 → 「14:20:05」。点名册和事件流里用，同一天的行不用重复日期。 */
export function clock(iso: string | undefined): string {
  if (!iso) return "";
  const at = new Date(iso);
  if (Number.isNaN(at.getTime())) return iso;
  return `${pad(at.getHours())}:${pad(at.getMinutes())}:${pad(at.getSeconds())}`;
}

function pad(value: number): string {
  return value.toString().padStart(2, "0");
}

/** 多久之前。「刚刚」「12 分钟前」「3 小时前」，隔了一天以上就回到具体日期。 */
export function ago(iso: string | undefined, now = Date.now()): string {
  if (!iso) return "";
  const at = new Date(iso).getTime();
  if (Number.isNaN(at)) return iso;
  const delta = Math.max(0, Math.floor((now - at) / 1000));
  if (delta < 60) return "刚刚";
  if (delta < 3600) return `${Math.floor(delta / 60)} 分钟前`;
  if (delta < 86400) return `${Math.floor(delta / 3600)} 小时前`;
  return moment(iso);
}

/** 到期还剩多久。「还剩 5 小时」「还剩 40 分钟」；已过期返回 null。 */
export function left(iso: string | undefined, now = Date.now()): string | null {
  if (!iso) return null;
  const at = new Date(iso).getTime();
  if (Number.isNaN(at)) return null;
  const delta = Math.floor((at - now) / 1000);
  if (delta <= 0) return null;
  if (delta < 3600) return `还剩 ${Math.max(1, Math.floor(delta / 60))} 分钟`;
  if (delta < 86400 * 2) return `还剩 ${Math.floor(delta / 3600)} 小时`;
  return `还剩 ${Math.floor(delta / 86400)} 天`;
}

/** 字节数 → 「1.2 MB」。版本清单里用，给开发者一个「这版有多大」的感觉。 */
export function bytes(value: number): string {
  if (value < 1024) return `${value} B`;
  if (value < 1024 * 1024) return `${Math.round(value / 1024)} KB`;
  if (value < 1024 * 1024 * 1024) return `${(value / 1024 / 1024).toFixed(1)} MB`;
  return `${(value / 1024 / 1024 / 1024).toFixed(2)} GB`;
}

/**
 * 一版的那段话。一句一行，数为 0 的句子不说——
 * 「0 条反馈」「0 个错误」占着地方却什么也没告诉人（DESIGN §3.5）。
 * 措辞按 §3.13「文案」：只报事实，「8 人打开」，不带括号和转折。
 */
export function sentences(version: VersionResults): string[] {
  if (version.opened === 0) {
    return ["还没有人打开。"];
  }

  const lines: string[] = [];
  const dropped = version.dropped_before_first_frame;
  if (dropped === null || dropped === undefined) {
    // 首帧要 SDK 才报得出来。没接就说没接，不拿「0 人在加载时离开」冒充。
    lines.push(`${version.opened} 人打开，${version.entered} 人点了开始。`);
    lines.push("没接 playtest.js，看不到加载时离开的人。");
  } else {
    const reached = Math.max(version.entered - dropped, 0);
    lines.push(
      dropped > 0
        ? `${version.opened} 人打开，${reached} 人进到游戏，${dropped} 人在加载时离开。`
        : `${version.opened} 人打开，${reached} 人进到游戏。`,
    );
  }

  const stayed: string[] = [];
  if ((version.named ?? 0) > 0) stayed.push(`${version.named} 人留名`);
  if ((version.played_5min_plus ?? 0) > 0) stayed.push(`${version.played_5min_plus} 人玩过 5 分钟`);
  if ((version.returned ?? 0) > 0) stayed.push(`${version.returned} 人回来过`);
  if (version.dwell_median_s !== null && version.dwell_median_s !== undefined) {
    stayed.push(`停留中位 ${seconds(version.dwell_median_s)}`);
  }
  if (stayed.length > 0) lines.push(`${stayed.join("，")}。`);

  // 三个环各带来了几个人，这一句是唯一的答案（DESIGN §3.5）。单独一行。
  const from = sources(version.sources);
  if (from) lines.push(from);

  const errors = version.errors;
  if (errors.total > 0) {
    const counted =
      errors.distinct === 1 ? `1 个错误，出现 ${errors.total} 次` : `${errors.distinct} 个错误，共 ${errors.total} 次`;
    const worst = errors.top?.[0]?.fingerprint;
    lines.push(worst ? `${counted}：${worst}` : `${counted}。`);
  }

  if (version.load_failures > 0) lines.push(`${version.load_failures} 次加载失败。`);
  if (version.feedback_count > 0) lines.push(`${version.feedback_count} 条反馈。`);

  return lines;
}

/** 「来自 邀请卡 4 · 广场 2 · 微信 2」。一个来源都没有就返回 null，整句不说。 */
function sources(tally: SourceTally[] | undefined): string | null {
  const counted = (tally ?? []).filter((one) => one.count > 0);
  if (counted.length === 0) return null;
  return `来自 ${counted.map((one) => `${sourceLabel(one.kind)} ${one.count}`).join(" · ")}`;
}

/** 和上一版比的那一行：「比 v6：打开 +3，加载失败 2 → 0，反馈 1 → 2」。没有一处变化就不说话。 */
export function versus(version: VersionResults, previous: VersionResults): string | null {
  const bits: string[] = [];

  const opened = version.opened - previous.opened;
  if (opened !== 0) bits.push(`打开 ${opened > 0 ? "+" : "−"}${Math.abs(opened)}`);
  if (version.load_failures !== previous.load_failures) {
    bits.push(`加载失败 ${previous.load_failures} → ${version.load_failures}`);
  }
  if (version.errors.total !== previous.errors.total) {
    bits.push(`错误 ${previous.errors.total} → ${version.errors.total}`);
  }
  if (version.feedback_count !== previous.feedback_count) {
    bits.push(`反馈 ${previous.feedback_count} → ${version.feedback_count}`);
  }

  if (bits.length === 0) return null;
  return `比 v${previous.version}：${bits.join("，")}`;
}

/** 设备 / 浏览器 / 系统的中文说法。库里存的是英文小写。 */
const NAMES: Record<string, string> = {
  phone: "手机",
  tablet: "平板",
  desktop: "电脑",
  chrome: "Chrome",
  safari: "Safari",
  firefox: "Firefox",
  wechat: "微信",
  ios: "iOS",
  android: "Android",
  windows: "Windows",
  macos: "macOS",
  linux: "Linux",
  discord: "Discord",
  plaza: "广场",
  direct: "直接打开",
  other: "其它",
};

export function label(value: string | undefined): string {
  if (!value) return "不知道";
  return NAMES[value] ?? value;
}

/**
 * 「来自哪里」的中文说法。抄的是 common/src/ingest.rs 的 `source::label`，
 * 连不认识的值说「其它」这一条也一样——那边加了新来源，这边照着补一行。
 */
const SOURCES: Record<string, string> = {
  card: "邀请卡",
  notice: "通知",
  collection: "合集",
  plaza: "广场",
  wechat: "微信",
  discord: "Discord",
  direct: "直接打开",
};

export function sourceLabel(kind: string): string {
  return SOURCES[kind] ?? "其它";
}

// 每版那段话在这里生成，不在控制面。
//
// 控制面只给数（DESIGN §3.4：只有计数和中位数，没有比例），句子是呈现：
// 手机上要短、`--json` 要原始数字、`playtest mcp` 要另一种说法。措辞钉在服务端的话，
// 每一处都得先把句子拆回数字。规则集中在这个文件，改文案只改这里。

import type { VersionResults } from "./api";

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

/**
 * 一版的那段话。一句一行，数为 0 的句子不说——
 * 「0 条反馈」「0 个错误」占着地方却什么也没告诉人（DESIGN §3.4）。
 */
export function sentences(version: VersionResults): string[] {
  if (version.opened === 0) {
    return ["还没有人打开这一版。"];
  }

  const lines: string[] = [];
  const dropped = version.dropped_before_first_frame;
  if (dropped === null || dropped === undefined) {
    // 首帧要 SDK 才报得出来。没接就说没接，不拿「0 个人在加载时走了」冒充。
    lines.push(`${version.opened} 个人打开，${version.entered} 个人点了开始。`);
    lines.push("这一版没接 playtest.js，加载时走掉几个人看不出来。");
  } else {
    const reached = Math.max(version.entered - dropped, 0);
    lines.push(
      dropped > 0
        ? `${version.opened} 个人打开，${reached} 个人进到游戏（${dropped} 个在加载时走了）。`
        : `${version.opened} 个人打开，${reached} 个人进到游戏。`,
    );
  }

  const stayed: string[] = [];
  if (version.played_5min_plus > 0) stayed.push(`${version.played_5min_plus} 个人玩了 5 分钟以上`);
  if (version.returned > 0) stayed.push(`${version.returned} 个人回来过`);
  if (version.dwell_median_s !== null && version.dwell_median_s !== undefined) {
    stayed.push(`停留中位数 ${seconds(version.dwell_median_s)}`);
  }
  if (stayed.length > 0) lines.push(`${stayed.join("，")}。`);

  const errors = version.errors;
  if (errors.total > 0) {
    const counted =
      errors.distinct === 1
        ? `1 个错误撞了 ${errors.total} 次`
        : `${errors.distinct} 种错误一共撞了 ${errors.total} 次`;
    const worst = errors.top?.[0]?.fingerprint;
    lines.push(worst ? `${counted}（${worst}）。` : `${counted}。`);
  }

  if (version.load_failures > 0) lines.push(`${version.load_failures} 次资源没加载出来。`);
  if (version.feedback_count > 0) lines.push(`${version.feedback_count} 条反馈。`);

  return lines;
}

/** 和上一版比的那一行。没有一处变化就不说话。 */
export function versus(version: VersionResults, previous: VersionResults): string | null {
  const bits: string[] = [];

  const opened = version.opened - previous.opened;
  if (opened !== 0) bits.push(`${opened > 0 ? "多" : "少"} ${Math.abs(opened)} 个人打开`);

  if (version.load_failures !== previous.load_failures) {
    bits.push(`加载失败从 ${previous.load_failures} ${moved(previous.load_failures, version.load_failures)}到 ${version.load_failures}`);
  }
  if (version.errors.total !== previous.errors.total) {
    bits.push(
      `错误从 ${previous.errors.total} 次${moved(previous.errors.total, version.errors.total)}到 ${version.errors.total} 次`,
    );
  }
  if (version.feedback_count !== previous.feedback_count) {
    bits.push(
      `反馈从 ${previous.feedback_count} 条${moved(previous.feedback_count, version.feedback_count)}到 ${version.feedback_count} 条`,
    );
  }

  if (bits.length === 0) return null;
  return `比 v${previous.version} ${bits.join("，")}。`;
}

function moved(before: number, after: number): string {
  return after < before ? "降" : "升";
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

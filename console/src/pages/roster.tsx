// 点名册（DESIGN §3.5、§3.13）：一版一张，每人一行，默认停留最短的排最前面。
//
// 「排在最前面的人就是你要看的人」——所以默认排序不是时间：时间序好看，
// 但它把最该看的那一行埋在中间。点一行展开这个人的事件与错误。
// 版本在顶上切，切换也是换地址（router.ts），能把「v7 的点名册」直接发给同伴。

import { useState } from "preact/hooks";

import type { RosterSort, SessionRow, VersionResults } from "../api";
import { api } from "../api";
import { useLoad } from "../load";
import { href } from "../router";
import { clock, label, moment, seconds, sourceLabel } from "../words";
import { Empty, Failed, Loading } from "./status";

const KINDS: Record<string, string> = {
  gate_view: "看到门禁页",
  start: "点了开始",
  html_view: "打开页面",
  report: "举报",
  breaker_trip: "熔断",
  resource_fail: "资源没加载出来",
  load: "首帧",
  input: "有输入",
  error: "错误",
  event: "自定义事件",
};

type Tag = { text: string; tone: "warn" | "good" | "plain" };

export function RosterTab({
  slug,
  version,
  versions,
}: {
  slug: string;
  version: number | undefined;
  versions: VersionResults[];
}) {
  const [sort, setSort] = useState<RosterSort>("dwell");

  if (version === undefined) {
    return (
      <Empty>
        <p>还没有版本，也就还没有人。</p>
      </Empty>
    );
  }

  return (
    <>
      <div class="roster-bar">
        <nav class="chips-nav" aria-label="版本">
          {versions.map((one) => (
            <a
              key={one.version}
              class={`chip-link ${one.version === version ? "active" : ""}`}
              href={href({ name: "site", slug, tab: "roster", version: one.version })}
            >
              v{one.version}
              {one.opened > 0 ? <span class="chip-count">{one.opened}</span> : null}
            </a>
          ))}
        </nav>
        <div class="switch">
          <button class={sort === "dwell" ? "on" : ""} onClick={() => setSort("dwell")} type="button">
            停留最短在前
          </button>
          <button class={sort === "time" ? "on" : ""} onClick={() => setSort("time")} type="button">
            最近打开在前
          </button>
        </div>
      </div>
      <Rows slug={slug} version={version} sort={sort} />
    </>
  );
}

function Rows({ slug, version, sort }: { slug: string; version: number; sort: RosterSort }) {
  const { data, error, loading, reload } = useLoad(
    () => api.sessions(slug, version, sort),
    [slug, version, sort],
  );

  if (loading) return <Loading />;
  if (error) return <Failed error={error} onRetry={reload} />;
  if (!data) return null;

  if (data.sessions.length === 0) {
    return (
      <Empty>
        <p>v{version} 还没有人打开。</p>
        <p class="muted">把链接发出去，第一个人点开就会出现在这里。</p>
      </Empty>
    );
  }

  // 首帧要 SDK 才报得出来。整版一个人都没报过，就不能把每一行都标成「没等到首帧」。
  const anyFirstFrame = data.sessions.some((session) => session.first_frame);

  return (
    <>
      {anyFirstFrame ? null : (
        <p class="notice soft">这一版没接 playtest.js：首帧、玩到哪、最后一次输入都看不到。</p>
      )}
      <ul class="sessions">
        {data.sessions.map((session) => (
          <Session key={session.id} session={session} anyFirstFrame={anyFirstFrame} />
        ))}
      </ul>
    </>
  );
}

function Session({ session, anyFirstFrame }: { session: SessionRow; anyFirstFrame: boolean }) {
  const [open, setOpen] = useState(false);
  const events = session.events ?? [];

  return (
    <li class={`session ${open ? "open" : ""}`}>
      <button class="session-head" type="button" onClick={() => setOpen(!open)} aria-expanded={open}>
        <span class="dwell mono">{seconds(session.dwell_s)}</span>
        <span class="session-who">
          <span>
            {/* 留了名字的人是这一行的主角；没留的仍然要有个能对上的记号，
                所以退回会话 id 的头几位——展开之后看到的是同一个 id。 */}
            {session.name ? (
              <b class="who">{session.name}</b>
            ) : (
              <span class="who mono muted">{session.id.slice(0, 6)}</span>
            )}{" "}
            <span class="muted">
              · {label(session.device)} · {label(session.browser)} · {label(session.os)}
            </span>
          </span>
          <span class="muted">{moment(session.at)} 打开</span>
        </span>
        <span class="muted expand">{open ? "收起" : "展开"}</span>
      </button>

      <p class="tags">
        {tagsOf(session, anyFirstFrame).map((tag) => (
          <span key={tag.text} class={`tag-pill ${tag.tone}`}>
            {tag.text}
          </span>
        ))}
      </p>

      {open ? (
        <div class="events">
          {events.length === 0 ? (
            <p class="muted">这个会话没有事件。错误、加载用时、自定义事件都来自 playtest.js。</p>
          ) : (
            <ol>
              {events.map((event, index) => (
                <li key={`${event.ts}-${index}`}>
                  <span class="mono">{clock(event.ts)}</span>
                  <span class={`tag-pill ${event.kind === "error" ? "warn" : "plain"}`}>
                    {KINDS[event.kind] ?? event.kind}
                  </span>
                  {event.name ? <span class="event-name">{event.name}</span> : null}
                  {event.data ? <code class="mono">{JSON.stringify(event.data)}</code> : null}
                </li>
              ))}
            </ol>
          )}
          {session.more_events ? (
            <p class="muted">事件太多，这里只显示最早的 100 条。</p>
          ) : null}
          <p class="muted mono">会话 {session.id}</p>
        </div>
      ) : null}
    </li>
  );
}

function tagsOf(session: SessionRow, anyFirstFrame: boolean): Tag[] {
  const tags: Tag[] = [];

  if (!session.entered) {
    tags.push({ text: "没进到游戏", tone: "warn" });
  } else if (session.first_frame) {
    tags.push({ text: "进到游戏", tone: "plain" });
  } else if (session.started && anyFirstFrame) {
    tags.push({ text: "点了开始，没等到首帧", tone: "warn" });
  } else {
    tags.push({ text: "点了开始", tone: "plain" });
  }

  if (session.reached) tags.push({ text: `玩到「${session.reached}」`, tone: "good" });
  if (session.errors > 0) tags.push({ text: `${session.errors} 个错误`, tone: "warn" });
  if (session.feedback > 0) tags.push({ text: "留了话", tone: "good" });
  if (session.is_return) tags.push({ text: "回头的", tone: "plain" });
  // 「来自邀请卡」和「微信里打开」是两件事：扫卡的人多半也在微信里，两个都说
  // 才答得上「我发出去的那张卡带来了谁」（common/src/ingest.rs 的 source_kind）。
  if (session.referrer_kind && session.referrer_kind !== "wechat") {
    tags.push({ text: `来自${sourceLabel(session.referrer_kind)}`, tone: "plain" });
  }
  if (session.wechat) tags.push({ text: "微信里打开", tone: "plain" });
  if (session.last_input_after_s !== null && session.last_input_after_s !== undefined) {
    tags.push({ text: `最后一次动手在进入后 ${seconds(session.last_input_after_s)}`, tone: "plain" });
  }

  return tags;
}

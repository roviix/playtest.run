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
  gate_view: "Saw door page",
  start: "Clicked start",
  html_view: "Opened page",
  report: "Reported",
  breaker_trip: "Circuit breaker tripped",
  resource_fail: "Resource load failed",
  load: "First frame",
  input: "Input received",
  error: "Error",
  event: "Custom event",
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
      <Empty
        icon={
          <svg class="icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">
            <rect x="2" y="3" width="20" height="14" rx="2" ry="2" />
            <line x1="8" y1="21" x2="16" y2="21" />
            <line x1="12" y1="17" x2="12" y2="21" />
          </svg>
        }
        title="No versions published yet"
        description="Run playtest in your project folder to upload your first build and start receiving visitors."
      />
    );
  }

  return (
    <>
      <div class="roster-bar">
        <nav class="chips-nav" aria-label="Version">
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
            Shortest dwell first
          </button>
          <button class={sort === "time" ? "on" : ""} onClick={() => setSort("time")} type="button">
            Recently opened first
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
      <Empty
        icon={
          <svg class="icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">
            <path d="M17 21v-2a4 4 0 0 0-4-4H5a4 4 0 0 0-4 4v2" />
            <circle cx="9" cy="7" r="4" />
            <path d="M23 21v-2a4 4 0 0 0-3-3.87" />
            <path d="M16 3.13a4 4 0 0 1 0 7.75" />
          </svg>
        }
        title={`No visits on v${version} yet`}
        description="Share your link with playtesters. As players open the page, their sessions, device info, and dwell times will appear here."
      />
    );
  }

  const anyFirstFrame = data.sessions.some((session) => session.first_frame);

  return (
    <>
      {anyFirstFrame ? null : (
        <p class="notice soft">playtest.js not integrated: unable to track first frame, progress, or errors.</p>
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
            {session.name ? (
              <b class="who">{session.name}</b>
            ) : (
              <span class="who mono muted">{session.id.slice(0, 6)}</span>
            )}{" "}
            <span class="muted">
              · {label(session.device)} · {label(session.browser)} · {label(session.os)}
            </span>
          </span>
          <span class="muted">Opened {moment(session.at)}</span>
        </span>
        <span class="muted expand">{open ? "Collapse" : "Expand"}</span>
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
            <p class="muted">No events.</p>
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
            <p class="muted">Showing earliest 100 events only.</p>
          ) : null}
          <p class="muted mono">Session {session.id}</p>
        </div>
      ) : null}
    </li>
  );
}

function tagsOf(session: SessionRow, anyFirstFrame: boolean): Tag[] {
  const tags: Tag[] = [];

  if (!session.entered) {
    tags.push({ text: "Did not enter game", tone: "warn" });
  } else if (session.first_frame) {
    tags.push({ text: "Entered game", tone: "plain" });
  } else if (session.started && anyFirstFrame) {
    tags.push({ text: "Dropped before first frame", tone: "warn" });
  } else {
    tags.push({ text: "Clicked start", tone: "plain" });
  }

  if (session.reached) tags.push({ text: `Reached "${session.reached}"`, tone: "good" });
  if (session.errors > 0) tags.push({ text: `${session.errors} error${session.errors > 1 ? "s" : ""}`, tone: "warn" });
  if (session.feedback > 0) tags.push({ text: "Left feedback", tone: "good" });
  if (session.is_return) tags.push({ text: "Returned", tone: "plain" });
  if (session.referrer_kind && session.referrer_kind !== "wechat") {
    tags.push({ text: `From ${sourceLabel(session.referrer_kind)}`, tone: "plain" });
  }
  if (session.wechat) tags.push({ text: "Opened in WeChat", tone: "plain" });
  if (session.last_input_after_s !== null && session.last_input_after_s !== undefined) {
    tags.push({ text: `Last input ${seconds(session.last_input_after_s)} after entry`, tone: "plain" });
  }

  return tags;
}

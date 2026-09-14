// 反馈（DESIGN §3.5、§3.13）：玩家写的一句话是主体，加上让这句话能被定位的上下文。
//
// 标「看过了 / 处理完了」是给自己用的，不回给玩家——玩家写完就走了，不留身份。
// 「隐藏」不一样：它玩家看得见，因为开了公开反馈之后这一条正挂在门禁页上。

import { useState } from "preact/hooks";

import type { FeedbackItem, FeedbackStatus, Site } from "../api";
import { api, ApiError } from "../api";
import { useLoad } from "../load";
import { href } from "../router";
import { audience, label, moment, seconds } from "../words";
import { Empty, Failed, Loading } from "./status";

const STATUS_TEXT: Record<FeedbackStatus, string> = {
  new: "New",
  seen: "Reviewed",
  done: "Resolved",
};

export function FeedbackTab({ slug, site }: { slug: string; site: Site }) {
  const { data, error, loading, reload } = useLoad(() => api.feedback(slug), [slug]);
  const [changed, setChanged] = useState<Record<number, FeedbackItem>>({});
  const [failed, setFailed] = useState<string>("");

  const feedbackPublic = site.listing?.feedback_public ?? false;

  if (loading) return <Loading />;
  if (error) return <Failed error={error} onRetry={reload} />;
  if (!data) return null;

  if (data.items.length === 0) {
    return (
      <Empty
        icon={
          <svg class="icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">
            <path d="M21 15a2 2 0 0 1-2 2H7l-4 4V5a2 2 0 0 1 2-2h14a2 2 0 0 1 2 2z" />
          </svg>
        }
        title="No feedback collected yet"
        description="When players try your build and leave impressions or bug reports on the door page, their quotes will appear here."
      />
    );
  }

  async function patch(item: FeedbackItem, request: Parameters<typeof api.updateFeedback>[2]) {
    setFailed("");
    try {
      const updated = await api.updateFeedback(slug, item.id, request);
      setChanged((all) => ({ ...all, [item.id]: updated }));
    } catch (err) {
      setFailed(err instanceof ApiError ? err.message : "Failed to update.");
    }
  }

  const items = [...data.items].sort((a, b) => b.ts.localeCompare(a.ts));

  return (
    <>
      <p class="tab-lead muted">
        {feedbackPublic
          ? "Public feedback is on: Door page displays the 3 latest entries."
          : "Public feedback is off: Only you can see feedback."}
      </p>
      {failed ? <p class="notice">{failed}</p> : null}
      <ul class="quotes">
        {items.map((original) => {
          const item = changed[original.id] ?? original;
          return (
            <li key={item.id} class={`quote-card ${item.status}`}>
              <p class="quote">{item.text}</p>
              <p class="muted quote-meta">
                <b class="who">{item.name ?? `A ${audience(site.kind)}`}</b> · v{item.version} · {moment(item.ts)} ·{" "}
                {label(item.device)} {label(item.browser)}
                {item.seconds_in !== undefined ? ` · In ${seconds(item.seconds_in)}` : ""}
              </p>
              <p class="row-actions">
                <span class={`tag-pill ${item.status === "new" ? "warn" : "plain"}`}>
                  {STATUS_TEXT[item.status]}
                </span>
                {feedbackPublic ? (
                  <span class={`tag-pill ${item.public ? "good" : "plain"}`}>
                    {item.public ? "Public" : "Hidden"}
                  </span>
                ) : null}
                {item.status === "new" ? (
                  <button class="button small" type="button" onClick={() => patch(item, { status: "seen" })}>
                    Mark Reviewed
                  </button>
                ) : null}
                {item.status !== "done" ? (
                  <button class="button small" type="button" onClick={() => patch(item, { status: "done" })}>
                    Resolve
                  </button>
                ) : (
                  <button class="button small quiet" type="button" onClick={() => patch(item, { status: "new" })}>
                    Reopen
                  </button>
                )}
                {feedbackPublic ? (
                  <button class="button small quiet" type="button" onClick={() => patch(item, { public: !item.public })}>
                    {item.public ? "Hide" : "Unhide"}
                  </button>
                ) : null}
                <a href={href({ name: "site", slug, tab: "roster", version: item.version })}>View roster for this version →</a>
              </p>
            </li>
          );
        })}
      </ul>
    </>
  );
}

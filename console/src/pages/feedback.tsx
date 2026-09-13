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
  new: "还没看",
  seen: "看过了",
  done: "处理完了",
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
      <Empty>
        <p>还没有反馈。</p>
      </Empty>
    );
  }

  async function patch(item: FeedbackItem, request: Parameters<typeof api.updateFeedback>[2]) {
    setFailed("");
    try {
      const updated = await api.updateFeedback(slug, item.id, request);
      setChanged((all) => ({ ...all, [item.id]: updated }));
    } catch (err) {
      setFailed(err instanceof ApiError ? err.message : "没改成。");
    }
  }

  const items = [...data.items].sort((a, b) => b.ts.localeCompare(a.ts));

  return (
    <>
      <p class="tab-lead muted">
        {feedbackPublic
          ? "公开反馈开着：门禁页显示最近 3 条。"
          : "公开反馈关着：只有你看得到。"}
      </p>
      {failed ? <p class="notice">{failed}</p> : null}
      <ul class="quotes">
        {items.map((original) => {
          const item = changed[original.id] ?? original;
          return (
            <li key={item.id} class={`quote-card ${item.status}`}>
              <p class="quote">{item.text}</p>
              <p class="muted quote-meta">
                {/* 没留名字的人也是一个人，不显示会话 id——那是给点名册对行用的。 */}
                <b class="who">{item.name ?? `一位${audience(site.kind)}`}</b> · v{item.version} · {moment(item.ts)} ·{" "}
                {label(item.device)} {label(item.browser)}
                {item.seconds_in !== undefined ? ` · 进入 ${seconds(item.seconds_in)}` : ""}
              </p>
              <p class="row-actions">
                <span class={`tag-pill ${item.status === "new" ? "warn" : "plain"}`}>
                  {STATUS_TEXT[item.status]}
                </span>
                {feedbackPublic ? (
                  <span class={`tag-pill ${item.public ? "good" : "plain"}`}>
                    {item.public ? "公开中" : "已隐藏"}
                  </span>
                ) : null}
                {item.status === "new" ? (
                  <button class="button small" type="button" onClick={() => patch(item, { status: "seen" })}>
                    看过了
                  </button>
                ) : null}
                {item.status !== "done" ? (
                  <button class="button small" type="button" onClick={() => patch(item, { status: "done" })}>
                    处理完了
                  </button>
                ) : (
                  <button class="button small quiet" type="button" onClick={() => patch(item, { status: "new" })}>
                    退回
                  </button>
                )}
                {feedbackPublic ? (
                  <button class="button small quiet" type="button" onClick={() => patch(item, { public: !item.public })}>
                    {item.public ? "隐藏" : "恢复"}
                  </button>
                ) : null}
                <a href={href({ name: "site", slug, tab: "roster", version: item.version })}>看这一版的人 →</a>
              </p>
            </li>
          );
        })}
      </ul>
    </>
  );
}

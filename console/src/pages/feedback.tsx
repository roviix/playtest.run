// 反馈流：玩家写的一句话，加上让这句话能被定位的上下文。
//
// 标「已看 / 已处理」是给自己用的，不回给玩家——玩家写完就走了，不留身份（DESIGN §3.4）。

import { useState } from "preact/hooks";

import type { FeedbackItem, FeedbackStatus } from "../api";
import { api, ApiError } from "../api";
import { useLoad } from "../load";
import { href } from "../router";
import { label, moment, seconds } from "../words";
import { Empty, Failed, Loading } from "./status";

const STATUS_TEXT: Record<FeedbackStatus, string> = {
  new: "还没看",
  seen: "看过了",
  done: "处理完了",
};

export function FeedbackPage({ slug }: { slug: string }) {
  const { data, error, loading, reload } = useLoad(() => api.feedback(slug), [slug]);
  const [changed, setChanged] = useState<Record<number, FeedbackStatus>>({});
  const [failed, setFailed] = useState<string>("");

  const head = (
    <header class="page-head">
      <p class="muted">
        <a href={href({ name: "timeline", slug })}>← {slug}</a>
      </p>
      <h1>反馈</h1>
    </header>
  );

  if (loading) return <>{head}<Loading /></>;
  if (error) return <>{head}<Failed error={error} onRetry={reload} /></>;
  if (!data) return head;

  if (data.items.length === 0) {
    return (
      <>
        {head}
        <Empty>
          <p>还没有人留话。</p>
          <p class="muted">反馈按钮来自 playtest.js，玩家写一句话就会出现在这里。</p>
        </Empty>
      </>
    );
  }

  async function mark(item: FeedbackItem, status: FeedbackStatus) {
    setFailed("");
    try {
      const updated = await api.markFeedback(slug, item.id, status);
      setChanged((all) => ({ ...all, [item.id]: updated.status }));
    } catch (err) {
      setFailed(err instanceof ApiError ? err.message : "没能改这条的状态。");
    }
  }

  return (
    <>
      {head}
      {failed ? <p class="notice">{failed}</p> : null}
      <ul class="cards">
        {data.items.map((item) => {
          const status = changed[item.id] ?? item.status;
          return (
            <li key={item.id} class={`card feedback ${status}`}>
              <p class="quote">{item.text}</p>
              <p class="muted">
                v{item.version} · {moment(item.ts)} · {label(item.device)} {label(item.browser)}
                {item.seconds_in !== undefined ? ` · 进入 ${seconds(item.seconds_in)}` : ""}
              </p>
              <p class="row-actions">
                <span class={`tag ${status === "new" ? "warn" : "plain"}`}>
                  {STATUS_TEXT[status]}
                </span>
                {status !== "seen" ? (
                  <button class="button" type="button" onClick={() => mark(item, "seen")}>
                    标为看过了
                  </button>
                ) : null}
                {status !== "done" ? (
                  <button class="button" type="button" onClick={() => mark(item, "done")}>
                    标为处理完了
                  </button>
                ) : null}
                {status !== "new" ? (
                  <button class="button" type="button" onClick={() => mark(item, "new")}>
                    退回还没看
                  </button>
                ) : null}
                <a href={href({ name: "roster", slug, version: item.version })}>看这一版的人</a>
              </p>
            </li>
          );
        })}
      </ul>
    </>
  );
}

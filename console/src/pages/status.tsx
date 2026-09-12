// 各页共用的三块：在取、没取到、什么都没有。
//
// 「什么都没有」单独做一块是因为它在这个产品里出现得最多——刚发出去的链接、
// 还没人打开的版本、一条反馈都没有的作品。空状态要说清「接下来会有什么」，
// 不是画一个灰色的方框（AGENTS 第 4 条：做不到的地方在界面上明说）。

import type { ComponentChildren } from "preact";

import { ApiError } from "../api";
import { href } from "../router";

export function Loading() {
  return <div class="loading" role="status" aria-live="polite"><span class="muted">正在加载…</span><div class="skeleton" aria-hidden="true"><span /><span /><span /></div></div>;
}

export function Failed({ error, onRetry }: { error: ApiError; onRetry: () => void }) {
  if (error.needsToken) {
    return (
      <div class="notice">
        <p>{error.message}</p>
        <p>
          <a class="button small" href={href({ name: "token" })}>
            去登录
          </a>
        </p>
      </div>
    );
  }
  return (
    <div class="notice" role="alert">
      <p>{error.message}</p>
      <p>
        <button class="button small" onClick={onRetry}>
          再试一次
        </button>
        <a class="docs-context-link" href={href({ name: "docs", section: "troubleshoot" })}>查看连接与身份排查 →</a>
      </p>
    </div>
  );
}

export function Empty({ children }: { children: ComponentChildren }) {
  return <div class="empty">{children}</div>;
}

// 控制台的外壳：一条顶栏 + 当前页。
//
// 这里只有四个页面（作品列表、时间线、点名册、反馈流），没有仪表盘、没有设置页。
// 加第五个页面之前先回答 DESIGN §1.2 的哪一个时刻因此更短（AGENTS 第 5 条）。

import { readToken } from "./api";
import { href, useRoute } from "./router";
import { FeedbackPage } from "./pages/feedback";
import { RosterPage } from "./pages/roster";
import { SitesPage } from "./pages/sites";
import { TimelinePage } from "./pages/timeline";
import { TokenPage } from "./pages/token";

export function App() {
  const route = useRoute();
  const hasToken = readToken() !== "";

  return (
    <div class="shell">
      <nav class="top">
        <a class="brand" href={href({ name: "sites" })}>
          playtest
        </a>
        <span class="top-links">
          <a href="https://playtest.run/">广场</a>
          <a href={href({ name: "token" })}>{hasToken ? "换令牌" : "粘贴令牌"}</a>
        </span>
      </nav>
      <main>{page(route, hasToken)}</main>
    </div>
  );
}

function page(route: ReturnType<typeof useRoute>, hasToken: boolean) {
  if (route.name === "token") return <TokenPage />;
  if (!hasToken) return <TokenPage />;

  switch (route.name) {
    case "sites":
      return <SitesPage />;
    case "timeline":
      return <TimelinePage slug={route.slug} />;
    case "roster":
      return <RosterPage slug={route.slug} version={route.version} />;
    case "feedback":
      return <FeedbackPage slug={route.slug} />;
  }
}

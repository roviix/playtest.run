// 控制台的外壳：一条顶栏 + 当前页。
//
// 这里只有四个页面（作品列表、时间线、点名册、反馈流），没有仪表盘、没有设置页。
// 加第五个页面之前先回答 DESIGN §1.2 的哪一个时刻因此更短（AGENTS 第 5 条）。

import { useEffect, useState } from "preact/hooks";

import { api, exchangeGitHubCode, readToken, saveToken, type Me } from "./api";
import { href, useRoute } from "./router";
import { FeedbackPage } from "./pages/feedback";
import { RosterPage } from "./pages/roster";
import { SitesPage } from "./pages/sites";
import { TimelinePage } from "./pages/timeline";
import { TokenPage } from "./pages/token";

/** GitHub 授权完回到这里时地址上挂着的两样东西。 */
function callbackParams(): { code: string; state: string } | null {
  const query = new URLSearchParams(location.search);
  const code = query.get("code");
  const state = query.get("state");
  return code && state ? { code, state } : null;
}

export function App() {
  const route = useRoute();
  const [token, setToken] = useState(readToken());
  const [me, setMe] = useState<Me | null>(null);
  const [callback, setCallback] = useState<"working" | string | null>(() =>
    callbackParams() ? "working" : null,
  );

  // GitHub 回来了：换令牌，把 code 从地址栏上擦掉（刷新不该再换一次），进作品列表。
  useEffect(() => {
    const params = callbackParams();
    if (!params) return;
    exchangeGitHubCode(params.code, params.state)
      .then((login) => {
        saveToken(login.token);
        setToken(login.token);
        setCallback(null);
        history.replaceState(null, "", `${location.pathname}#/`);
      })
      .catch((err: Error) => {
        setCallback(err.message);
        history.replaceState(null, "", `${location.pathname}#/token`);
      });
  }, []);

  useEffect(() => {
    if (!token) {
      setMe(null);
      return;
    }
    api
      .me()
      .then(setMe)
      .catch(() => setMe(null));
  }, [token]);

  const hasToken = token !== "";
  const who = me?.kind === "github" ? `@${me.login ?? me.display_name}` : null;

  return (
    <div class="shell">
      <nav class="top">
        <a class="brand" href={href({ name: "sites" })}>
          playtest
        </a>
        <span class="top-links">
          <a href="https://playtest.run/">广场</a>
          <a href={href({ name: "token" })}>{who ?? (hasToken ? "匿名 · 换令牌" : "登录")}</a>
        </span>
      </nav>
      <main>
        {callback === "working" ? (
          <p class="muted">正在从 GitHub 回来……</p>
        ) : (
          page(route, hasToken, callback, me)
        )}
      </main>
    </div>
  );
}

function page(
  route: ReturnType<typeof useRoute>,
  hasToken: boolean,
  loginError: string | null,
  me: Me | null,
) {
  if (route.name === "token" || !hasToken) return <TokenPage loginError={loginError} me={me} />;

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

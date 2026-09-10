// 控制台的外壳：左边一条栏，右边一张台面（DESIGN §3.13）。
//
// 和广场（edge/src/plaza.rs）是同一个壳：同样的栏、同样的夜色、同一支绿。
// 开发者是这个产品里唯一同时进两间房的人，两套皮会让他觉得是两个产品。
//
// 栏上只有三样：作品清单、去广场、账号；栏底一张便条说「发下一版」的命令。
// 没有仪表盘、没有通知中心、没有设置菜单——加第四样之前先回答 DESIGN §1.2 的哪一格因此更短。

import { useEffect, useState } from "preact/hooks";

import { api, exchangeGitHubCode, readToken, saveToken, type Me, type Site } from "./api";
import { useLoad, type Loaded } from "./load";
import { href, useRoute, type Route } from "./router";
import { HomePage } from "./pages/home";
import { SitePage } from "./pages/site";
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

  // GitHub 回来了：换令牌，把 code 从地址栏上擦掉（刷新不该再换一次），进作品墙。
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
  // 作品清单在壳上取一次，栏和作品墙共用；作品页改了设置之后叫 reload 让栏上的点跟着变。
  const sites = useLoad(() => (hasToken ? api.sites() : Promise.resolve([] as Site[])), [token]);
  const plazaUrl = plazaOf(sites.data);

  return (
    <div class="shell">
      <Rail route={route} me={me} hasToken={hasToken} sites={sites.data ?? []} plazaUrl={plazaUrl} />
      <main class="stage">
        {callback === "working" ? (
          <p class="muted stage-note">正在从 GitHub 回来……</p>
        ) : (
          page(route, hasToken, callback, me, sites, plazaUrl)
        )}
      </main>
    </div>
  );
}

function page(
  route: Route,
  hasToken: boolean,
  loginError: string | null,
  me: Me | null,
  sites: Loaded<Site[]>,
  plazaUrl: string,
) {
  if (route.name === "token" || !hasToken) return <TokenPage loginError={loginError} me={me} />;

  switch (route.name) {
    case "sites":
      return <HomePage sites={sites} me={me} />;
    case "site":
      return (
        <SitePage
          key={route.slug}
          slug={route.slug}
          tab={route.tab}
          version={route.version}
          plazaUrl={plazaUrl}
          onSiteChanged={sites.reload}
        />
      );
  }
}

// ------------------------------------------------------------------ 栏

function Rail({
  route,
  me,
  hasToken,
  sites,
  plazaUrl,
}: {
  route: Route;
  me: Me | null;
  hasToken: boolean;
  sites: Site[];
  plazaUrl: string;
}) {
  const who = me?.kind === "github" ? `@${me.login ?? me.display_name}` : null;
  const currentSlug = route.name === "site" ? route.slug : null;

  return (
    <aside class="rail">
      <a class="brand" href={href({ name: "sites" })}>
        <span class="wordmark">
          playtest<span>.run</span>
        </span>
        <span class="brand-sub">控制台</span>
      </a>

      <nav class="rail-nav">
        <a class={`nav-item ${route.name === "sites" ? "active" : ""}`} href={href({ name: "sites" })}>
          作品
          {sites.length > 0 ? <span class="nav-count">{sites.length}</span> : null}
        </a>
        {sites.length > 0 ? (
          <ul class="rail-sites">
            {sites.map((site) => (
              <li key={site.slug}>
                <a
                  class={`rail-site ${site.slug === currentSlug ? "active" : ""}`}
                  href={href({ name: "site", slug: site.slug, tab: "results" })}
                  title={site.slug}
                >
                  <span class={`dot ${dotOf(site)}`} />
                  <span class="rail-site-title">{site.title}</span>
                </a>
              </li>
            ))}
          </ul>
        ) : null}
      </nav>

      <div class="rail-bottom">
        {hasToken ? <PublishNote /> : null}
        <a class="nav-item" href={plazaUrl} target="_blank" rel="noreferrer">
          广场
          <span class="nav-arrow">↗</span>
        </a>
        <a class={`nav-item account ${route.name === "token" ? "active" : ""}`} href={href({ name: "token" })}>
          {me?.avatar_url ? <img class="avatar" src={me.avatar_url} alt="" /> : <span class="avatar blank" />}
          <span class="account-name">{who ?? (hasToken ? "匿名 · 24 小时" : "登录")}</span>
        </a>
      </div>
    </aside>
  );
}

/** 栏上每个作品前面那个点：在广场上是绿的，匿名快到期是橙的，其余是灰的。只说事实，不做按钮。 */
function dotOf(site: Site): string {
  if (site.listing?.public && !site.listing.hidden) return "on";
  if (site.expires_at) return "soon";
  return "";
}

/**
 * 「发下一版」的便条。发布在终端里，不在网页上——这里只把那一行命令递到手边，
 * 和广场栏底那张便条是同一扇门（DESIGN §3.9）。
 */
function PublishNote() {
  return (
    <div class="publish-note">
      <h3>发下一版</h3>
      <p>
        在作品目录里再跑一次，链接不变，关注的人会收到通知。
      </p>
      <code>playtest ./dist --note "这版改了什么"</code>
    </div>
  );
}

/** 玩家链接去掉 slug 那一级就是广场：`https://brisk-otter-41.playtest.run` → `https://playtest.run/`。 */
function plazaOf(sites: Site[] | undefined): string {
  const sample = sites?.[0]?.url;
  if (!sample) return "https://playtest.run/";
  try {
    const url = new URL(sample);
    const host = url.host.split(".").slice(1).join(".");
    return host ? `${url.protocol}//${host}/` : "https://playtest.run/";
  } catch {
    return "https://playtest.run/";
  }
}

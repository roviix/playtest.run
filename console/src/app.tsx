// 控制台的外壳：左边一条栏，右边一张台面（DESIGN §3.13）。
//
// 和广场（edge/src/plaza.rs）是同一套记号：同一个标记、同样的夜色、同一支琥珀。
// 栏比广场的宽：它要放作品清单，广场的栏只放两间房。
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
import { Publish } from "./publish";
import { CollectionsPage } from "./pages/collections";
import { DocsPage } from "./pages/docs";

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
      <a class="skip-link" href="#main-content" onClick={(event) => { event.preventDefault(); document.getElementById("main-content")?.focus(); }}>跳到内容</a>
      <main class="stage" id="main-content" tabIndex={-1}>
        {callback === "working" ? (
          <p class="muted stage-note">正在登录…</p>
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
  if (route.name === "docs") return <DocsPage section={route.section} />;
  if (route.name === "token" || !hasToken) return <TokenPage loginError={loginError} me={me} />;

  switch (route.name) {
    case "collections":
      return <CollectionsPage key={route.slug ?? "index"} slug={route.slug} sites={sites.data ?? []} me={me} plazaUrl={plazaUrl} />;
    case "sites":
      return <HomePage sites={sites} me={me} />;
    case "site":
      return (
        <SitePage
          key={route.slug}
          slug={route.slug}
          initialSite={sites.data?.find((site) => site.slug === route.slug)}
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
    <aside class="rail sidebar">
      <a class="brand" href={href({ name: "sites" })}>
        <span class="mark" aria-hidden="true">
          <svg class="mark-svg" viewBox="0 0 24 24">
            <path d="M4 8V5a1 1 0 0 1 1-1h3M16 4h3a1 1 0 0 1 1 1v3M4 16v3a1 1 0 0 0 1 1h3M16 20h3a1 1 0 0 0 1-1v-3" />
            <circle class="dot" cx="12" cy="12" r="2.2" />
          </svg>
        </span>
        <span class="brand-text">
          <span class="wordmark">
            playtest<span class="tld">.run</span>
          </span>
          <span class="brand-sub">控制台</span>
        </span>
      </a>

      <nav class="rail-nav">
        <a class={`nav-item ${route.name === "collections" ? "active" : ""}`} href={href({ name: "collections" })} aria-current={route.name === "collections" ? "page" : undefined}>
          <svg class="icon" viewBox="0 0 24 24" aria-hidden="true"><path d="M4 7V5a1 1 0 0 1 1-1h5l2 3h7a1 1 0 0 1 1 1v11a1 1 0 0 1-1 1H5a1 1 0 0 1-1-1V7Z"/></svg>合集与挑战
        </a>
        <a class={`nav-item docs-nav-link ${route.name === "docs" ? "active" : ""}`} href={href({ name: "docs", section: "start" })} aria-current={route.name === "docs" ? "page" : undefined}>
          <svg class="icon" viewBox="0 0 24 24" aria-hidden="true"><path d="M5 4h11a3 3 0 0 1 3 3v13H8a3 3 0 0 1-3-3V4Z"/><path d="M5 16h14M9 8h6M9 11h4"/></svg>使用文档
        </a>
        <a class={`nav-item ${route.name === "sites" ? "active" : ""}`} href={href({ name: "sites" })} aria-current={route.name === "sites" ? "page" : undefined}>
          <svg class="icon" viewBox="0 0 24 24" aria-hidden="true"><rect x="4" y="4" width="6" height="6" rx="1.5"/><rect x="14" y="4" width="6" height="6" rx="1.5"/><rect x="4" y="14" width="6" height="6" rx="1.5"/><rect x="14" y="14" width="6" height="6" rx="1.5"/></svg>我的作品
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
        <Publish />
        <a class="nav-item" href={plazaUrl} target="_blank" rel="noreferrer">
          <svg class="icon" viewBox="0 0 24 24" aria-hidden="true"><circle cx="12" cy="12" r="8"/><path d="m15 9-2 4-4 2 2-4Z"/></svg>探索广场
          <span class="nav-arrow">↗</span>
        </a>
        <a class={`nav-item account ${route.name === "token" ? "active" : ""}`} href={href({ name: "token" })} aria-label={hasToken ? "账号与访问令牌" : "登录控制台"}>
          {me?.avatar_url ? <img class="avatar" src={me.avatar_url} alt="" /> : <span class="avatar blank" />}
          <span class="account-name">{who ?? (hasToken ? "匿名" : "登录")}</span>
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

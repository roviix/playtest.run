// 控制台外壳统一处理导航、身份与登录，页面只负责自己的内容（DESIGN §3.13）。

import mark from "../../ui/mark.svg?raw";
import wordmark from "../../ui/wordmark.svg?raw";

import { useEffect, useState } from "preact/hooks";

import { api, AUTH_EXPIRED_EVENT, AUTH_REQUEST_EVENT, exchangeGitHubCode, forgetToken, readToken, saveToken, type Me, type Site } from "./api";
import { useLoad } from "./load";
import { go, href, useRoute, type Route } from "./router";
import { HomePage } from "./pages/home";
import { SitePage } from "./pages/site";
import { TokenPage } from "./pages/token";
import { Publish } from "./publish";
import { CollectionsPage } from "./pages/collections";
import { DocsPage } from "./pages/docs";
import { LoginDialog, LoginRequired, takeLoginReturn, type AuthRequest } from "./auth";

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
  const [meError, setMeError] = useState("");
  const [meAttempt, setMeAttempt] = useState(0);
  const [exchanging, setExchanging] = useState(() => !!callbackParams());
  const [login, setLogin] = useState<AuthRequest | null>(null);

  function authenticated(token: string, identity: Me | null, target: Route) {
    saveToken(token);
    setToken(token);
    setMe(identity);
    setMeError("");
    setLogin(null);
    setExchanging(false);
    go(target);
  }

  function logout() {
    forgetToken();
    setToken("");
    setMe(null);
  }

  useEffect(() => {
    const params = callbackParams();
    if (!params) return;
    const target = takeLoginReturn();
    // 授权码只使用一次，也不让刷新重复交换。
    history.replaceState(null, "", `${location.pathname}${location.hash}`);
    exchangeGitHubCode(params.code, params.state)
      .then((result) => authenticated(result.token, null, target))
      .catch((err: Error) => {
        setExchanging(false);
        setLogin({ target, error: `登录失败：${err.message}` });
      });
  }, []);

  useEffect(() => {
    if (!token && route.name !== "docs" && !exchanging) setLogin({ target: route });
  }, [route]);

  useEffect(() => {
    const expired = () => {
      logout();
      setLogin({ target: route, error: "登录已失效，请重新登录。" });
    };
    const requested = () => setLogin({ target: route });
    addEventListener(AUTH_EXPIRED_EVENT, expired);
    addEventListener(AUTH_REQUEST_EVENT, requested);
    return () => {
      removeEventListener(AUTH_EXPIRED_EVENT, expired);
      removeEventListener(AUTH_REQUEST_EVENT, requested);
    };
  }, [route]);

  useEffect(() => {
    let alive = true;
    setMeError("");
    if (!token) { setMe(null); return; }
    api.me().then((identity) => { if (alive) setMe(identity); }).catch((err: Error) => {
      if (alive) { setMe(null); setMeError(err.message); }
    });
    return () => { alive = false; };
  }, [token, meAttempt]);

  const hasToken = token !== "";
  const sites = useLoad(() => (hasToken ? api.sites() : Promise.resolve([] as Site[])), [token]);
  const plazaUrl = plazaOf(sites.data);

  function navigate(event: MouseEvent, target: Route) {
    if (hasToken || target.name === "docs" || event.button !== 0 || event.metaKey || event.ctrlKey || event.shiftKey || event.altKey) return;
    event.preventDefault();
    setLogin({ target });
  }

  let content;
  if (exchanging) content = <p class="muted stage-note" role="status">正在登录…</p>;
  else if (route.name === "docs") content = <DocsPage section={route.section} />;
  else if (!hasToken) content = <LoginRequired route={route} onLogin={() => setLogin({ target: route })} />;
  else if (route.name === "token") content = <TokenPage me={me} error={meError} onRetry={() => setMeAttempt((n) => n + 1)} onLogin={() => setLogin({ target: route })} onLogout={logout} />;
  else if (route.name === "collections") content = <CollectionsPage key={route.slug ?? "index"} slug={route.slug} sites={sites.data ?? []} me={me} plazaUrl={plazaUrl} />;
  else if (route.name === "sites") content = <HomePage sites={sites} me={me} />;
  else content = <SitePage key={route.slug} slug={route.slug} initialSite={sites.data?.find((site) => site.slug === route.slug)} tab={route.tab} version={route.version} plazaUrl={plazaUrl} onSiteChanged={sites.reload} />;

  return <div class="shell">
    <Rail route={route} me={me} hasToken={hasToken} sites={hasToken ? sites.data ?? [] : []} plazaUrl={plazaUrl} onNavigate={navigate} />
    <a class="skip-link" href="#main-content" onClick={(event) => { event.preventDefault(); document.getElementById("main-content")?.focus(); }}>跳到内容</a>
    <main class="stage" id="main-content" tabIndex={-1} key={token}>{content}</main>
    {login ? <LoginDialog request={login} onClose={() => setLogin(null)} onAuthenticated={authenticated} /> : null}
  </div>;
}

// ------------------------------------------------------------------ 栏

function Rail({
  route,
  me,
  hasToken,
  sites,
  plazaUrl,
  onNavigate,
}: {
  route: Route;
  me: Me | null;
  hasToken: boolean;
  sites: Site[];
  plazaUrl: string;
  onNavigate: (event: MouseEvent, route: Route) => void;
}) {
  const who = me?.kind === "github" ? `@${me.login ?? me.display_name}` : null;
  const currentSlug = route.name === "site" ? route.slug : null;

  return (
    <aside class="rail sidebar">
      <a class="brand" href={href({ name: "sites" })} onClick={(event) => onNavigate(event, { name: "sites" })} aria-label="playtest.run 控制台">
        <span class="mark" aria-hidden="true" dangerouslySetInnerHTML={{ __html: mark }} />
        <span class="brand-text">
          <span class="brand-wordmark" dangerouslySetInnerHTML={{ __html: wordmark }} />
        </span>
      </a>

      <nav class="rail-nav">
        <a class={`nav-item ${route.name === "collections" ? "active" : ""}`} href={href({ name: "collections" })} onClick={(event) => onNavigate(event, { name: "collections" })} aria-current={route.name === "collections" ? "page" : undefined}>
          <svg class="icon" viewBox="0 0 24 24" aria-hidden="true"><path d="M4 7V5a1 1 0 0 1 1-1h5l2 3h7a1 1 0 0 1 1 1v11a1 1 0 0 1-1 1H5a1 1 0 0 1-1-1V7Z"/></svg>合集与挑战
        </a>
        <a class={`nav-item docs-nav-link ${route.name === "docs" ? "active" : ""}`} href={href({ name: "docs", section: "start" })} aria-current={route.name === "docs" ? "page" : undefined}>
          <svg class="icon" viewBox="0 0 24 24" aria-hidden="true"><path d="M5 4h11a3 3 0 0 1 3 3v13H8a3 3 0 0 1-3-3V4Z"/><path d="M5 16h14M9 8h6M9 11h4"/></svg>使用文档
        </a>
        <a class={`nav-item ${route.name === "sites" ? "active" : ""}`} href={href({ name: "sites" })} onClick={(event) => onNavigate(event, { name: "sites" })} aria-current={route.name === "sites" ? "page" : undefined}>
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
        <a class={`nav-item account ${route.name === "token" ? "active" : ""}`} href={href({ name: "token" })} onClick={(event) => onNavigate(event, { name: "token" })} aria-label={hasToken ? "账号与访问令牌" : "登录控制台"}>
          {me?.avatar_url ? <img class="avatar" src={me.avatar_url} alt="" /> : <span class="avatar blank" />}
          <span class="account-name">{who ?? (hasToken ? me?.kind === "anon" ? "匿名" : "账号" : "登录")}</span>
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

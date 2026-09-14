import mark from "../../ui/mark.svg?raw";
import wordmark from "../../ui/wordmark.svg?raw";
import { useEffect, useState } from "preact/hooks";
import { account, api, AUTH_EXPIRED_EVENT, AUTH_REQUEST_EVENT, exchangeGitHubCode, type AccountState, type Me, type Site } from "./api";
import { useLoad } from "./load";
import { href, useRoute, type Route } from "./router";
import { HomePage } from "./pages/home";
import { SitePage } from "./pages/site";
import { TokenPage } from "./pages/token";
import { Publish } from "./publish";
import { CollectionsPage } from "./pages/collections";
import { DocsPage } from "./pages/docs";
import { DevicePage } from "./pages/device";
import { BrandMark, LoginDialog, LoginRequired, type AuthRequest } from "./auth";

export function App() {
  const route = useRoute();
  const [identity, setIdentity] = useState<AccountState | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState("");
  const [login, setLogin] = useState<AuthRequest | null>(null);
  const [emailToken, setEmailToken] = useState(() => new URLSearchParams(location.search).get("email_token"));
  const [confirming, setConfirming] = useState(false);
  const [emailPreview, setEmailPreview] = useState<{ email: string; link: boolean } | null>(null);
  const [exchanging, setExchanging] = useState(() => new URLSearchParams(location.search).has("code"));
  const me = identity?.account?.me ?? null;

  async function refresh() {
    setError("");
    try { setIdentity(await account.view()); }
    catch (error) { setError(error instanceof Error ? error.message : "暂时无法连接，请重试。"); }
    finally { setLoading(false); }
  }

  useEffect(() => {
    const query = new URLSearchParams(location.search);
    const code = query.get("code");
    const state = query.get("state");
    const returnTo = query.get("return_to") ?? undefined;
    history.replaceState(null, "", `${location.pathname}${location.hash}`);
    if (code && state) {
      exchangeGitHubCode(code, state).then(async (result) => {
        await refresh();
        setExchanging(false);
        setLogin(null);
        const dest = result.return_to || "/console/";
        if (dest.startsWith("/console/#")) {
          location.hash = dest.slice("/console/".length);
        } else if (dest !== location.pathname && dest !== `${location.pathname}${location.hash}`) {
          location.replace(dest);
        }
      }).catch((error: Error) => {
        setExchanging(false);
        setLogin({ target: route, error: error.message });
        void refresh();
      });
    } else {
      setExchanging(false);
      void refresh();
      if (query.has("login") || query.has("error")) setLogin({ target: route, returnTo, error: query.has("error") ? "GitHub 登录未完成，可以重试或使用邮箱。" : undefined });
    }
  }, []);

  useEffect(() => {
    if (emailToken) account.previewEmail(emailToken).then(setEmailPreview).catch((error: Error) => setError(error.message));
  }, [emailToken]);

  useEffect(() => {
    const expired = () => {
      setIdentity((previous) => previous ? { ...previous, account: null } : null);
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

  const sites = useLoad(() => me ? api.sites() : Promise.resolve([] as Site[]), [me]);

  async function confirmEmail() {
    if (!emailToken || confirming) return;
    setConfirming(true);
    setError("");
    try {
      const result = await account.confirmEmail(emailToken);
      setEmailToken(null);
      await refresh();
      setLogin(null);
      const dest = result.return_to || "/console/";
      if (dest.startsWith("/console/#")) {
        location.hash = dest.slice("/console/".length);
      } else if (dest !== location.pathname && dest !== `${location.pathname}${location.hash}`) {
        location.replace(dest);
      }
    } catch (error) {
      setError(error instanceof Error ? error.message : "Sign in incomplete, please try again.");
    } finally {
      setConfirming(false);
    }
  }

  async function logout() {
    setError("");
    try { await account.logout(); setIdentity((previous) => previous ? { ...previous, account: null } : null); }
    catch (error) { setError(error instanceof Error ? error.message : "Sign out failed, please try again."); }
  }

  function navigate(event: MouseEvent, target: Route) {
    if (me || target.name === "docs" || event.button !== 0 || event.metaKey || event.ctrlKey || event.shiftKey || event.altKey) return;
    event.preventDefault();
    setLogin({ target });
    if (!identity) void refresh();
  }

  let content;
  if (emailToken) content = <section class="auth-empty"><BrandMark /><h1>{emailPreview?.link ? "Link Email" : "Sign in to playtest"}</h1><p class="muted">{emailPreview ? `${emailPreview.email} · Confirm to return to your previous page.` : "Verifying sign-in link…"}</p><button class="button primary" disabled={confirming || !emailPreview} onClick={confirmEmail}>{confirming ? "Confirming…" : emailPreview?.link ? "Confirm Link" : "Confirm Sign in"}</button><button class="button quiet" onClick={() => { setEmailToken(null); setError(""); setLogin({ target: route }); }}>Resend sign-in link</button></section>;
  else if (exchanging || loading && route.name !== "docs") content = <p class="muted stage-note" role="status">Connecting…</p>;
  else if (route.name === "docs") content = <DocsPage section={route.section} />;
  else if (!me) content = <LoginRequired route={route} onLogin={() => { setLogin({ target: route }); if (!identity) void refresh(); }} />;
  else if (route.name === "token") content = <TokenPage identity={identity!} onRefresh={refresh} onLogout={logout} />;
  else if (route.name === "device") content = <DevicePage />;
  else if (route.name === "collections") content = <CollectionsPage key={route.slug ?? "index"} slug={route.slug} sites={sites.data ?? []} me={me} plazaUrl="/" />;
  else if (route.name === "sites") content = <HomePage sites={sites} me={me} />;
  else content = <SitePage key={route.slug} slug={route.slug} initialSite={sites.data?.find((site) => site.slug === route.slug)} tab={route.tab} version={route.version} plazaUrl="/" onSiteChanged={sites.reload} />;

  return <div class="shell">
    <Rail route={route} me={me} onNavigate={navigate} />
    <a class="skip-link" href="#main-content" onClick={(event) => { event.preventDefault(); document.getElementById("main-content")?.focus(); }}>Skip to content</a>
    <main class="stage" id="main-content" tabIndex={-1}>
      {error ? <p class="notice" role="alert">{error} {!emailToken ? <button class="button quiet" onClick={refresh}>Retry</button> : null}</p> : null}
      {content}
    </main>
    {login ? <LoginDialog request={login} available={identity} onClose={() => setLogin(null)} /> : null}
  </div>;
}

function Rail({ route, me, onNavigate }: { route: Route; me: Me | null; onNavigate: (event: MouseEvent, target: Route) => void }) {
  const mine = route.name === "sites" || route.name === "site";
  return <aside class="sidebar">
    <a class="brand" href="/" aria-label="playtest home"><span class="mark" aria-hidden="true" dangerouslySetInnerHTML={{ __html: mark }} /><span dangerouslySetInnerHTML={{ __html: wordmark }} /></a>
    <div class="sidebar-action">
      <Publish />
    </div>
    <nav class="rail-nav" aria-label="Pages">
      <a class="nav-item" href="/"><NavIcon kind="grid" />Plaza</a>
      <a class="nav-item" href="/me"><NavIcon kind="bell" />Following</a>
      <a class={`nav-item ${mine ? "active" : ""}`} aria-current={mine ? "page" : undefined} href={href({ name: "sites" })} onClick={(event) => onNavigate(event, { name: "sites" })}><NavIcon kind="grid" />My Projects</a>
      <a class={`nav-item ${route.name === "collections" ? "active" : ""}`} aria-current={route.name === "collections" ? "page" : undefined} href={href({ name: "collections" })} onClick={(event) => onNavigate(event, { name: "collections" })}><NavIcon kind="folder" />My Collections</a>
    </nav>
    <div class="rail-bottom">
      <a class={`nav-item ${route.name === "docs" ? "active" : ""}`} href={href({ name: "docs", section: "start" })} aria-current={route.name === "docs" ? "page" : undefined}><NavIcon kind="book" />Documentation</a>
      <a class={`nav-item account ${route.name === "token" ? "active" : ""}`} href={href({ name: "token" })} onClick={(event) => onNavigate(event, { name: "token" })}>
        {me?.avatar_url ? <img class="avatar" src={me.avatar_url} alt="" /> : <NavIcon kind="person" />}
        <span class="account-name">{me?.display_name ?? "Sign in"}</span>
      </a>
    </div>
  </aside>;
}

function NavIcon({ kind }: { kind: string }) {
  return <svg class="icon" viewBox="0 0 24 24" aria-hidden="true">{kind === "folder" ? <path d="M3 7V5a1 1 0 0 1 1-1h5l2 3h9v13H3Z" /> : kind === "bell" ? <path d="M18 8a6 6 0 0 0-12 0c0 7-3 7-3 9h18c0-2-3-2-3-9M10 21h4" /> : kind === "book" ? <path d="M5 4h14v17H8a3 3 0 0 1-3-3V4Zm0 13h14M9 8h6M9 11h4" /> : kind === "person" ? <><circle cx="12" cy="8" r="4" /><path d="M4 21v-2a8 8 0 0 1 16 0v2" /></> : <><rect x="4" y="4" width="6" height="6" rx="1.5" /><rect x="14" y="4" width="6" height="6" rx="1.5" /><rect x="4" y="14" width="6" height="6" rx="1.5" /><rect x="14" y="14" width="6" height="6" rx="1.5" /></>}</svg>;
}

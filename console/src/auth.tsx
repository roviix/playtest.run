// 登录是中途的一步；弹窗与回程由 App 统一管理。
import { useEffect, useRef, useState } from "preact/hooks";
import mark from "../../ui/mark.svg?raw";
import { ApiError, githubLoginUrl, validateToken, type Me } from "./api";
import { href, parse, type Route } from "./router";

const RETURN_KEY = "playtest.login-return";

export function rememberLoginReturn(route: Route) {
  try { sessionStorage.setItem(RETURN_KEY, href(route)); } catch { /* 禁用存储时仍可去 GitHub。 */ }
}

export function takeLoginReturn(): Route {
  let value = "#/";
  try {
    value = sessionStorage.getItem(RETURN_KEY) ?? value;
    sessionStorage.removeItem(RETURN_KEY);
  } catch { /* 无回程记录时回作品页。 */ }
  return parse(value.startsWith("#/") ? value : "#/");
}

export function BrandMark() {
  return <span class="dialog-mark" aria-hidden="true" dangerouslySetInnerHTML={{ __html: mark }} />;
}

export type AuthRequest = { target: Route; error?: string };

export function LoginDialog({ request, onClose, onAuthenticated }: {
  request: AuthRequest;
  onClose: () => void;
  onAuthenticated: (token: string, me: Me, target: Route) => void;
}) {
  const dialog = useRef<HTMLDialogElement>(null);
  const input = useRef<HTMLInputElement>(null);
  const generation = useRef(0);
  const [tokenMode, setTokenMode] = useState(false);
  const [value, setValue] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState(request.error ?? "");

  useEffect(() => {
    const trigger = document.activeElement instanceof HTMLElement ? document.activeElement : null;
    dialog.current?.showModal();
    return () => {
      generation.current++;
      dialog.current?.close();
      if (trigger?.isConnected) trigger.focus({ preventScroll: true });
    };
  }, []);
  useEffect(() => { if (tokenMode && !busy) input.current?.focus(); }, [tokenMode, busy]);
  useEffect(() => { setError(request.error ?? ""); }, [request.error]);

  async function submit(event: Event) {
    event.preventDefault();
    const token = value.trim();
    if (!token || busy) return;
    const current = ++generation.current;
    setBusy(true);
    setError("");
    try {
      const me = await validateToken(token);
      if (current === generation.current) onAuthenticated(token, me, request.target);
    } catch (err) {
      if (current !== generation.current) return;
      setError(err instanceof ApiError && err.needsToken ? "令牌无效或已失效，请检查后重试。" : err instanceof Error ? err.message : "暂时无法登录，请重试。");
    } finally {
      if (current === generation.current) setBusy(false);
    }
  }

  return <dialog ref={dialog} class="account-dialog login-dialog" aria-labelledby="login-title" onClose={onClose} onCancel={() => { generation.current++; }} onClick={(event) => {
    const element = dialog.current;
    if (!element || event.target !== element) return;
    const rect = element.getBoundingClientRect();
    if (event.clientX < rect.left || event.clientX > rect.right || event.clientY < rect.top || event.clientY > rect.bottom) element.close();
  }}>
    <div class="notice-head">
      <div class="dialog-title-wrap"><BrandMark /><h2 id="login-title">登录</h2></div>
      <button class="dialog-close-btn" type="button" aria-label="关闭登录" onClick={() => dialog.current?.close()}>×</button>
    </div>
    <a class="button primary github-login" href={githubLoginUrl} autoFocus onClick={() => rememberLoginReturn(request.target)}>
      <svg class="icon" viewBox="0 0 24 24" aria-hidden="true"><path d="M9 19c-4 1-4-2-6-2m12 5v-3.4a3 3 0 0 0-.8-2.3c2.7-.3 5.6-1.3 5.6-6a4.7 4.7 0 0 0-1.3-3.3 4.3 4.3 0 0 0-.1-3.3S17.4 3.4 15 5a11.5 11.5 0 0 0-6 0C6.6 3.4 5.6 3.7 5.6 3.7a4.3 4.3 0 0 0-.1 3.3 4.7 4.7 0 0 0-1.3 3.3c0 4.7 2.9 5.7 5.6 6A3 3 0 0 0 9 18.6V22"/></svg>
      使用 GitHub 登录
    </a>
    <div class="login-token">
      <button class="login-alternative" type="button" aria-expanded={tokenMode} aria-controls="token-login-form" onClick={() => setTokenMode(!tokenMode)}>
        <span>使用访问令牌</span><svg class="icon" viewBox="0 0 24 24" aria-hidden="true"><path d="m9 5 7 7-7 7"/></svg>
      </button>
      {tokenMode ? <form id="token-login-form" class="notice-form" onSubmit={submit} aria-busy={busy}>
        <div class="field-label-row">
          <label for="token-input">访问令牌</label>
          <a class="token-help-tip" href={href({ name: "docs", section: "start" })} onClick={onClose} title="如何获取访问令牌？" aria-label="如何获取访问令牌？">
            <svg class="icon help-icon" viewBox="0 0 24 24" aria-hidden="true"><circle cx="12" cy="12" r="10"/><path d="M9.09 9a3 3 0 0 1 5.83 1c0 2-3 3-3 3m.08 4h.01"/></svg>
            <span>怎么获取？</span>
          </a>
        </div>
        <input id="token-input" ref={input} type="password" name="token" autoComplete="off" spellcheck={false} value={value} placeholder="粘贴访问令牌" required disabled={busy} aria-invalid={error ? true : undefined} aria-describedby={error ? "login-error" : undefined} onInput={(event) => { setValue(event.currentTarget.value); setError(""); }} />
        <button type="submit" class="button-submit" disabled={!value.trim() || busy}>{busy ? "正在验证…" : "登录"}</button>
      </form> : null}
    </div>
    {error ? <p class="login-error" id="login-error" role="alert">{error}</p> : null}
  </dialog>;
}

export function LoginRequired({ route, onLogin }: { route: Route; onLogin: () => void }) {
  const title = route.name === "collections" ? "合集与挑战" : route.name === "token" ? "账号" : route.name === "site" ? "作品" : "我的作品";
  return <div class="auth-required">
    <header class="stage-head"><h1>{title}</h1></header>
    <div class="auth-empty">
      <span class="auth-empty-icon" aria-hidden="true"><svg class="icon" viewBox="0 0 24 24"><rect x="5" y="10" width="14" height="11" rx="2"/><path d="M8 10V7a4 4 0 0 1 8 0v3M12 14v3"/></svg></span>
      <h2>登录后查看{title === "账号" ? "账号" : title === "合集与挑战" ? "合集" : "作品"}</h2>
      <button class="button primary" type="button" onClick={onLogin}>登录</button>
    </div>
  </div>;
}

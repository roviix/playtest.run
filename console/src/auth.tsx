import { useEffect, useRef, useState } from "preact/hooks";
import mark from "../../ui/mark.svg?raw";
import { account, githubLoginUrl, type AccountState } from "./api";
import { href, type Route } from "./router";

export function BrandMark() {
  return <span class="dialog-mark" aria-hidden="true" dangerouslySetInnerHTML={{ __html: mark }} />;
}

export type AuthRequest = { target: Route; error?: string; returnTo?: string };

export function LoginDialog({ request, onClose, available }: {
  request: AuthRequest;
  onClose: () => void;
  available: AccountState | null;
}) {
  const dialog = useRef<HTMLDialogElement>(null);
  const generation = useRef(0);
  const [email, setEmail] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState(request.error ?? "");
  const [sent, setSent] = useState(false);
  const returnTo = request.returnTo ?? `/console/${href(request.target)}`;

  useEffect(() => {
    const trigger = document.activeElement instanceof HTMLElement ? document.activeElement : null;
    dialog.current?.showModal();
    return () => {
      generation.current++;
      dialog.current?.close();
      if (trigger?.isConnected) trigger.focus({ preventScroll: true });
    };
  }, []);

  async function submit(event: Event) {
    event.preventDefault();
    if (!email.trim() || busy) return;
    const current = ++generation.current;
    setBusy(true);
    setError("");
    try {
      await account.email(email.trim(), returnTo);
      if (current === generation.current) setSent(true);
    } catch (error) {
      if (current === generation.current) setError(error instanceof Error ? error.message : "未能发送，请稍后重试。");
    } finally {
      if (current === generation.current) setBusy(false);
    }
  }

  return <dialog ref={dialog} class="account-dialog login-dialog" aria-labelledby="login-title" onClose={onClose} onClick={(event) => {
    const element = dialog.current;
    if (element && event.target === element) {
      const rect = element.getBoundingClientRect();
      if (event.clientX < rect.left || event.clientX > rect.right || event.clientY < rect.top || event.clientY > rect.bottom) element.close();
    }
  }}>
    <div class="notice-head">
      <div class="dialog-title-wrap"><BrandMark /><h2 id="login-title">{sent ? "去邮箱继续" : "登录 playtest"}</h2></div>
      <button class="dialog-close-btn" type="button" aria-label="关闭登录" onClick={() => dialog.current?.close()}>×</button>
    </div>
    {sent ? <div class="login-sent" role="status">
      <p>登录链接已加入发送队列，请查收 <strong>{email}</strong>。</p>
      <p class="muted">链接一小时内有效。没有收到？可以检查垃圾邮件，或稍后重试。</p>
      <button class="button quiet" type="button" onClick={() => { setSent(false); setError(""); }}>更换邮箱或重发</button>
    </div> : <>
      {available?.email_available ? <form class="notice-form" onSubmit={submit} aria-busy={busy}>
        <label for="login-email" class="sr-only">邮箱</label>
        <input id="login-email" type="email" autoComplete="email" inputMode="email" placeholder="你的邮箱" value={email} required autoFocus disabled={busy} onInput={(event) => { setEmail(event.currentTarget.value); setError(""); }} />
        <button class="button primary" type="submit" disabled={busy || !email.trim()}>{busy ? "正在发送…" : "用邮箱继续"}</button>
      </form> : null}
      {available?.github_available ? <a class="button github-login" href={`${githubLoginUrl}?return_to=${encodeURIComponent(returnTo)}`}>
        <svg class="icon" viewBox="0 0 24 24" aria-hidden="true"><path d="M9 19c-4 1-4-2-6-2m12 5v-3.4a3 3 0 0 0-.8-2.3c2.7-.3 5.6-1.3 5.6-6a4.7 4.7 0 0 0-1.3-3.3 4.3 4.3 0 0 0-.1-3.3S17.4 3.4 15 5a11.5 11.5 0 0 0-6 0C6.6 3.4 5.6 3.7 5.6 3.7a4.3 4.3 0 0 0-.1 3.3 4.7 4.7 0 0 0-1.3 3.3c0 4.7 2.9 5.7 5.6 6A3 3 0 0 0 9 18.6V22"/></svg>
        使用 GitHub 继续
      </a> : null}
      {available && !available.email_available && !available.github_available ? <p role="status">登录暂不可用，作品仍可直接体验。请稍后再来。</p> : null}
      {!available ? <p role="status">暂时无法获取登录方式，请关闭后重试。</p> : null}
      <p class="login-caption">一个账号，关注与创作。不会自动订阅任何内容。</p>
    </>}
    {error ? <p class="login-error" role="alert">{error}</p> : null}
  </dialog>;
}

export function LoginRequired({ route, onLogin }: { route: Route; onLogin: () => void }) {
  const title = route.name === "collections" ? "我的合集" : route.name === "token" ? "账号" : "我的作品";
  return <div class="auth-required">
    <header class="stage-head"><h1>{title}</h1></header>
    <div class="auth-empty">
      <BrandMark />
      <h2>登录后，接着创作</h2>
      <p class="muted">关注、作品与合集，都在同一个账号里。</p>
      <button class="button primary" type="button" onClick={onLogin}>登录</button>
    </div>
  </div>;
}

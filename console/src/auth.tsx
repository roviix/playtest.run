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
      if (current === generation.current) setError(error instanceof Error ? error.message : "Failed to send. Please try again later.");
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
      <div class="dialog-title-wrap"><BrandMark /><h2 id="login-title">{sent ? "Check your email" : "Sign in to playtest"}</h2></div>
      <button class="dialog-close-btn" type="button" aria-label="Close sign in dialog" onClick={() => dialog.current?.close()}>×</button>
    </div>
    {sent ? <div class="login-sent" role="status">
      <p>Sign-in link sent. Please check <strong>{email}</strong>.</p>
      <p class="muted">Valid for 1 hour. Didn't receive it? Check your spam folder, or try again shortly.</p>
      <button class="button quiet" type="button" onClick={() => { setSent(false); setError(""); }}>Use another email or resend</button>
    </div> : <>
      {available?.email_available ? <form class="notice-form" onSubmit={submit} aria-busy={busy}>
        <label for="login-email" class="sr-only">Email</label>
        <input id="login-email" type="email" autoComplete="email" inputMode="email" placeholder="your@email.com" value={email} required autoFocus disabled={busy} onInput={(event) => { setEmail(event.currentTarget.value); setError(""); }} />
        <button class="button primary" type="submit" disabled={busy || !email.trim()}>{busy ? "Sending…" : "Continue with Email"}</button>
      </form> : null}
      {available?.github_available ? <a class="button github-login" href={`${githubLoginUrl}?return_to=${encodeURIComponent(returnTo)}`}>
        <svg class="icon" viewBox="0 0 24 24" aria-hidden="true"><path d="M9 19c-4 1-4-2-6-2m12 5v-3.4a3 3 0 0 0-.8-2.3c2.7-.3 5.6-1.3 5.6-6a4.7 4.7 0 0 0-1.3-3.3 4.3 4.3 0 0 0-.1-3.3S17.4 3.4 15 5a11.5 11.5 0 0 0-6 0C6.6 3.4 5.6 3.7 5.6 3.7a4.3 4.3 0 0 0-.1 3.3 4.7 4.7 0 0 0-1.3 3.3c0 4.7 2.9 5.7 5.6 6A3 3 0 0 0 9 18.6V22"/></svg>
        Continue with GitHub
      </a> : null}
      {available && !available.email_available && !available.github_available ? <p role="status">Sign in is temporarily unavailable. Projects remain accessible. Please try again later.</p> : null}
      {!available ? <p role="status">Unable to load sign in methods right now. Please close and try again.</p> : null}
    </>}
    {error ? <p class="login-error" role="alert">{error}</p> : null}
  </dialog>;
}

export function LoginRequired({ route, onLogin }: { route: Route; onLogin: () => void }) {
  const title = route.name === "collections" ? "My Collections" : route.name === "token" ? "Account" : "My Projects";
  return <div class="auth-required">
    <header class="stage-head"><h1>{title}</h1></header>
    <div class="auth-empty">
      <BrandMark />
      <h2>Sign in to continue</h2>
      <p class="muted">Following, projects, and collections — all in one account.</p>
      <button class="button primary" type="button" onClick={onLogin}>Sign in</button>
    </div>
  </div>;
}

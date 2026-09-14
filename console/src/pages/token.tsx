import { useEffect, useState } from "preact/hooks";
import { account, githubLoginUrl, type AccountState, type AccessToken } from "../api";

export function TokenPage({ identity, onRefresh, onLogout }: { identity: AccountState; onRefresh: () => Promise<void>; onLogout: () => Promise<void> }) {
  const profile = identity.account!;
  const [email, setEmail] = useState("");
  const [busy, setBusy] = useState(false);
  const [message, setMessage] = useState("");
  const [error, setError] = useState("");
  const [token, setToken] = useState("");
  const [copied, setCopied] = useState(false);
  const [name, setName] = useState(profile.me.display_name);
  const [tokens, setTokens] = useState<AccessToken[]>([]);
  const loadTokens = () => account.tokens().then(setTokens).catch((error: Error) => setError(error.message));
  useEffect(() => { void loadTokens(); }, []);

  async function linkEmail(event: Event) {
    event.preventDefault();
    setBusy(true); setError(""); setMessage("");
    try { const result = await account.email(email, "/console/#/token", true); setMessage(result.message); }
    catch (error) { setError(error instanceof Error ? error.message : "Failed to link email, please try again."); }
    finally { setBusy(false); }
  }

  async function createToken() {
    setBusy(true); setError("");
    try { const result = await account.createToken(); setToken(result.token); setCopied(false); await loadTokens(); }
    catch (error) { setError(error instanceof Error ? error.message : "Failed to create token, please try again."); }
    finally { setBusy(false); }
  }

  return <div class="account-page">
    <header class="stage-head"><h1>Account</h1></header>
    <section class="account-profile">
      {profile.me.avatar_url ? <img class="avatar" src={profile.me.avatar_url} alt="" /> : null}
      <div><h2>{profile.me.display_name}</h2><p>Following and creating, all in one account.</p></div>
    </section>
    <form class="notice-form profile-name" onSubmit={async (event) => { event.preventDefault(); setBusy(true); setError(""); try { await account.profile(name); await onRefresh(); setMessage("Name saved."); } catch (error) { setError(error instanceof Error ? error.message : "Failed to save."); } finally { setBusy(false); } }}><label for="display-name">Display Name</label><input id="display-name" value={name} maxLength={40} required onInput={(event) => setName(event.currentTarget.value)} /><button class="button" disabled={busy || name.trim() === profile.me.display_name}>Save Name</button></form>
    <section class="account-methods" aria-labelledby="methods-title">
      <h2 id="methods-title">Sign-in Methods</h2>
      <div class="account-method"><span>Email</span>{profile.email ? <span>{profile.email}</span> : identity.email_available ? <form class="notice-form" onSubmit={linkEmail}><label class="sr-only" for="link-email">Email to link</label><input id="link-email" type="email" autoComplete="email" placeholder="Link email for updates" required value={email} onInput={(event) => setEmail(event.currentTarget.value)} /><button class="button" disabled={busy || !email.trim()}>Link Email</button></form> : <span class="muted">Unavailable</span>}</div>
      <div class="account-method"><span>GitHub</span>{profile.me.login ? <span>@{profile.me.login}</span> : identity.github_available ? <a class="button quiet" href={`${githubLoginUrl}?link=true&return_to=${encodeURIComponent("/console/#/token")}`}>Link GitHub</a> : <span class="muted">Unavailable</span>}</div>
      <p class="muted">Both sign-in methods will access this account. Works and follows from other accounts will not be merged.</p>
      {message ? <p role="status">{message} <button class="button quiet" onClick={onRefresh}>Verified, Refresh</button></p> : null}
    </section>
    <div class="account-actions"><a class="button quiet" href="/me">Notifications & Follows</a><button class="button quiet" type="button" onClick={onLogout}>Sign Out</button></div>
    <details class="account-security"><summary>Developer Settings</summary><p>Personal access tokens allow CLI and AI assistants to manage your works. It will only be shown once, keep it secret.</p>
      {token ? <div class="token-result"><label for="created-token">New Access Token</label><input id="created-token" type="password" readOnly value={token} autoComplete="off" /><button class="button" onClick={async () => { try { await navigator.clipboard.writeText(token); setCopied(true); } catch { setError("Failed to copy, please select and copy manually."); } }}>{copied ? "Copied" : "Copy Token"}</button></div> : <button class="button" disabled={busy} onClick={createToken}>{busy ? "Creating…" : "Create Access Token"}</button>}
      {tokens.length ? <ul class="token-list">{tokens.map(item => <li key={item.id}><span>Created {new Date(item.created_at).toLocaleString()}</span><button class="button quiet" disabled={busy} onClick={async () => { if (!confirm("Revoke this access token? CLI and assistants using it will lose access to manage works. Web sign-in is unaffected.")) return; setBusy(true); try { await account.revokeToken(item.id); setToken(""); await loadTokens(); } catch (error) { setError(error instanceof Error ? error.message : "Failed to revoke."); } finally { setBusy(false); } }}>Revoke</button></li>)}</ul> : null}
    </details>
    {error ? <p class="notice" role="alert">{error}</p> : null}
  </div>;
}

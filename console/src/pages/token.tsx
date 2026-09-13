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
    catch (error) { setError(error instanceof Error ? error.message : "关联未完成，请重试。"); }
    finally { setBusy(false); }
  }

  async function createToken() {
    setBusy(true); setError("");
    try { const result = await account.createToken(); setToken(result.token); setCopied(false); await loadTokens(); }
    catch (error) { setError(error instanceof Error ? error.message : "未能创建令牌，请重试。"); }
    finally { setBusy(false); }
  }

  return <div class="account-page">
    <header class="stage-head"><h1>账号</h1></header>
    <section class="account-profile">
      {profile.me.avatar_url ? <img class="avatar" src={profile.me.avatar_url} alt="" /> : null}
      <div><h2>{profile.me.display_name}</h2><p>关注与创作，都是你。</p></div>
    </section>
    <form class="notice-form profile-name" onSubmit={async (event) => { event.preventDefault(); setBusy(true); setError(""); try { await account.profile(name); await onRefresh(); setMessage("名字已保存。"); } catch (error) { setError(error instanceof Error ? error.message : "保存失败。"); } finally { setBusy(false); } }}><label for="display-name">作品上的名字</label><input id="display-name" value={name} maxLength={40} required onInput={(event) => setName(event.currentTarget.value)} /><button class="button" disabled={busy || name.trim() === profile.me.display_name}>保存名字</button></form>
    <section class="account-methods" aria-labelledby="methods-title">
      <h2 id="methods-title">登录方式</h2>
      <div class="account-method"><span>邮箱</span>{profile.email ? <span>{profile.email}</span> : identity.email_available ? <form class="notice-form" onSubmit={linkEmail}><label class="sr-only" for="link-email">要关联的邮箱</label><input id="link-email" type="email" autoComplete="email" placeholder="关联邮箱，接收更新" required value={email} onInput={(event) => setEmail(event.currentTarget.value)} /><button class="button" disabled={busy || !email.trim()}>关联邮箱</button></form> : <span class="muted">暂不可用</span>}</div>
      <div class="account-method"><span>GitHub</span>{profile.me.login ? <span>@{profile.me.login}</span> : identity.github_available ? <a class="button quiet" href={`${githubLoginUrl}?link=true&return_to=${encodeURIComponent("/console/#/token")}`}>关联 GitHub</a> : <span class="muted">暂不可用</span>}</div>
      <p class="muted">关联后，两种方式都登录这个账号。不会合并其他账号的作品或关注。</p>
      {message ? <p role="status">{message} <button class="button quiet" onClick={onRefresh}>已验证，刷新</button></p> : null}
    </section>
    <div class="account-actions"><a class="button quiet" href="/me">关注与通知设置</a><button class="button quiet" type="button" onClick={onLogout}>退出此设备</button></div>
    <details class="account-security"><summary>开发者设置</summary><p>访问令牌用于命令行与助手，有管理作品的权限。只在这里展示一次，不要发给别人。</p>
      {token ? <div class="token-result"><label for="created-token">新访问令牌</label><input id="created-token" type="password" readOnly value={token} autoComplete="off" /><button class="button" onClick={async () => { try { await navigator.clipboard.writeText(token); setCopied(true); } catch { setError("复制失败，请手动选择令牌复制。"); } }}>{copied ? "已复制" : "复制令牌"}</button></div> : <button class="button" disabled={busy} onClick={createToken}>{busy ? "正在创建…" : "创建访问令牌"}</button>}
      {tokens.length ? <ul class="token-list">{tokens.map(item => <li key={item.id}><span>{new Date(item.created_at).toLocaleString()} 创建</span><button class="button quiet" disabled={busy} onClick={async () => { if (!confirm("撤销这枚访问令牌？使用它的命令行和助手将无法继续管理作品，网页登录不受影响。")) return; setBusy(true); try { await account.revokeToken(item.id); setToken(""); await loadTokens(); } catch (error) { setError(error instanceof Error ? error.message : "撤销失败。"); } finally { setBusy(false); } }}>撤销</button></li>)}</ul> : null}
    </details>
    {error ? <p class="notice" role="alert">{error}</p> : null}
  </div>;
}

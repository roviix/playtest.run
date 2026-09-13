// 账号页只管理已连接的身份；登录与切换共用外壳弹窗。
import { useState } from "preact/hooks";
import { revokeToken, type Me } from "../api";
import { Loading } from "./status";

export function TokenPage({ me, error, onRetry, onLogin, onLogout }: {
  me: Me | null;
  error: string;
  onRetry: () => void;
  onLogin: () => void;
  onLogout: () => void;
}) {
  const [revoking, setRevoking] = useState(false);
  const [confirmRevoke, setConfirmRevoke] = useState(false);
  const [revokeError, setRevokeError] = useState("");

  async function revoke() {
    setRevoking(true);
    setRevokeError("");
    try { await revokeToken(); onLogout(); }
    catch (error) { setRevokeError(error instanceof Error ? error.message : "撤销失败，请重试。"); }
    finally { setRevoking(false); }
  }

  const github = me?.kind === "github";
  return <div class="account-page">
    <header class="stage-head"><h1>账号</h1></header>
    {!me ? error ? <p class="notice" role="alert">{error} <button class="button quiet" onClick={onRetry}>重试</button></p> : <Loading /> : <section class="account-profile">
      {me.avatar_url ? <img class="avatar" src={me.avatar_url} alt="" /> : null}
      <div><h2>{github ? `@${me.login ?? me.display_name}` : "匿名身份"}</h2>
        <p>{github ? "GitHub" : "登录 GitHub 后可接管匿名作品。"}</p>
      </div>
    </section>}
    <div class="account-actions">
      <button class="button" type="button" onClick={onLogin}>{github ? "切换账号" : "登录"}</button>
      <button class="button quiet" type="button" onClick={onLogout}>退出此设备</button>
    </div>
    <details class="account-security">
      <summary>访问令牌</summary>
      <p>撤销后，使用同一令牌的设备将无法继续管理作品。作品会保留，已连接的隧道不会立即断开。</p>
      {!github ? <p class="notice">匿名作品请先登录接管，否则撤销后会失去管理入口。</p> : null}
      {confirmRevoke ? <div class="row-actions">
        <button class="button" type="button" disabled={revoking} onClick={revoke}>{revoking ? "正在撤销…" : "确认撤销令牌"}</button>
        <button class="button quiet" type="button" disabled={revoking} onClick={() => setConfirmRevoke(false)}>取消</button>
      </div> : <button class="button quiet" type="button" onClick={() => setConfirmRevoke(true)}>撤销当前令牌…</button>}
      {revokeError ? <p class="notice" role="alert">{revokeError}</p> : null}
    </details>
  </div>;
}

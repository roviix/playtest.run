import { useEffect, useState } from "preact/hooks";
import { account, githubLoginUrl, type AccountState, type AccessToken } from "../api";

export function TokenPage({
  identity,
  onRefresh,
  onLogout,
}: {
  identity: AccountState;
  onRefresh: () => Promise<void>;
  onLogout: () => Promise<void>;
}) {
  const profile = identity.account!;
  const [email, setEmail] = useState("");
  const [showEmailInput, setShowEmailInput] = useState(!profile.email);
  const [busy, setBusy] = useState(false);
  const [message, setMessage] = useState("");
  const [error, setError] = useState("");
  const [token, setToken] = useState("");
  const [copied, setCopied] = useState(false);
  const [tokens, setTokens] = useState<AccessToken[]>([]);
  const [revokingId, setRevokingId] = useState<string | null>(null);

  // 行内编辑展示名
  const [editingName, setEditingName] = useState(false);
  const [draftName, setDraftName] = useState(profile.me.display_name);

  // 展开说明提示
  const [tipOpen, setTipOpen] = useState(false);

  const loadTokens = () =>
    account.tokens().then(setTokens).catch((err: Error) => setError(err.message));

  useEffect(() => {
    void loadTokens();
  }, []);

  async function linkEmail(event: Event) {
    event.preventDefault();
    if (!email.trim()) return;
    setBusy(true);
    setError("");
    setMessage("");
    try {
      const result = await account.email(email.trim(), "/console/#/token", true);
      setMessage(result.message);
      setEmail("");
      setShowEmailInput(false);
    } catch (err) {
      setError(err instanceof Error ? err.message : "绑定邮箱失败，请稍后重试。");
    } finally {
      setBusy(false);
    }
  }

  async function saveName() {
    const next = draftName.trim();
    if (!next || next === profile.me.display_name) {
      setEditingName(false);
      return;
    }
    setBusy(true);
    setError("");
    try {
      await account.profile(next);
      await onRefresh();
      setEditingName(false);
      setMessage("创作者名称已更新。");
      setTimeout(() => setMessage(""), 3000);
    } catch (err) {
      setError(err instanceof Error ? err.message : "保存名称失败，请稍后重试。");
    } finally {
      setBusy(false);
    }
  }

  async function createToken() {
    setBusy(true);
    setError("");
    try {
      const result = await account.createToken();
      setToken(result.token);
      setCopied(false);
      await loadTokens();
    } catch (err) {
      setError(err instanceof Error ? err.message : "生成令牌失败，请稍后重试。");
    } finally {
      setBusy(false);
    }
  }

  async function confirmRevoke(id: string) {
    setBusy(true);
    setError("");
    try {
      await account.revokeToken(id);
      if (token) setToken("");
      setRevokingId(null);
      await loadTokens();
      setMessage("令牌已撤销。");
      setTimeout(() => setMessage(""), 3000);
    } catch (err) {
      setError(err instanceof Error ? err.message : "撤销令牌失败。");
    } finally {
      setBusy(false);
    }
  }

  function copyToken() {
    if (!token) return;
    navigator.clipboard.writeText(token).then(() => {
      setCopied(true);
      setTimeout(() => setCopied(false), 2500);
    });
  }

  return (
    <div class="account-page">
      <header class="stage-head">
        <div>
          <h1>账号与设置</h1>
          <p class="muted">管理你的创作者公开资料、登录方式及自动化开发者令牌。</p>
        </div>
      </header>

      {error ? (
        <div class="account-notice error" role="alert">
          <svg class="icon" viewBox="0 0 24 24" aria-hidden="true" fill="none" stroke="currentColor" stroke-width="2">
            <circle cx="12" cy="12" r="10" />
            <line x1="12" y1="8" x2="12" y2="12" />
            <line x1="12" y1="16" x2="12.01" y2="16" />
          </svg>
          <span>{error}</span>
          <button type="button" class="notice-dismiss" onClick={() => setError("")} aria-label="关闭">×</button>
        </div>
      ) : null}

      {message ? (
        <div class="account-notice success" role="status">
          <svg class="icon" viewBox="0 0 24 24" aria-hidden="true" fill="none" stroke="currentColor" stroke-width="2">
            <path d="M22 11.08V12a10 10 0 1 1-5.93-9.14" />
            <polyline points="22 4 12 14.01 9 11.01" />
          </svg>
          <span>{message}</span>
          <button type="button" class="notice-dismiss" onClick={() => setMessage("")} aria-label="关闭">×</button>
        </div>
      ) : null}

      {/* 1. 创作者身份展台 (Identity Surface) */}
      <section class="settings-card">
        <div class="settings-identity">
          <div class="settings-identity-main">
            {profile.me.avatar_url ? (
              <img class="settings-identity-avatar" src={profile.me.avatar_url} alt="" />
            ) : (
              <div class="settings-identity-avatar placeholder">
                {(profile.me.display_name || "P").charAt(0).toUpperCase()}
              </div>
            )}
            <div class="settings-identity-copy">
              <div class="settings-identity-name-row">
                <strong>{profile.me.display_name}</strong>
                <span class="settings-chip settings-chip--pos">
                  <span class="settings-chip-dot" />
                  Creator
                </span>
              </div>
              <span>{profile.me.login ? `@${profile.me.login}` : (profile.email || "独立创作者")}</span>
            </div>
          </div>

          <div class="settings-identity-actions">
            <a class="settings-action" href="/me" title="查看公开作品、动态与关注列表">
              <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" aria-hidden="true">
                <path d="M18 13v6a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V8a2 2 0 0 1 2-2h6" />
                <polyline points="15 3 21 3 21 9" />
                <line x1="10" y1="14" x2="21" y2="3" />
              </svg>
              公开主页
            </a>
            <button class="settings-action settings-action--danger" type="button" onClick={onLogout} title="退出当前账号">
              <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" aria-hidden="true">
                <path d="M9 21H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h4" />
                <polyline points="16 17 21 12 16 7" />
                <line x1="21" y1="12" x2="9" y2="12" />
              </svg>
              退出
            </button>
          </div>
        </div>
      </section>

      {/* 2. 创作者资料 (Profile Card) */}
      <section class="settings-card">
        <header class="settings-card-head">
          <div class="settings-card-head-row">
            <div class="settings-card-heading">
              <h2 class="settings-card-title">创作者资料</h2>
            </div>
          </div>
          <p class="settings-card-desc">在作品门禁邀请函、广场署名与玩家反馈流中展示的公开身份。</p>
        </header>

        <div class="settings-card-body">
          {/* 行内就地编辑展示名称 */}
          <div class="settings-row">
            <div class="settings-row-meta">
              <span class="settings-row-label">展示名称</span>
              <span class="settings-row-hint">作品页和原声交流中呈现的称呼，支持中文与特殊字符</span>
            </div>
            <div class="settings-row-control">
              {editingName ? (
                <div class="settings-inline-edit">
                  <input
                    class="settings-inline-input"
                    value={draftName}
                    maxLength={40}
                    autoFocus
                    placeholder="输入展示名称"
                    disabled={busy}
                    onInput={(e) => setDraftName(e.currentTarget.value)}
                    onKeyDown={(e) => {
                      if (e.key === "Enter") void saveName();
                      if (e.key === "Escape") setEditingName(false);
                    }}
                  />
                  <span class="settings-inline-counter">{draftName.length}/40</span>
                  <button
                    type="button"
                    class="settings-inline-btn cancel"
                    disabled={busy}
                    onClick={() => {
                      setDraftName(profile.me.display_name);
                      setEditingName(false);
                    }}
                  >
                    取消
                  </button>
                  <button
                    type="button"
                    class="settings-inline-btn save"
                    disabled={busy || !draftName.trim() || draftName.trim() === profile.me.display_name}
                    onClick={() => void saveName()}
                  >
                    保存
                  </button>
                </div>
              ) : (
                <button
                  type="button"
                  class="settings-editable"
                  onClick={() => {
                    setDraftName(profile.me.display_name);
                    setEditingName(true);
                  }}
                  title="点击修改展示名称"
                >
                  <span>{profile.me.display_name}</span>
                  <svg class="pencil-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" aria-hidden="true">
                    <path d="M17 3a2.85 2.83 0 1 1 4 4L7.5 20.5 2 22l1.5-5.5Z" />
                  </svg>
                </button>
              )}
            </div>
          </div>

          <div class="settings-row">
            <div class="settings-row-meta">
              <span class="settings-row-label">主账号邮箱</span>
              <span class="settings-row-hint">接收动态推送与安全登录的邮箱地址</span>
            </div>
            <div class="settings-row-control">
              <span class="settings-value settings-value--mono">{profile.email || "未绑定"}</span>
            </div>
          </div>

          <div class="settings-row settings-row--last">
            <div class="settings-row-meta">
              <span class="settings-row-label">平台身份</span>
              <span class="settings-row-hint">已开通即时发布、合集归属与作者工作台管理权限</span>
            </div>
            <div class="settings-row-control">
              <span class="settings-chip settings-chip--accent">
                <span class="settings-chip-dot" />
                独立创作者
              </span>
            </div>
          </div>
        </div>
      </section>

      {/* 3. 登录方式与安全凭据 (Sign-in Methods Card) */}
      <section class="settings-card">
        <header class="settings-card-head">
          <div class="settings-card-head-row">
            <div class="settings-card-heading">
              <h2 class="settings-card-title">登录方式</h2>
              <button
                type="button"
                class={`settings-card-tip-btn ${tipOpen ? "is-open" : ""}`}
                aria-expanded={tipOpen}
                aria-label="查看登录方式机制说明"
                onClick={() => setTipOpen((v) => !v)}
              >
                ?
              </button>
            </div>
          </div>
          <p class="settings-card-desc">任何已绑定的登录通道均指向当前唯一账号，无缝切换设备无缝登录。</p>
          {tipOpen ? (
            <div class="settings-card-tip" role="note">
              平台实行单账号统一体系：GitHub 授权与邮箱验证码登录双通道均指向同一作品库与开发者令牌，无论从哪一条路径进入，均不会产生分裂账号。
            </div>
          ) : null}
        </header>

        <div class="settings-card-body">
          {/* GitHub 账号 */}
          <div class="settings-row">
            <div class="settings-row-meta">
              <span class="settings-row-label">GitHub 账号</span>
              <span class="settings-row-hint">用于作品签名、身份识别与命令行授权</span>
            </div>
            <div class="settings-row-control">
              {profile.me.login ? (
                <span class="settings-chip settings-chip--pos">
                  <span class="settings-chip-dot" />
                  已连接 @{profile.me.login}
                </span>
              ) : (
                <a class="settings-action settings-action--primary" href={`${githubLoginUrl}?return_to=${encodeURIComponent("/console/#/token")}`}>
                  连接 GitHub
                </a>
              )}
            </div>
          </div>

          {/* 邮箱登录 */}
          <div class="settings-row settings-row--last">
            <div class="settings-row-meta">
              <span class="settings-row-label">邮箱 Magic Link 登录</span>
              <span class="settings-row-hint">无需密码，向该邮箱发送一次性安全登录链接</span>
            </div>
            <div class="settings-row-control">
              {profile.email && !showEmailInput ? (
                <div style={{ display: "flex", alignItems: "center", gap: "8px" }}>
                  <span class="settings-chip settings-chip--pos">
                    <span class="settings-chip-dot" />
                    已连接
                  </span>
                  <button type="button" class="settings-action" onClick={() => setShowEmailInput(true)}>
                    更换
                  </button>
                </div>
              ) : (
                <form onSubmit={linkEmail} style={{ display: "flex", alignItems: "center", gap: "6px" }}>
                  <input
                    type="email"
                    class="settings-inline-input"
                    value={email}
                    placeholder="输入新邮箱地址"
                    required
                    disabled={busy}
                    onInput={(e) => setEmail(e.currentTarget.value)}
                  />
                  <button type="submit" class="settings-inline-btn save" disabled={busy || !email.trim()}>
                    发送链接
                  </button>
                  {profile.email ? (
                    <button type="button" class="settings-inline-btn cancel" onClick={() => setShowEmailInput(false)}>
                      取消
                    </button>
                  ) : null}
                </form>
              )}
            </div>
          </div>
        </div>
      </section>

      {/* 4. 开发者令牌管理 (Developer Tokens Card) */}
      <section class="settings-card">
        <header class="settings-card-head">
          <div class="settings-card-head-row">
            <div class="settings-card-heading">
              <h2 class="settings-card-title">开发者令牌</h2>
            </div>
            <button
              type="button"
              class="settings-action settings-action--primary"
              disabled={busy}
              onClick={createToken}
            >
              + 生成新令牌
            </button>
          </div>
          <p class="settings-card-desc">用于 playtest 命令行发布、CI/CD 自动化流水线及 AI 助手交互，拥有作品管理权限。</p>
        </header>

        {token ? (
          <div class="settings-token-box">
            <div class="settings-token-box-head">
              <span class="settings-token-box-badge">✓ 新令牌已就绪</span>
              <span class="settings-token-box-warn">离开本页后将不再完整呈现，请妥善保存</span>
            </div>
            <div class="settings-token-box-row">
              <input class="settings-token-box-input" value={token} readOnly onClick={(e) => e.currentTarget.select()} />
              <button type="button" class="settings-token-box-btn" onClick={copyToken}>
                {copied ? "已复制 ✓" : "复制令牌"}
              </button>
            </div>
          </div>
        ) : null}

        <div class="settings-card-body">
          {tokens.length === 0 ? (
            <div class="token-empty-state">
              暂无生成的开发者令牌。点击右上角即可生成新令牌。
            </div>
          ) : (
            tokens.map((t) => (
              <div class="token-row-item" key={t.id}>
                <div class="token-row-left">
                  <div class="token-row-info">
                    <span class="token-row-prefix">pt_{t.id.slice(0, 8)}••••••••</span>
                    <span class="token-row-time">创建于 {new Date(t.created_at).toLocaleDateString()}</span>
                    <span class="settings-chip settings-chip--pos">
                      <span class="settings-chip-dot" />
                      活跃中
                    </span>
                  </div>
                </div>
                <div>
                  {revokingId === t.id ? (
                    <div style={{ display: "flex", alignItems: "center", gap: "6px" }}>
                      <span style={{ fontSize: "12px", color: "var(--danger, #f87171)" }}>确认撤销？</span>
                      <button
                        type="button"
                        class="settings-action settings-action--danger"
                        disabled={busy}
                        onClick={() => void confirmRevoke(t.id)}
                      >
                        确认
                      </button>
                      <button
                        type="button"
                        class="settings-action"
                        disabled={busy}
                        onClick={() => setRevokingId(null)}
                      >
                        取消
                      </button>
                    </div>
                  ) : (
                    <button
                      type="button"
                      class="settings-action settings-action--danger"
                      disabled={busy}
                      onClick={() => setRevokingId(t.id)}
                    >
                      撤销
                    </button>
                  )}
                </div>
              </div>
            ))
          )}
        </div>
      </section>
    </div>
  );
}

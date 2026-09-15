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
      setError(err instanceof Error ? err.message : "Could not link that email. Try again.");
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
      setMessage("Saved.");
      setTimeout(() => setMessage(""), 2500);
    } catch (err) {
      setError(err instanceof Error ? err.message : "Could not save. Try again.");
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
      setError(err instanceof Error ? err.message : "Could not create a token. Try again.");
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
      setMessage("Token revoked.");
      setTimeout(() => setMessage(""), 2500);
    } catch (err) {
      setError(err instanceof Error ? err.message : "Could not revoke that token. Try again.");
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
          <h1>Settings</h1>
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
          <button type="button" class="notice-dismiss" onClick={() => setError("")} aria-label="Dismiss">×</button>
        </div>
      ) : null}

      {message ? (
        <div class="account-notice success" role="status">
          <svg class="icon" viewBox="0 0 24 24" aria-hidden="true" fill="none" stroke="currentColor" stroke-width="2">
            <path d="M22 11.08V12a10 10 0 1 1-5.93-9.14" />
            <polyline points="22 4 12 14.01 9 11.01" />
          </svg>
          <span>{message}</span>
          <button type="button" class="notice-dismiss" onClick={() => setMessage("")} aria-label="Dismiss">×</button>
        </div>
      ) : null}

      {/* 1. 创作者身份 (Identity Surface) */}
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
              <span>{profile.me.login ? `@${profile.me.login}` : (profile.email || "Author")}</span>
            </div>
          </div>

          <div class="settings-identity-actions">
            <a class="settings-action" href="/me" title="View your public page">
              <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" aria-hidden="true">
                <path d="M18 13v6a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V8a2 2 0 0 1 2-2h6" />
                <polyline points="15 3 21 3 21 9" />
                <line x1="10" y1="14" x2="21" y2="3" />
              </svg>
              Public page
            </a>
            <button class="settings-action settings-action--danger" type="button" onClick={onLogout} title="Sign out">
              <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" aria-hidden="true">
                <path d="M9 21H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h4" />
                <polyline points="16 17 21 12 16 7" />
                <line x1="21" y1="12" x2="9" y2="12" />
              </svg>
              Sign out
            </button>
          </div>
        </div>
      </section>

      {/* 2. 资料 (Profile Card) */}
      <section class="settings-card">
        <header class="settings-card-head">
          <div class="settings-card-head-row">
            <div class="settings-card-heading">
              <h2 class="settings-card-title">Profile</h2>
            </div>
          </div>
        </header>

        <div class="settings-card-body">
          <div class="settings-row">
            <div class="settings-row-meta">
              <span class="settings-row-label">Display name</span>
            </div>
            <div class="settings-row-control">
              {editingName ? (
                <div class="settings-inline-edit">
                  <input
                    class="settings-inline-input"
                    value={draftName}
                    maxLength={40}
                    autoFocus
                    placeholder="Your display name"
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
                    Cancel
                  </button>
                  <button
                    type="button"
                    class="settings-inline-btn save"
                    disabled={busy || !draftName.trim() || draftName.trim() === profile.me.display_name}
                    onClick={() => void saveName()}
                  >
                    Save
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
                  title="Click to edit"
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
              <span class="settings-row-label">Email</span>
            </div>
            <div class="settings-row-control">
              <span class="settings-value settings-value--mono">{profile.email || "Not linked"}</span>
            </div>
          </div>

          <div class="settings-row settings-row--last">
            <div class="settings-row-meta">
              <span class="settings-row-label">Account</span>
            </div>
            <div class="settings-row-control">
              <span class="settings-chip settings-chip--accent">
                <span class="settings-chip-dot" />
                Independent author
              </span>
            </div>
          </div>
        </div>
      </section>

      {/* 3. 登录方式 (Sign-in Methods Card) */}
      <section class="settings-card">
        <header class="settings-card-head">
          <div class="settings-card-head-row">
            <div class="settings-card-heading">
              <h2 class="settings-card-title">Sign-in methods</h2>
              <div class="settings-tip-anchor">
                <button
                  type="button"
                  class="settings-card-tip-btn"
                  aria-label="About sign-in methods"
                >
                  ?
                </button>
                <div class="settings-tip-popover" role="tooltip">
                  GitHub and email both sign you into this same account. Your projects and tokens stay the same either way.
                </div>
              </div>
            </div>
          </div>
        </header>

        <div class="settings-card-body">
          <div class="settings-row">
            <div class="settings-row-meta">
              <span class="settings-row-label">GitHub</span>
            </div>
            <div class="settings-row-control">
              {profile.me.login ? (
                <span class="settings-chip settings-chip--pos">
                  <span class="settings-chip-dot" />
                  Connected as @{profile.me.login}
                </span>
              ) : (
                <a class="settings-action settings-action--primary" href={`${githubLoginUrl}?return_to=${encodeURIComponent("/console/#/token")}`}>
                  Connect GitHub
                </a>
              )}
            </div>
          </div>

          <div class="settings-row settings-row--last">
            <div class="settings-row-meta">
              <span class="settings-row-label">Email sign-in</span>
            </div>
            <div class="settings-row-control">
              {profile.email && !showEmailInput ? (
                <div style={{ display: "flex", alignItems: "center", gap: "8px" }}>
                  <span class="settings-chip settings-chip--pos">
                    <span class="settings-chip-dot" />
                    Connected
                  </span>
                  <button type="button" class="settings-action" onClick={() => setShowEmailInput(true)}>
                    Change
                  </button>
                </div>
              ) : (
                <form onSubmit={linkEmail} style={{ display: "flex", alignItems: "center", gap: "6px" }}>
                  <input
                    type="email"
                    class="settings-inline-input"
                    value={email}
                    placeholder="you@example.com"
                    required
                    disabled={busy}
                    onInput={(e) => setEmail(e.currentTarget.value)}
                  />
                  <button type="submit" class="settings-inline-btn save" disabled={busy || !email.trim()}>
                    Send link
                  </button>
                  {profile.email ? (
                    <button type="button" class="settings-inline-btn cancel" onClick={() => setShowEmailInput(false)}>
                      Cancel
                    </button>
                  ) : null}
                </form>
              )}
            </div>
          </div>
        </div>
      </section>

      {/* 4. 开发者令牌 (Developer Tokens Card) */}
      <section class="settings-card">
        <header class="settings-card-head">
          <div class="settings-card-head-row">
            <div class="settings-card-heading">
              <h2 class="settings-card-title">Access tokens</h2>
            </div>
            <button
              type="button"
              class="settings-action settings-action--primary"
              disabled={busy}
              onClick={createToken}
            >
              + New token
            </button>
          </div>
        </header>

        {token ? (
          <div class="settings-token-box">
            <div class="settings-token-box-head">
              <span class="settings-token-box-badge">New token</span>
              <span class="settings-token-box-warn">Copy it now, it will not be shown again</span>
            </div>
            <div class="settings-token-box-row">
              <input class="settings-token-box-input" value={token} readOnly onClick={(e) => e.currentTarget.select()} />
              <button type="button" class="settings-token-box-btn" onClick={copyToken}>
                {copied ? "Copied ✓" : "Copy"}
              </button>
            </div>
          </div>
        ) : null}

        <div class="settings-card-body">
          {tokens.length === 0 ? (
            <div class="token-empty-state">
              No tokens yet. Create one to sign in the CLI on a machine without a browser.
            </div>
          ) : (
            tokens.map((t) => (
              <div class="token-row-item" key={t.id}>
                <div class="token-row-left">
                  <div class="token-row-info">
                    <span class="token-row-prefix">pt_{t.id.slice(0, 8)}••••••••</span>
                    <span class="token-row-time">{new Date(t.created_at).toLocaleDateString()}</span>
                    <span class="settings-chip settings-chip--pos">
                      <span class="settings-chip-dot" />
                      Active
                    </span>
                  </div>
                </div>
                <div>
                  {revokingId === t.id ? (
                    <div style={{ display: "flex", alignItems: "center", gap: "6px" }}>
                      <button
                        type="button"
                        class="settings-action settings-action--danger"
                        disabled={busy}
                        onClick={() => void confirmRevoke(t.id)}
                      >
                        Confirm revoke
                      </button>
                      <button
                        type="button"
                        class="settings-action"
                        disabled={busy}
                        onClick={() => setRevokingId(null)}
                      >
                        Cancel
                      </button>
                    </div>
                  ) : (
                    <button
                      type="button"
                      class="settings-action settings-action--danger"
                      disabled={busy}
                      onClick={() => setRevokingId(t.id)}
                    >
                      Revoke
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

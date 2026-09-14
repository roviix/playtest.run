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
  const [busy, setBusy] = useState(false);
  const [message, setMessage] = useState("");
  const [error, setError] = useState("");
  const [token, setToken] = useState("");
  const [copied, setCopied] = useState(false);
  const [name, setName] = useState(profile.me.display_name);
  const [tokens, setTokens] = useState<AccessToken[]>([]);

  const loadTokens = () =>
    account.tokens().then(setTokens).catch((err: Error) => setError(err.message));

  useEffect(() => {
    void loadTokens();
  }, []);

  async function linkEmail(event: Event) {
    event.preventDefault();
    setBusy(true);
    setError("");
    setMessage("");
    try {
      const result = await account.email(email, "/console/#/token", true);
      setMessage(result.message);
      setEmail("");
    } catch (err) {
      setError(err instanceof Error ? err.message : "Failed to link email, please try again.");
    } finally {
      setBusy(false);
    }
  }

  async function saveName(event: Event) {
    event.preventDefault();
    if (!name.trim() || name.trim() === profile.me.display_name) return;
    setBusy(true);
    setError("");
    try {
      await account.profile(name.trim());
      await onRefresh();
      setMessage("Display name updated successfully.");
      setTimeout(() => setMessage(""), 3000);
    } catch (err) {
      setError(err instanceof Error ? err.message : "Failed to update display name.");
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
      setError(err instanceof Error ? err.message : "Failed to create token, please try again.");
    } finally {
      setBusy(false);
    }
  }

  async function revokeToken(id: string) {
    if (
      !confirm(
        "Revoke this access token? CLI and assistants using it will lose access to manage works. Web sign-in is unaffected."
      )
    ) {
      return;
    }
    setBusy(true);
    try {
      await account.revokeToken(id);
      if (token) setToken("");
      await loadTokens();
    } catch (err) {
      setError(err instanceof Error ? err.message : "Failed to revoke token.");
    } finally {
      setBusy(false);
    }
  }

  return (
    <div class="account-page">
      <header class="stage-head">
        <div>
          <h1>Account &amp; Settings</h1>
          <p class="muted">Manage your public creator profile, connected credentials, and developer access tokens.</p>
        </div>
      </header>

      {error ? (
        <div class="account-notice error" role="alert">
          <svg class="icon" viewBox="0 0 24 24" aria-hidden="true">
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
          <svg class="icon" viewBox="0 0 24 24" aria-hidden="true">
            <path d="M22 11.08V12a10 10 0 1 1-5.93-9.14" />
            <polyline points="22 4 12 14.01 9 11.01" />
          </svg>
          <span>{message}</span>
          <button class="button quiet small" onClick={onRefresh}>Refresh</button>
        </div>
      ) : null}

      {/* Hero Profile Card */}
      <section class="account-hero-card">
        <div class="hero-identity">
          <div class="hero-avatar-wrap">
            {profile.me.avatar_url ? (
              <img class="hero-avatar" src={profile.me.avatar_url} alt="" />
            ) : (
              <div class="hero-avatar placeholder">
                {(profile.me.display_name || "P").charAt(0).toUpperCase()}
              </div>
            )}
          </div>
          <div class="hero-info">
            <div class="hero-name-row">
              <h2>{profile.me.display_name}</h2>
              <span class="hero-badge">Creator</span>
              {profile.me.login ? <span class="hero-handle">@{profile.me.login}</span> : null}
            </div>
            <p class="hero-summary">One account across playing, publishing, and community engagement.</p>
          </div>
        </div>

        <div class="hero-actions">
          <a class="button quiet hero-btn" href="/me">
            <svg class="icon" viewBox="0 0 24 24" aria-hidden="true">
              <path d="M18 8a6 6 0 0 0-12 0c0 7-3 7-3 9h18c0-2-3-2-3-9" />
              <path d="M10 21h4" />
            </svg>
            Followed Works &amp; Feed
          </a>
          <button class="button quiet hero-btn signout" type="button" onClick={onLogout}>
            <svg class="icon" viewBox="0 0 24 24" aria-hidden="true">
              <path d="M9 21H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h4" />
              <polyline points="16 17 21 12 16 7" />
              <line x1="21" y1="12" x2="9" y2="12" />
            </svg>
            Sign Out
          </button>
        </div>
      </section>

      {/* Section 1: Public Profile */}
      <section class="account-card">
        <div class="account-card-head">
          <div class="head-title-row">
            <svg class="icon head-icon" viewBox="0 0 24 24" aria-hidden="true">
              <path d="M20 21v-2a4 4 0 0 0-4-4H8a4 4 0 0 0-4 4v2" />
              <circle cx="12" cy="7" r="4" />
            </svg>
            <h2>Public Profile</h2>
          </div>
          <p>Your display name shown on project invitation cards, player feedback streams, and collections.</p>
        </div>

        <form class="account-field-group" onSubmit={saveName}>
          <div class="field-header">
            <label for="display-name">Display Name</label>
            <span class="field-counter">{name.length} / 40</span>
          </div>
          <div class="field-control-row">
            <input
              id="display-name"
              class="account-input"
              value={name}
              maxLength={40}
              required
              placeholder="Your public name"
              onInput={(event) => setName(event.currentTarget.value)}
            />
            <button
              class="button primary"
              type="submit"
              disabled={busy || !name.trim() || name.trim() === profile.me.display_name}
            >
              {busy ? "Saving…" : "Save Name"}
            </button>
          </div>
        </form>
      </section>

      {/* Section 2: Sign-in Methods */}
      <section class="account-card">
        <div class="account-card-head">
          <div class="head-title-row">
            <svg class="icon head-icon" viewBox="0 0 24 24" aria-hidden="true">
              <rect x="3" y="11" width="18" height="11" rx="2" ry="2" />
              <path d="M7 11V7a5 5 0 0 1 10 0v4" />
            </svg>
            <h2>Sign-in Methods</h2>
          </div>
          <p>Both methods will access this account. Works and follows from other accounts are not merged.</p>
        </div>

        <div class="methods-grid">
          {/* Email Method */}
          <div class="method-box">
            <div class="method-box-header">
              <div class="method-title-wrap">
                <svg class="icon method-icon" viewBox="0 0 24 24" aria-hidden="true">
                  <path d="M4 4h16c1.1 0 2 .9 2 2v12c0 1.1-.9 2-2 2H4c-1.1 0-2-.9-2-2V6c0-1.1.9-2 2-2z" />
                  <polyline points="22,6 12,13 2,6" />
                </svg>
                <span class="method-name">Email</span>
              </div>
              {profile.email ? (
                <span class="status-pill active">
                  <span class="status-dot"></span> Linked
                </span>
              ) : (
                <span class="status-pill idle">Unlinked</span>
              )}
            </div>

            <div class="method-box-body">
              {profile.email ? (
                <div class="method-identity mono">{profile.email}</div>
              ) : identity.email_available ? (
                <form class="method-link-form" onSubmit={linkEmail}>
                  <input
                    id="link-email"
                    class="account-input"
                    type="email"
                    autoComplete="email"
                    placeholder="Enter email to link"
                    required
                    value={email}
                    onInput={(event) => setEmail(event.currentTarget.value)}
                  />
                  <button class="button small" type="submit" disabled={busy || !email.trim()}>
                    Link
                  </button>
                </form>
              ) : (
                <span class="muted text-sm">Email service currently unavailable</span>
              )}
            </div>
          </div>

          {/* GitHub Method */}
          <div class="method-box">
            <div class="method-box-header">
              <div class="method-title-wrap">
                <svg class="icon method-icon" viewBox="0 0 24 24" aria-hidden="true">
                  <path d="M9 19c-5 1.5-5-2.5-7-3m14 6v-3.87a3.37 3.37 0 0 0-.94-2.61c3.14-.35 6.44-1.54 6.44-7A5.44 5.44 0 0 0 20 4.77 5.07 5.07 0 0 0 19.91 1S18.73.65 16 2.48a13.38 13.38 0 0 0-7 0C6.27.65 5.09 1 5.09 1A5.07 5.07 0 0 0 5 4.77a5.44 5.44 0 0 0-1.5 3.78c0 5.42 3.3 6.61 6.44 7A3.37 3.37 0 0 0 9 18.13V22" />
                </svg>
                <span class="method-name">GitHub</span>
              </div>
              {profile.me.login ? (
                <span class="status-pill active">
                  <span class="status-dot"></span> Connected
                </span>
              ) : (
                <span class="status-pill idle">Unconnected</span>
              )}
            </div>

            <div class="method-box-body">
              {profile.me.login ? (
                <div class="method-identity mono">@{profile.me.login}</div>
              ) : identity.github_available ? (
                <a
                  class="button quiet small method-connect-btn"
                  href={`${githubLoginUrl}?link=true&return_to=${encodeURIComponent("/console/#/token")}`}
                >
                  Connect GitHub
                </a>
              ) : (
                <span class="muted text-sm">GitHub OAuth unavailable</span>
              )}
            </div>
          </div>
        </div>
      </section>

      {/* Section 3: Developer Settings & Access Tokens */}
      <section class="account-card developer-section">
        <div class="account-card-head">
          <div class="head-title-row">
            <svg class="icon head-icon" viewBox="0 0 24 24" aria-hidden="true">
              <path d="M21 2l-2 2m-1.5 1.5L16 7m-1.5 1.5L13 10m-1.5 1.5L10 13m-1.5 1.5L7 16m-1.5 1.5L4 19m-2 2l2-2" />
              <circle cx="7.5" cy="7.5" r="4.5" />
            </svg>
            <h2>Developer Settings</h2>
          </div>
          <p>Personal access tokens allow CLI (<code>playtest login</code>) and AI assistants (Claude Code, Cursor, Windsurf) to publish and manage your projects.</p>
        </div>

        <div class="token-actions-bar">
          <button class="button primary" type="button" disabled={busy} onClick={createToken}>
            <svg class="icon" viewBox="0 0 24 24" aria-hidden="true">
              <line x1="12" y1="5" x2="12" y2="19" />
              <line x1="5" y1="12" x2="19" y2="12" />
            </svg>
            {busy ? "Creating…" : "Generate New Token"}
          </button>
        </div>

        {token ? (
          <div class="token-reveal-banner" role="region" aria-label="Newly generated token">
            <div class="reveal-head">
              <span class="reveal-badge">New Token Generated</span>
              <span class="reveal-warning">Copy this token now. It will never be shown again.</span>
            </div>
            <div class="reveal-control">
              <input
                id="created-token"
                class="token-input mono"
                type="text"
                readOnly
                value={token}
                autoComplete="off"
                onClick={(e) => (e.target as HTMLInputElement).select()}
              />
              <button
                class="button copy-token-btn"
                type="button"
                onClick={async () => {
                  try {
                    await navigator.clipboard.writeText(token);
                    setCopied(true);
                  } catch {
                    setError("Failed to copy. Please select and copy manually.");
                  }
                }}
              >
                {copied ? "Copied ✓" : "Copy Token"}
              </button>
            </div>
          </div>
        ) : null}

        <div class="token-list-wrap">
          <h3 class="token-list-title">Active Access Tokens ({tokens.length})</h3>
          {tokens.length > 0 ? (
            <ul class="token-list">
              {tokens.map((item) => (
                <li key={item.id} class="token-item">
                  <div class="token-item-info">
                    <span class="token-key-icon" aria-hidden="true">
                      <svg class="icon" viewBox="0 0 24 24">
                        <path d="M21 2l-2 2m-1.5 1.5L16 7m-1.5 1.5L13 10m-1.5 1.5L10 13" />
                        <circle cx="7.5" cy="7.5" r="4.5" />
                      </svg>
                    </span>
                    <div class="token-meta">
                      <span class="token-date">
                        Created on {new Date(item.created_at).toLocaleDateString()} at{" "}
                        {new Date(item.created_at).toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" })}
                      </span>
                      <span class="token-active-badge">Active</span>
                    </div>
                  </div>
                  <button
                    class="button quiet danger small"
                    type="button"
                    disabled={busy}
                    onClick={() => revokeToken(item.id)}
                  >
                    Revoke
                  </button>
                </li>
              ))}
            </ul>
          ) : (
            <div class="empty-tokens-placeholder">
              <svg class="icon empty-icon" viewBox="0 0 24 24" aria-hidden="true">
                <rect x="3" y="11" width="18" height="11" rx="2" ry="2" />
                <path d="M7 11V7a5 5 0 0 1 10 0v4" />
              </svg>
              <p>No active personal access tokens.</p>
            </div>
          )}
        </div>
      </section>
    </div>
  );
}

import { useState } from "preact/hooks";
import { account } from "../api";

export function DevicePage() {
  const [code, setCode] = useState("");
  const [preview, setPreview] = useState<{ anonymous_works: number } | null>(null);
  const [done, setDone] = useState(false);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");

  async function submit(event: Event) {
    event.preventDefault();
    if (busy) return;
    setBusy(true);
    setError("");
    try {
      if (preview) {
        await account.approveDevice(code);
        setDone(true);
      } else {
        setPreview(await account.previewDevice(code));
      }
    } catch (error) {
      setError(error instanceof Error ? error.message : "Authorization did not complete. Try again.");
    } finally {
      setBusy(false);
    }
  }

  return (
    <section class="device-page">
      <div class="device-card">
        <div class="device-badge">
          <svg class="icon" viewBox="0 0 24 24" width="22" height="22" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <polyline points="4 17 10 11 4 5" />
            <line x1="12" y1="19" x2="20" y2="19" />
          </svg>
        </div>

        <header class="device-head">
          <h1>{done ? "Device connected" : "Connect the CLI"}</h1>
          <p class="muted">
            {done
              ? "This device is linked to your account. Go back to the terminal to continue."
              : "Run playtest login in your terminal, then enter the 8-character code it shows."}
          </p>
        </header>

        {done ? (
          <div class="device-done-body">
            <p class="device-tip">The terminal is signed in. Projects you publish from now on stay in your account for good.</p>
            <a class="button primary" href="#/">Open My Projects</a>
          </div>
        ) : (
          <form class="device-form" onSubmit={submit}>
            <div class="device-input-wrap">
              <label for="device-code" class="device-label">One-time device code</label>
              <input
                id="device-code"
                autoComplete="off"
                spellcheck={false}
                autoCapitalize="characters"
                maxLength={9}
                placeholder="XXXX-XXXX"
                value={code}
                readOnly={!!preview}
                required
                onInput={(event) => {
                  let val = event.currentTarget.value.toUpperCase().replace(/[^A-Z0-9-]/g, "");
                  if (val.length === 4 && !val.includes("-") && code.length < 4) {
                    val = val + "-";
                  }
                  setCode(val);
                }}
              />
            </div>

            {preview ? (
              <div class="device-preview-box">
                <p class="preview-text">
                  <strong>This grants</strong> the CLI permission to publish, update and manage your projects. You can revoke the token any time in Settings.
                </p>
                {preview.anonymous_works ? (
                  <p class="preview-sub">
                    ✓ The <strong>{preview.anonymous_works}</strong> temporary {preview.anonymous_works === 1 ? "project" : "projects"} published from this terminal will move into your account and stop expiring.
                  </p>
                ) : null}
              </div>
            ) : (
              <p class="notice-note">
                🔒 Only enter a code that your own terminal showed after <code>playtest login</code>. Never enter a code someone sent you.
              </p>
            )}

            <div class="device-actions">
              <button class="button primary" type="submit" disabled={busy || !code.trim() || code.length < 4}>
                {busy ? "Confirming…" : preview ? "Allow" : "Continue"}
              </button>
              {preview ? (
                <button class="button quiet" type="button" disabled={busy} onClick={() => setPreview(null)}>
                  Use another code
                </button>
              ) : null}
            </div>
          </form>
        )}

        {error ? <p class="notice" role="alert">{error}</p> : null}
      </div>
    </section>
  );
}

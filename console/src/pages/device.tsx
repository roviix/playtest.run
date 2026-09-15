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
      setError(error instanceof Error ? error.message : "授权未完成，请重试。");
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
          <h1>{done ? "已授权连接" : "连接命令行"}</h1>
          <p class="muted">
            {done
              ? "这台设备已成功与你的账号绑定，可以回到终端继续操作。"
              : "在终端运行 playtest login 后，输入屏幕上显示的 8 位代码。"}
          </p>
        </header>

        {done ? (
          <div class="device-done-body">
            <p class="device-tip">回到终端，已自动登录完毕；之后发布的作品将永久保留在你的账号下。</p>
            <a class="button primary" href="#/">查看我的作品</a>
          </div>
        ) : (
          <form class="device-form" onSubmit={submit}>
            <div class="device-input-wrap">
              <label for="device-code" class="device-label">一次性授权代码</label>
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
                  <strong>授权权限</strong>：允许此命令行发布、更新和管理你的作品。令牌可随时在账号设置中撤销。
                </p>
                {preview.anonymous_works ? (
                  <p class="preview-sub">
                    ✓ 将自动把该终端上的 <strong>{preview.anonymous_works}</strong> 件临时作品合并保留至当前账号。
                  </p>
                ) : null}
              </div>
            ) : (
              <p class="notice-note">
                🔒 仅在你自己的终端执行 <code>playtest login</code> 时输入，切勿输入别人发来的代码。
              </p>
            )}

            <div class="device-actions">
              <button class="button primary" type="submit" disabled={busy || !code.trim() || code.length < 4}>
                {busy ? "正在确认…" : preview ? "允许连接" : "继续"}
              </button>
              {preview ? (
                <button class="button quiet" type="button" disabled={busy} onClick={() => setPreview(null)}>
                  更换代码
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

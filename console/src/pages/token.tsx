// 粘贴令牌。
//
// v0.1 没有登录：CLI 第一次运行会拿到一个 24 小时的匿名令牌，把它粘进来就能看结果。
// GitHub 登录第三周接，接上之后这一页留着——自托管和 CI 里仍然是粘一个令牌最省事。

import { useState } from "preact/hooks";

import { forgetToken, readToken, saveToken } from "../api";
import { go } from "../router";

export function TokenPage() {
  const existing = readToken();
  const [value, setValue] = useState("");
  const [saved, setSaved] = useState(false);

  function submit(event: Event) {
    event.preventDefault();
    const token = value.trim();
    if (!token) return;
    saveToken(token);
    setSaved(true);
    go({ name: "sites" });
  }

  return (
    <>
      <header class="page-head">
        <h1>令牌</h1>
        <p class="muted">
          令牌只存在这台设备的浏览器里，不会发给第三方。
          {existing ? "现在这台设备上已经有一个。" : ""}
        </p>
      </header>

      <form class="card" onSubmit={submit}>
        <label class="field">
          <span>把 CLI 给你的令牌粘在这里</span>
          <input
            type="password"
            autocomplete="off"
            spellcheck={false}
            value={value}
            placeholder="例如 vBoqXNsFgmJsWa5ZzE_WgKxT…"
            onInput={(event) => setValue((event.target as HTMLInputElement).value)}
          />
        </label>
        <p class="row-actions">
          <button class="button primary" type="submit">
            存下来
          </button>
          {existing ? (
            <button
              class="button"
              type="button"
              onClick={() => {
                forgetToken();
                setValue("");
                setSaved(false);
                go({ name: "token" });
                location.reload();
              }}
            >
              清掉这台设备上的令牌
            </button>
          ) : null}
        </p>
        {saved ? <p class="muted">存好了。</p> : null}
      </form>

      <div class="empty">
        <p class="muted">
          还没有令牌？在作品目录里运行 <code>playtest</code>，它会自己申请一个 24 小时的匿名链接，
          令牌就在那次输出里。
        </p>
      </div>
    </>
  );
}

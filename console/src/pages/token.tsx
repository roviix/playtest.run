// 登录，或者粘贴令牌。
//
// 正路是「用 GitHub 登录」：浏览器去 GitHub 授权一次，回来就是账号令牌，长期有效，
// 和终端里 playtest login 进的是同一个账号。手里若已经有一个匿名令牌，登录时会一起带过去，
// 那个身份下的作品归到账号里、不再 24 小时后失效。
// 粘贴令牌留着：自托管没配 GitHub、CI 里、或者只想看一眼匿名链接的结果时，粘一个最省事。

import { useState } from "preact/hooks";

import { forgetToken, githubLoginUrl, readToken, saveToken, type Me } from "../api";
import { go } from "../router";

export function TokenPage({ loginError, me }: { loginError: string | null; me: Me | null }) {
  const existing = readToken();
  const [value, setValue] = useState("");

  function submit(event: Event) {
    event.preventDefault();
    const token = value.trim();
    if (!token) return;
    saveToken(token);
    location.hash = "#/";
    location.reload();
  }

  const loggedIn = me?.kind === "github";

  return (
    <div class="account-page">
      <header class="stage-head">
        <h1>{loggedIn ? "账号" : "登录"}</h1>
        <p class="muted">
          {loggedIn
            ? `这台设备上登录的是 @${me?.login ?? me?.display_name}。`
            : existing
              ? "这台设备上是 24 小时匿名身份。登录后作品归到账号，不再到期。"
              : "令牌只存在这台设备的浏览器里。"}
        </p>
      </header>

      {loginError ? <p class="notice">登录失败：{loginError}</p> : null}

      {loggedIn ? (
        <section class="block">
          <div class="block-head">
            <h2>
              {me?.avatar_url ? <img class="avatar big" src={me.avatar_url} alt="" /> : null}@
              {me?.login ?? me?.display_name}
            </h2>
            <p class="muted">玩家在门禁页和广场上看到的名字。</p>
          </div>
        </section>
      ) : (
        <section class="block">
          <div class="block-head">
            <h2>用 GitHub 登录</h2>
            <p class="muted">
              和终端里 <code>playtest login</code> 是同一个账号。
            </p>
          </div>
          <p class="row-actions">
            <a class="button primary" href={githubLoginUrl}>
              用 GitHub 登录
            </a>
          </p>
        </section>
      )}

      <form class="block" onSubmit={submit}>
        <div class="block-head">
          <h2>{loggedIn ? "换令牌" : "粘贴令牌"}</h2>
          {existing ? null : (
            <p class="muted">
              运行过 <code>playtest</code> 之后，令牌在 <code>~/.config/playtest/config.json</code>。
            </p>
          )}
        </div>
        <div class="field-row">
          <label class="field grow">
            <span>令牌</span>
            <input
              type="password"
              autocomplete="off"
              spellcheck={false}
              value={value}
              placeholder="例如 vBoqXNsFgmJsWa5ZzE_WgKxT…"
              onInput={(event) => setValue((event.target as HTMLInputElement).value)}
            />
          </label>
          <button class="button" type="submit" disabled={!value.trim()}>
            保存
          </button>
        </div>
        {existing ? (
          <p class="row-actions">
            <button
              class="button quiet"
              type="button"
              onClick={() => {
                forgetToken();
                go({ name: "token" });
                location.reload();
              }}
            >
              {loggedIn ? "退出" : "清除令牌"}
            </button>
          </p>
        ) : null}
      </form>
    </div>
  );
}

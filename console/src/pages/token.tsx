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
              ? "这台设备上现在是一个 24 小时的匿名身份。登录之后它的作品会归到你的账号里，不再失效。"
              : "令牌只存在这台设备的浏览器里，不会发给第三方。"}
        </p>
      </header>

      {loginError ? <p class="notice">没登录成：{loginError}</p> : null}

      {loggedIn ? (
        <section class="block">
          <div class="block-head">
            <h2>
              {me?.avatar_url ? <img class="avatar big" src={me.avatar_url} alt="" /> : null}@
              {me?.login ?? me?.display_name}
            </h2>
            <p class="muted">玩家在门禁页和广场上看到的是同一张脸、同一个名字。</p>
          </div>
        </section>
      ) : (
        <section class="block">
          <div class="block-head">
            <h2>用 GitHub 登录</h2>
            <p class="muted">
              和终端里 <code>playtest login</code> 进的是同一个账号。GitHub 在你这里打不开的话，下面粘令牌也一样能看结果。
            </p>
          </div>
          <p class="row-actions">
            <a class="button primary" href={githubLoginUrl}>
              去 GitHub 授权
            </a>
          </p>
        </section>
      )}

      <form class="block" onSubmit={submit}>
        <div class="block-head">
          <h2>{loggedIn ? "换一个令牌" : "或者，粘贴 CLI 给你的令牌"}</h2>
          {existing ? null : (
            <p class="muted">
              在作品目录里运行 <code>playtest</code>，它会自己申请一个 24 小时的匿名链接，令牌在{" "}
              <code>~/.config/playtest/config.json</code> 里。
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
            存下来
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
              {loggedIn ? "退出这台设备" : "清掉这台设备上的令牌"}
            </button>
          </p>
        ) : null}
      </form>
    </div>
  );
}

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
    setBusy(true); setError("");
    try {
      if (preview) { await account.approveDevice(code); setDone(true); }
      else setPreview(await account.previewDevice(code));
    } catch (error) { setError(error instanceof Error ? error.message : "授权未完成，请重试。"); }
    finally { setBusy(false); }
  }

  return <section class="device-page">
    <header class="stage-head"><h1>{done ? "已授权" : "连接命令行"}</h1></header>
    {done ? <><p>回到终端，继续发布你的作品。</p><a class="button quiet" href="#/">我的作品</a></> : <form class="notice-form" onSubmit={submit}>
      <label for="device-code">终端里显示的代码</label>
      <input id="device-code" autoComplete="off" spellcheck={false} autoCapitalize="characters" maxLength={9} placeholder="XXXX-XXXX" value={code} readOnly={!!preview} required onInput={(event) => setCode(event.currentTarget.value.toUpperCase())} />
      {preview ? <><p>允许这台命令行管理你的作品，令牌可以随时在账号设置中撤销。</p>{preview.anonymous_works ? <p>同时把这台命令行的 {preview.anonymous_works} 件临时作品保留到你的账号。</p> : null}</> : null}
      <p class="notice-note">只有你刚在自己的终端运行了 <code>playtest login</code> 才继续。不要输入别人发来的代码。</p>
      <button class="button primary" disabled={busy || !code.trim()}>{busy ? "正在确认…" : preview ? "允许连接" : "继续"}</button>
      {preview ? <button class="button quiet" type="button" disabled={busy} onClick={() => setPreview(null)}>返回</button> : null}
    </form>}
    {error ? <p class="notice" role="alert">{error}</p> : null}
  </section>;
}

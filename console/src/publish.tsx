import { useRef, useState } from "preact/hooks";
import { href } from "./router";

const modes = [
  {
    label: "导出目录", command: "playtest ./dist", hint: "先构建项目，将 ./dist 换成导出目录。",
    parameters: [["./dist", "项目构建后的导出目录。"], ["--public", "可选，将作品放到广场；默认不上广场。"], ["再次发布", "原作品有效时，同一目录更新沿用链接。"]],
  },
  {
    label: "本地端口", command: "playtest 5173", hint: "先启动本地服务，并保持终端运行。",
    parameters: [["5173", "本地服务的端口。"], ["临时链接", "关闭终端后就不能访问。"]],
  },
  {
    label: "带后端", command: "playtest ./dist --backend 3000", hint: "先启动后端，发布期间保持终端运行。",
    parameters: [["./dist", "项目构建后的导出目录。"], ["--backend 3000", "后端端口，发布期间保持服务运行。"], ["请求转发", "静态文件直接发布，未匹配的请求转到后端；SPA 导航回退优先。"], ["--public", "可选，将作品放到广场；默认不上广场。"]],
  },
];

export function Publish() {
  const dialog = useRef<HTMLDialogElement>(null);
  const copyGeneration = useRef(0);
  const [mode, setMode] = useState(0);
  const [message, setMessage] = useState("");
  const [copied, setCopied] = useState(false);
  const selected = modes[mode];

  function resetCopy() {
    copyGeneration.current++;
    setMessage("");
    setCopied(false);
  }

  async function copy() {
    const generation = copyGeneration.current;
    try {
      await navigator.clipboard.writeText(selected.command);
      if (generation !== copyGeneration.current) return;
      setCopied(true);
      setMessage("已复制命令。");
    } catch {
      if (generation !== copyGeneration.current) return;
      setCopied(false);
      setMessage("复制失败，请选中命令手动复制。");
    }
  }

  return <>
    <button class="publish" type="button" onClick={() => { resetCopy(); dialog.current?.showModal(); }}>
      <svg class="icon" viewBox="0 0 24 24" aria-hidden="true"><path d="M12 5v14M5 12h14" /></svg> 发布作品
    </button>
    <dialog class="overlay" ref={dialog} aria-labelledby="publish-title" onClose={resetCopy}>
      <button class="overlay-back" type="button" aria-label="关闭发布说明" tabIndex={-1} onClick={() => dialog.current?.close()} />
      <div class="sheet publish-sheet">
        <div class="dialog-head"><h2 id="publish-title">发布作品</h2><button class="close" type="button" autoFocus aria-label="关闭发布说明" onClick={() => dialog.current?.close()}><svg class="icon" viewBox="0 0 24 24" aria-hidden="true"><path d="m6 6 12 12M6 18 18 6" /></svg></button></div>
        <div class="pub">
          <div class="cli">
            <div class="cli-bar">
              <div class="publish-tabs" role="group" aria-label="发布方式">
                {modes.map((item, index) => <button type="button" key={item.label} aria-pressed={mode === index} onClick={() => { setMode(index); resetCopy(); }}>{item.label}</button>)}
              </div>
              <button class="copy-command" type="button" aria-label={copied ? "已复制命令" : "复制命令"} title={copied ? "已复制" : "复制命令"} data-copied={copied || undefined} onClick={copy}>
                <svg class="icon copy-glyph" viewBox="0 0 24 24" aria-hidden="true"><rect x="8" y="8" width="12" height="12" rx="2" /><path d="M16 8V5a1 1 0 0 0-1-1H5a1 1 0 0 0-1 1v10a1 1 0 0 0 1 1h3" /></svg>
                <svg class="icon copied-glyph" viewBox="0 0 24 24" aria-hidden="true"><path d="m5 12 4 4L19 6" /></svg>
              </button>
            </div>
            <div class="codebox selected" tabIndex={0} role="region" aria-label="发布命令"><b aria-hidden="true">$</b><code>{selected.command}</code></div>
            <p class="leg selected">{selected.hint}</p>
          </div>
          <p class="pub-status" role="status" data-state={copied ? "success" : "error"}>{message}</p>
          <details class="pub-details selected" key={mode}><summary>参数说明<svg class="icon" viewBox="0 0 24 24" aria-hidden="true"><path d="m9 6 6 6-6 6" /></svg></summary><dl>{selected.parameters.map(([parameter, meaning]) => <div key={parameter}><dt>{parameter.startsWith("--") || parameter === "./dist" || parameter === "5173" ? <code>{parameter}</code> : parameter}</dt><dd>{meaning}</dd></div>)}</dl></details>
        </div>
        <div class="pub-foot"><a href="https://github.com/roviix/playtest.run/releases" target="_blank" rel="noreferrer">下载 playtest <svg class="icon" viewBox="0 0 24 24" aria-hidden="true"><path d="M7 17 16.2 7.8M17 17V8H8" /></svg></a><a href={href({ name: "docs", section: "publish" })} onClick={() => dialog.current?.close()}>发布文档 <svg class="icon" viewBox="0 0 24 24" aria-hidden="true"><path d="M5 12h14m-5-5 5 5-5 5" /></svg></a></div>
      </div>
    </dialog>
  </>;
}

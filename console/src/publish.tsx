import { useRef, useState } from "preact/hooks";
import { href } from "./router";
import { BrandMark } from "./auth";

const modes = [
  {
    label: "Export Folder",
    command: "playtest ./dist",
    hint: "Build your project first, then replace ./dist with your build output folder.",
  },
  {
    label: "Local Port",
    command: "playtest 3000",
    hint: "Start your local dev server, and keep your terminal open during sharing.",
  },
  {
    label: "Static + Backend",
    command: "playtest ./dist --backend 8000",
    hint: "Serves static files as usual; unmatched requests forward to your local backend.",
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
      setMessage("Command copied.");
    } catch {
      if (generation !== copyGeneration.current) return;
      setCopied(false);
      setMessage("Failed to copy automatically, please select and copy manually.");
    }
  }

  return <>
    <button class="publish" type="button" onClick={() => { resetCopy(); dialog.current?.showModal(); }}>
      <svg class="icon" viewBox="0 0 24 24" aria-hidden="true"><path d="M12 5v14M5 12h14" /></svg> Publish Work
    </button>
    <dialog class="overlay" ref={dialog} aria-labelledby="publish-title" onClose={resetCopy}>
      <button class="overlay-back" type="button" aria-label="Close publish dialog" tabIndex={-1} onClick={() => dialog.current?.close()} />
      <div class="sheet publish-sheet">
        <div class="dialog-head">
          <div class="dialog-title-wrap">
            <BrandMark />
            <h2 id="publish-title">Publish Work</h2>
          </div>
          <button class="close" type="button" autoFocus aria-label="Close publish dialog" onClick={() => dialog.current?.close()}>
            <svg class="icon" viewBox="0 0 24 24" aria-hidden="true"><path d="m6 6 12 12M6 18 18 6" /></svg>
          </button>
        </div>
        <div class="pub">
          <div class="publish-tabs" role="group" aria-label="Publish mode">
            {modes.map((item, index) => <button type="button" key={item.label} aria-pressed={mode === index} onClick={() => { setMode(index); resetCopy(); }}>{item.label}</button>)}
          </div>
          <div class="cli">
            <div class="cli-bar">
              <div class="cli-info">
                <span class="cli-dots" aria-hidden="true"></span>
                <span class="cli-meta">bash · CLI Publish</span>
              </div>
              <button class="copy-command" type="button" aria-label={copied ? "Command copied" : "Copy command"} title={copied ? "Copied" : "Copy command"} data-copied={copied || undefined} onClick={copy}>
                <svg class="icon copy-glyph" viewBox="0 0 24 24" aria-hidden="true"><rect x="8" y="8" width="12" height="12" rx="2" /><path d="M16 8V5a1 1 0 0 0-1-1H5a1 1 0 0 0-1 1v10a1 1 0 0 0 1 1h3" /></svg>
                <svg class="icon copied-glyph" viewBox="0 0 24 24" aria-hidden="true"><path d="m5 12 4 4L19 6" /></svg>
                <span class="copy-text">{copied ? "Copied" : "Copy"}</span>
              </button>
            </div>
            <div class="codebox selected" tabIndex={0} role="region" aria-label="Publish command"><b aria-hidden="true">$</b><code>{selected.command}</code></div>
            <p class="leg selected">
              <svg class="icon" viewBox="0 0 24 24" aria-hidden="true"><circle cx="12" cy="12" r="10" /><path d="M12 16v-4m0-4h.01" /></svg>
              <span>{selected.hint}</span>
            </p>
          </div>
          <p class="pub-status" role="status" data-state={copied ? "success" : "error"}>{message}</p>
        </div>
        <div class="pub-foot">
          <a href="https://github.com/roviix/playtest.run/releases" target="_blank" rel="noreferrer">
            <svg class="icon" viewBox="0 0 24 24" aria-hidden="true"><path d="M12 4v12m0 0-4-4m4 4 4-4M4 20h16" /></svg>
            Download CLI
          </a>
          <a href={href({ name: "docs", section: "publish" })} onClick={() => dialog.current?.close()}>
            <svg class="icon" viewBox="0 0 24 24" aria-hidden="true"><path d="M4 19.5A2.5 2.5 0 0 1 6.5 17H20M4 4.5A2.5 2.5 0 0 1 6.5 2H20v20H6.5A2.5 2.5 0 0 1 4 19.5v-15Z" /></svg>
            Documentation
          </a>
        </div>
      </div>
    </dialog>
  </>;
}

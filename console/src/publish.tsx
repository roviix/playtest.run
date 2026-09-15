import { useRef, useState } from "preact/hooks";
import { href } from "./router";
import { BrandMark } from "./auth";

const AI_PROMPT_ZH = `请将当前项目构建并发布到 playtest.run：
1. 检查或安装 CLI：curl -fsSL https://playtest.run/install.sh | bash
2. 依项目选择发布：含 index.html 产物运行 playtest <目录>（单页应用加 --spa），或本地服务运行 playtest <端口>。可用 -n 指定名称，-m 简述更新。
3. 将返回的试玩链接与二维码反馈给我。`;

const AI_PROMPT_EN = `Build and publish this project to playtest.run:
1. Ensure CLI is installed: curl -fsSL https://playtest.run/install.sh | bash
2. Publish: run playtest <dir> for static build with index.html (add --spa for SPA), or playtest <port> for local dev server. Optional: -n "<name>", -m "<note>".
3. Report back the playable link and QR code.`;

type ModeType = "ai" | "command";

interface Mode {
  id: string;
  label: string;
  type: ModeType;
  command?: string;
  hint: string;
  hintEn?: string;
}

const modes: Mode[] = [
  {
    id: "ai",
    label: "AI Prompt",
    type: "ai",
    hint: "把提示词发给 Cursor、Claude Code 等 AI 助手，它会自动完成构建与发布。",
    hintEn: "Paste this prompt into Cursor, Claude Code, or Windsurf to publish autonomously.",
  },
  {
    id: "static",
    label: "Export Folder",
    type: "command",
    command: "playtest ./dist",
    hint: "Build your project first, then replace ./dist with your build output folder.",
  },
  {
    id: "local",
    label: "Local Port",
    type: "command",
    command: "playtest 3000",
    hint: "Start your local dev server, and keep your terminal open during sharing.",
  },
  {
    id: "install",
    label: "Install CLI",
    type: "command",
    command: "curl -fsSL https://playtest.run/install.sh | bash",
    hint: "macOS & Linux · Installs to ~/.local/bin",
  },
];

export function Publish() {
  const dialog = useRef<HTMLDialogElement>(null);
  const copyGeneration = useRef(0);
  const [mode, setMode] = useState(0);
  const [lang, setLang] = useState<"zh" | "en">("zh");
  const [message, setMessage] = useState("");
  const [copied, setCopied] = useState(false);

  const selected = modes[mode];
  const activePrompt = lang === "zh" ? AI_PROMPT_ZH : AI_PROMPT_EN;
  const copyContent = selected.type === "ai" ? activePrompt : (selected.command ?? "");

  function resetCopy() {
    copyGeneration.current++;
    setMessage("");
    setCopied(false);
  }

  async function copy() {
    const generation = copyGeneration.current;
    try {
      await navigator.clipboard.writeText(copyContent);
      if (generation !== copyGeneration.current) return;
      setCopied(true);
      setMessage(selected.type === "ai" ? "Prompt copied." : "Command copied.");
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
            {modes.map((item, index) => (
              <button
                type="button"
                key={item.label}
                class={item.type === "ai" ? "tab-ai" : ""}
                aria-pressed={mode === index}
                onClick={() => { setMode(index); resetCopy(); }}
              >
                {item.type === "ai" && (
                  <svg class="icon tab-spark" viewBox="0 0 24 24" aria-hidden="true">
                    <path d="m12 3 1.9 4.9a3.5 3.5 0 0 0 2.2 2.2L21 12l-4.9 1.9a3.5 3.5 0 0 0-2.2 2.2L12 21l-1.9-4.9a3.5 3.5 0 0 0-2.2-2.2L3 12l4.9-1.9a3.5 3.5 0 0 0 2.2-2.2z" />
                  </svg>
                )}
                {item.label}
              </button>
            ))}
          </div>
          <div class="cli">
            <div class="cli-bar">
              <div class="cli-info">
                <span class="cli-dots" aria-hidden="true"></span>
                {selected.type === "ai" && (
                  <div class="prompt-lang-toggle" role="group" aria-label="Prompt language">
                    <button
                      type="button"
                      class={`lang-btn ${lang === "zh" ? "active" : ""}`}
                      onClick={() => { setLang("zh"); resetCopy(); }}
                    >
                      中文
                    </button>
                    <button
                      type="button"
                      class={`lang-btn ${lang === "en" ? "active" : ""}`}
                      onClick={() => { setLang("en"); resetCopy(); }}
                    >
                      EN
                    </button>
                  </div>
                )}
              </div>
              <button
                class="copy-command"
                type="button"
                aria-label={copied ? (selected.type === "ai" ? "Prompt copied" : "Command copied") : (selected.type === "ai" ? "Copy prompt" : "Copy command")}
                title={copied ? "Copied" : (selected.type === "ai" ? "Copy prompt" : "Copy command")}
                data-copied={copied || undefined}
                onClick={copy}
              >
                <svg class="icon copy-glyph" viewBox="0 0 24 24" aria-hidden="true"><rect x="8" y="8" width="12" height="12" rx="2" /><path d="M16 8V5a1 1 0 0 0-1-1H5a1 1 0 0 0-1 1v10a1 1 0 0 0 1 1h3" /></svg>
                <svg class="icon copied-glyph" viewBox="0 0 24 24" aria-hidden="true"><path d="m5 12 4 4L19 6" /></svg>
                <span class="copy-text">{copied ? "Copied" : (selected.type === "ai" ? "Copy Prompt" : "Copy")}</span>
              </button>
            </div>
            {selected.type === "ai" ? (
              <div class="codebox ai-box selected" tabIndex={0} role="region" aria-label="AI Prompt">
                <code>{activePrompt}</code>
              </div>
            ) : (
              <div class="codebox selected" tabIndex={0} role="region" aria-label="Publish command">
                <b aria-hidden="true">$</b><code>{selected.command}</code>
              </div>
            )}
            <p class="leg selected">
              <svg class="icon" viewBox="0 0 24 24" aria-hidden="true"><circle cx="12" cy="12" r="10" /><path d="M12 16v-4m0-4h.01" /></svg>
              <span>{selected.type === "ai" ? (lang === "zh" ? selected.hint : selected.hintEn) : selected.hint}</span>
            </p>
          </div>
          <p class="pub-status" role="status" data-state={copied ? "success" : "error"}>{message}</p>
        </div>
        <div class="pub-foot">
          <a href={href({ name: "docs", section: "publish" })} onClick={() => dialog.current?.close()}>
            <svg class="icon" viewBox="0 0 24 24" aria-hidden="true"><path d="M4 19.5A2.5 2.5 0 0 1 6.5 17H20M4 4.5A2.5 2.5 0 0 1 6.5 2H20v20H6.5A2.5 2.5 0 0 1 4 19.5v-15Z" /></svg>
            Documentation
          </a>
        </div>
      </div>
    </dialog>
  </>;
}

import { createContext, type ComponentChildren } from "preact";
import { useContext, useEffect, useRef, useState } from "preact/hooks";
import { DOC_SECTIONS, href, type DocSection } from "../router";

export type Lang = "zh" | "en";
const LangContext = createContext<Lang>("zh");

function initialLang(): Lang {
  try {
    const saved = localStorage.getItem("playtest_docs_lang");
    if (saved === "zh" || saved === "en") return saved;
  } catch {}
  return typeof navigator !== "undefined" && navigator.language?.toLowerCase().startsWith("zh") ? "zh" : "en";
}

const chapters: Record<DocSection, { zh: { title: string; description: string }; en: { title: string; description: string } }> = {
  start: {
    zh: { title: "快速开始", description: "从一个可玩的构建，到第一个发得出去的链接。" },
    en: { title: "Quickstart", description: "From a playable build to your first shareable link." },
  },
  publish: {
    zh: { title: "发布与更新", description: "根据技术栈选对模式，多次迭代保持同一个玩家链接。" },
    en: { title: "Publish & Update", description: "Choose the right mode and keep the same link across iterations." },
  },
  share: {
    zh: { title: "分享与招募", description: "私下发给朋友，或直接在广场面向社区招募玩家。" },
    en: { title: "Share & Recruit", description: "Send to friends or let the community discover your work." },
  },
  manage: {
    zh: { title: "管理作品", description: "打开、查看与回滚，规则清晰，绝不猜错目标。" },
    en: { title: "Manage Works", description: "Open, inspect, and rollback with clear target resolution." },
  },
  automation: {
    zh: { title: "脚本与 AI", description: "接入 CI/CD 自动化发布管线，或连接 AI 辅助编码工具。" },
    en: { title: "Scripts & AI", description: "Integrate publishing into your CI/CD pipelines and AI workflows." },
  },
  troubleshoot: {
    zh: { title: "常见排查与 FAQ", description: "快速定位异常现象，走最短路径恢复可用。" },
    en: { title: "Troubleshooting", description: "Identify issues fast and take the shortest recovery path." },
  },
};

export function CommandBlock({ command, label = "Terminal", lang = "en" }: { command: string; label?: string; lang?: Lang }) {
  const [copied, setCopied] = useState(false);
  const [errorMsg, setErrorMsg] = useState("");
  const timer = useRef<ReturnType<typeof setTimeout>>();

  useEffect(() => () => clearTimeout(timer.current), []);

  async function copy() {
    clearTimeout(timer.current);
    try {
      await navigator.clipboard.writeText(command);
      setCopied(true);
      setErrorMsg("");
      timer.current = setTimeout(() => setCopied(false), 2400);
    } catch {
      setErrorMsg(lang === "zh" ? "未能自动复制，请手动选中文本复制。" : "Unable to copy automatically, please select and copy manually.");
      timer.current = setTimeout(() => setErrorMsg(""), 3500);
    }
  }

  const lines = command.split("\n");

  return (
    <div class="doc-code">
      <div class="doc-code-bar">
        <div class="doc-code-lead">
          <div class="doc-mac-dots" aria-hidden="true">
            <span class="doc-mac-dot" />
            <span class="doc-mac-dot" />
            <span class="doc-mac-dot" />
          </div>
          <span class="doc-code-title">{label}</span>
        </div>
        <button
          type="button"
          class={`doc-copy-btn ${copied ? "copied" : ""}`}
          onClick={copy}
          aria-label={`${lang === "zh" ? "复制命令" : "Copy command"}: ${command}`}
        >
          {copied ? (
            <>
              <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
                <polyline points="20 6 9 17 4 12" />
              </svg>
              <span>{lang === "zh" ? "已复制" : "Copied"}</span>
            </>
          ) : (
            <>
              <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
                <rect x="9" y="9" width="13" height="13" rx="2" ry="2" />
                <path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1" />
              </svg>
              <span>{lang === "zh" ? "复制" : "Copy"}</span>
            </>
          )}
        </button>
      </div>
      <pre tabIndex={0} aria-label={label}>
        <code>
          {lines.map((line, idx) => (
            <span key={idx} class="doc-code-line">
              <span class="doc-prompt" aria-hidden="true">$</span>
              {line}
            </span>
          ))}
        </code>
      </pre>
      {errorMsg ? <span class="doc-copy-status" role="status">{errorMsg}</span> : null}
    </div>
  );
}

function Note({ title, lang = "en", children }: { title: ComponentChildren; lang?: Lang; children: ComponentChildren }) {
  return (
    <aside class="doc-note">
      <div class="doc-note-tag" aria-hidden="true">
        <svg width="11" height="11" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round">
          <circle cx="12" cy="12" r="10" />
          <line x1="12" y1="16" x2="12" y2="12" />
          <line x1="12" y1="8" x2="12.01" y2="8" />
        </svg>
        <span>{lang === "zh" ? "关键原则" : "Key Principle"}</span>
      </div>
      <strong>{title}</strong>
      <div class="doc-note-body">{children}</div>
    </aside>
  );
}

function Chapter({ name, children }: { name: DocSection; children: ComponentChildren }) {
  const lang = useContext(LangContext);
  const number = String(DOC_SECTIONS.indexOf(name) + 1).padStart(2, "0");
  return (
    <section class="doc-section" id={`doc-${name}`} aria-labelledby={`doc-title-${name}`}>
      <header class="doc-section-head">
        <span class="doc-number">{number}</span>
        <div>
          <h2 id={`doc-title-${name}`} tabIndex={-1}>{chapters[name][lang].title}</h2>
          <p>{chapters[name][lang].description}</p>
        </div>
      </header>
      {children}
    </section>
  );
}

export function DocsPage({ section }: { section: DocSection }) {
  const [active, setActive] = useState<DocSection>(section);
  const [lang, setLang] = useState<Lang>(initialLang);
  const mobileMenu = useRef<HTMLDetailsElement>(null);

  function toggleLang(next: Lang) {
    setLang(next);
    try {
      localStorage.setItem("playtest_docs_lang", next);
    } catch {}
  }

  const t = <T,>(zh: T, en: T): T => (lang === "zh" ? zh : en);

  function locate(name: DocSection) {
    mobileMenu.current?.removeAttribute("open");
    if (name === "start") window.scrollTo({ top: 0, behavior: "instant" });
    else document.getElementById(`doc-${name}`)?.scrollIntoView({ block: "start", behavior: "instant" });
    document.getElementById(name === "start" ? "docs-title" : `doc-title-${name}`)?.focus({ preventScroll: true });
    setActive(name);
  }

  useEffect(() => {
    const frame = requestAnimationFrame(() => locate(section));
    return () => cancelAnimationFrame(frame);
  }, [section]);

  useEffect(() => {
    const previousTitle = document.title;
    document.title = lang === "zh" ? "使用文档 · playtest" : "Documentation · playtest";
    const page = document.querySelector<HTMLElement>(".docs-page");
    const rail = document.querySelector<HTMLElement>(".sidebar");
    const measure = () => page?.style.setProperty("--docs-top", `${matchMedia("(max-width: 767px)").matches ? rail?.getBoundingClientRect().height ?? 0 : 0}px`);
    const observer = new ResizeObserver(measure);
    if (rail) observer.observe(rail);
    measure();
    let pending = 0;
    const update = () => {
      cancelAnimationFrame(pending);
      pending = requestAnimationFrame(() => {
        let current: DocSection = "start";
        for (const name of DOC_SECTIONS) {
          const threshold = 180 + (matchMedia("(max-width: 767px)").matches ? rail?.getBoundingClientRect().height ?? 0 : 0);
          if ((document.getElementById(`doc-${name}`)?.getBoundingClientRect().top ?? Infinity) <= threshold) current = name;
        }
        setActive(current);
      });
    };
    addEventListener("scroll", update, { passive: true });
    return () => { observer.disconnect(); document.title = previousTitle; removeEventListener("scroll", update); cancelAnimationFrame(pending); };
  }, [lang]);

  const links = DOC_SECTIONS.map((name, index) => (
    <a
      key={name}
      href={href({ name: "docs", section: name })}
      aria-current={active === name ? "location" : undefined}
      onClick={() => locate(name)}
    >
      <span>{String(index + 1).padStart(2, "0")}</span>
      {chapters[name][lang].title}
    </a>
  ));

  return (
    <LangContext.Provider value={lang}>
      <div class="docs-page">
      <header class="docs-hero">
        <div class="docs-hero-top">
          <div class="docs-eyebrow">{t("PLAYTEST · 使用指南", "PLAYTEST · USER GUIDE")}</div>
          <div class="docs-lang-switch" role="group" aria-label={t("选择语言", "Select language")}>
            <button
              type="button"
              class={`docs-lang-btn ${lang === "zh" ? "active" : ""}`}
              onClick={() => toggleLang("zh")}
              aria-pressed={lang === "zh"}
            >
              中文
            </button>
            <span class="docs-lang-divider" aria-hidden="true">/</span>
            <button
              type="button"
              class={`docs-lang-btn ${lang === "en" ? "active" : ""}`}
              onClick={() => toggleLang("en")}
              aria-pressed={lang === "en"}
            >
              EN
            </button>
          </div>
        </div>

        <h1 id="docs-title" tabIndex={-1}>
          {t("从做完可玩，到真的有人玩。", "From playable, to played.")}
        </h1>
        <p>
          {t(
            "把打包目录或本地端口交给 playtest，立刻拿到发得出去的链接。零门槛发布、极简玩家招募，为你下一版带来真实反馈。",
            "Hand your build directory or local port to playtest, get an instant shareable link. Frictionless publishing, effortless playtester recruitment, and real feedback for your next version."
          )}
        </p>
        <div class="docs-hero-meta">
          <span>{t("首次发布免注册", "No sign-up for first publish")}</span>
          <i aria-hidden="true" />
          <span>{t("玩家点开即玩无门槛", "Zero-friction browser play")}</span>
          <i aria-hidden="true" />
          <span>{t("同一链接哈希增量更新", "Incremental hash updates on the same link")}</span>
        </div>
      </header>

      <details class="docs-mobile-menu" ref={mobileMenu}>
        <summary>
          <span>{t("当前章节：", "Current Chapter: ")}<b>{chapters[active][lang].title}</b></span>
          <span aria-hidden="true">{t("目录 ⌄", "Table of Contents ⌄")}</span>
        </summary>
        <div class="docs-mobile-menu-toolbar">
          <span class="docs-mobile-lang-label">{t("文档语言 / Language", "Documentation Language")}</span>
          <div class="docs-lang-switch" role="group">
            <button
              type="button"
              class={`docs-lang-btn ${lang === "zh" ? "active" : ""}`}
              onClick={() => toggleLang("zh")}
            >
              中文
            </button>
            <span class="docs-lang-divider" aria-hidden="true">/</span>
            <button
              type="button"
              class={`docs-lang-btn ${lang === "en" ? "active" : ""}`}
              onClick={() => toggleLang("en")}
            >
              EN
            </button>
          </div>
        </div>
        <nav aria-label={t("文档章节（手机端）", "Documentation chapters (mobile)")}>{links}</nav>
      </details>

      <div class="docs-layout">
        <article class="docs-article" aria-label={t("playtest 使用指南", "playtest user guide")}>
          <Chapter name="start">
            <div class="doc-start-command">
              <span class="doc-kicker">{t("单条命令快速开始", "Quickstart: Publish with a single command")}</span>
              <CommandBlock command="playtest ./dist" label={t("终端命令", "Terminal")} lang={lang} />
              <p>
                {t(
                  "把 ./dist 换成你的 Web 导出目录。完成后复制链接发给朋友，或用手机扫描终端打印的二维码立即实机试玩。",
                  "Replace ./dist with your exported web build directory. Once completed, copy the shareable link or scan the terminal QR code with your phone to test immediately."
                )}
              </p>
            </div>

            <div class="doc-step">
              <div class="doc-step-head">
                <span class="doc-step-badge">1</span>
                <h3>{t("安装 CLI", "Install CLI")}</h3>
              </div>
              <p>{t("一行命令安装（macOS 与 Linux）：", "Install via one-line command (macOS & Linux):")}</p>
              <CommandBlock command="curl -fsSL https://playtest.run/install.sh | bash" label={t("一键安装 (macOS & Linux)", "One-line install (macOS & Linux)")} lang={lang} />
              <p>
                {t(
                  "或前往 GitHub Releases ↗ 为你的操作系统（macOS、Linux、Windows playtest.exe）直接下载编译好的独立二进制：",
                  "Or download pre-built binaries manually for your OS (macOS, Linux, Windows playtest.exe) from "
                )}
                <a href="https://github.com/roviix/playtest.run/releases" target="_blank" rel="noreferrer">GitHub Releases ↗</a>。
              </p>
              <p>{t("打开终端验证安装：", "Open your terminal and verify the installation:")}</p>
              <CommandBlock command="playtest --version" label={t("检查版本", "Verify version")} lang={lang} />
              <p>
                {t(
                  "独立单二进制文件，零运行时依赖，开箱即用。若终端提示找不到命令，请检查该文件所在路径是否已加入系统的 PATH 环境变量。",
                  "Standalone binary, works out of the box with zero runtime dependencies. If your terminal cannot find the command, check that the binary's directory is included in your PATH environment variable."
                )}
              </p>
            </div>

            <div class="doc-step">
              <div class="doc-step-head">
                <span class="doc-step-badge">2</span>
                <h3>{t("准备 Web 导出产物", "Prepare Web Build Artifacts")}</h3>
              </div>
              <p>
                {t(
                  "先在你的项目里执行打包构建（如 npm run build），或使用游戏引擎（Godot / Unity / Cocos / Phaser）的 HTML5/Web 导出功能，获得一个包含 index.html 的导出目录。",
                  "First run your project's build command (e.g. npm run build) or use your game engine's (Godot / Unity / Cocos / Phaser) Web export feature. You should obtain an output directory containing index.html."
                )}
              </p>
              <Note title={t("请上传构建导出物，不要传源码仓库", "Upload build exports, not source repository")} lang={lang}>
                <p>
                  {t(
                    "传入 dist 或 build 等产物目录。CLI 专职负责分发，不代跑项目构建；扫描时不读取 .gitignore，以「.」开头的隐藏文件及软链接会自动跳过。",
                    "Specify export folders like dist or build. The CLI focuses on distribution and will not execute project build scripts. Scans do not filter by .gitignore; dotfiles and symlinks are automatically skipped."
                  )}
                </p>
              </Note>
            </div>

            <div class="doc-step">
              <div class="doc-step-head">
                <span class="doc-step-badge">3</span>
                <h3>{t("发布并获取分享链接", "Publish & Get Share Link")}</h3>
              </div>
              <p>
                {t(
                  "首次发布时，CLI 会自动分配匿名凭据，并在终端打印作品名、版本号、分享链接、终端二维码及结果查看入口：",
                  "On your first publish, the CLI automatically provisions anonymous credentials and prints the work name, version, share URL, terminal QR code, and results entry point:"
                )}
              </p>
              <div class="doc-output">
                <div class="doc-output-head"><span aria-hidden="true" />{t("终端输出示例", "Terminal Output Example")}</div>
                <pre>
                  {t(
                    '已发布 "Tiny Planet" v1\nhttps://playtest.run/p/brisk-otter-41\n\n[终端二维码]\n匿名链接到期时间：见终端打印\n查看试玩结果：见终端打印的控制台地址',
                    'Published "Tiny Planet" v1\nhttps://playtest.run/p/brisk-otter-41\n\n[Terminal QR Code]\nAnonymous link expires at scheduled time\nView playtest results: console URL printed in terminal'
                  )}
                </pre>
              </div>
              <p>
                {t(
                  "默认发布不写图片，也不会把作品挂到公开广场。匿名链接通常保留 24 小时（以终端提示为准），适合即时发给朋友或在手机真机验证。",
                  "Default publish does not save an image or list on the public Plaza. Anonymous links typically remain valid for 24 hours (refer to terminal prompt), ideal for instant friend sharing or mobile device testing."
                )}
              </p>
            </div>

            <h3>{t("随时关联账号以长期保留作品", "Link an Account Anytime to Preserve Works")}</h3>
            <p>
              {t(
                "当你需要跨设备管理或长期保留作品时，在终端运行 playtest login。浏览器打开提示的页面，通过邮箱或 GitHub 登录、核对验证码并确认授权即可。该设备上所有未过期的匿名作品会自动绑定到你的账号下。",
                "When you need to preserve works long-term or manage across devices, run playtest login in your terminal. Open the prompted playtest URL, sign in with email or GitHub, verify the code and authorize. Any unexpired anonymous works on this device will be linked to your account."
              )}
            </p>
            <p>
              {t(
                "浏览器、CLI、关注与发布使用同一个账号体系，各自独立凭据。浏览器退出登录不会让 CLI 下线。自动化调用令牌可在账号设置中随时创建与吊销——切勿将令牌泄漏给玩家。",
                "Browser, CLI, following, and publishing share one account with separate credentials. Signing out of the browser will not log out the CLI. Automation access tokens can be created or revoked in Account settings—never share them with players."
              )}
            </p>
          </Chapter>

          <Chapter name="publish">
            <h3>{t("根据技术栈选择发布模式", "Choose Publishing Mode for Your Stack")}</h3>
            <p>
              {t(
                "playtest 原生支持静态目录托管、本地端口隧道及前后端同源混合部署：",
                "playtest natively supports static directory hosting, local port tunneling, and hybrid front-end/back-end deployments:"
              )}
            </p>
            <div class="doc-table-wrap">
              <table>
                <thead>
                  <tr>
                    <th>{t("产物形态", "Artifact Form")}</th>
                    <th>{t("推荐命令", "Recommended Command")}</th>
                    <th>{t("终端关闭后的表现", "Behavior on Terminal Close")}</th>
                  </tr>
                </thead>
                <tbody>
                  <tr>
                    <td>{t("构建好的 Web 目录 (HTML5 / Web)", "Built web directory (HTML5 / Web)")}</td>
                    <td><code>playtest ./dist</code></td>
                    <td>{t("静态作品持续在线，浏览器秒级直出", "Static work stays live, accessible directly in browser")}</td>
                  </tr>
                  <tr>
                    <td>{t("正在运行的本地开发服务", "Running local dev server")}</td>
                    <td><code>playtest 5173</code></td>
                    <td>{t("隧道断开，实时服务停止", "Tunnel disconnects, live service stops")}</td>
                  </tr>
                  <tr>
                    <td>{t("静态前端 + 本地 API 服务", "Static frontend + local API")}</td>
                    <td><code>playtest ./dist --backend 3000</code></td>
                    <td>{t("静态网页持续在线；本地后端接口返回 503", "Static pages stay live; backend API returns 503")}</td>
                  </tr>
                </tbody>
              </table>
            </div>
            <p>
              {t(
                "在分享端口前，请确保本地开发服务已正常启动并在监听对应端口。CLI 会自动检查端口、建立加密隧道并在前台保持运行。遇到网络抖动会自动重连，按 Ctrl-C 随时退出。",
                "Ensure your local server is running before sharing ports. The CLI checks the port, connects to the tunnel, and keeps running in foreground. Automatically reconnects on network interruptions; press Ctrl-C to exit anytime."
              )}
            </p>

            <h3>{t("版本迭代：保持同一个玩家链接", "Version Iteration: Keep the Same Link")}</h3>
            <CommandBlock command={'playtest ./dist -m "Fixed mobile controls, please test again"'} label={t("发布新版本", "Publish new version")} lang={lang} />
            <p>
              {t(
                "在同一个目录下再次运行发布命令，CLI 会比对文件哈希，仅增量上传修改过的文件。版本号自动自增（v1 → v2），分享链接始终不变——玩家点开永远是最新版。",
                "Running the publish command again in the same directory hashes file contents and only uploads modified files incrementally. The version increments automatically (v1 → v2), and the shareable link remains identical—players always see the latest build."
              )}
            </p>
            <CommandBlock command="playtest ./dist --to brisk-otter-41" label={t("从新文件夹更新指定作品", "Update specific work from a new folder")} lang={lang} />
            <p>
              {t(
                "如果更换了输出目录或在新的机器构建，可传 --to <slug> 指定要更新的远程作品。若想完全另起炉灶创建一个新链接，传 --to new。",
                "If you changed output folders or build environments, pass --to <slug> to specify the target work. If you intend to create a brand new work with a fresh link, use --to new."
              )}
            </p>
            <Note title={t("认当前目录，绝不乱猜", "Current directory, no guessing")} lang={lang}>
              <p>
                {t(
                  "运行 playtest ./dist 之后，用 playtest open ./dist 查看，或 cd dist 后运行 playtest open。CLI 绝不跨层级乱猜未指定的作品或子目录。",
                  "After running playtest ./dist, use playtest open ./dist, or cd dist and run playtest open. The CLI never guesses subdirectories or recently touched works."
                )}
              </p>
            </Note>

            <h3>{t("前后端混合与 SPA 路由", "Hybrid Front-End / Back-End & SPA Routing")}</h3>
            <CommandBlock command="playtest ./dist --backend 3000 --spa" label={t("静态前端 + 本地后端隧道", "Static frontend + local backend tunnel")} lang={lang} />
            <p>
              {t(
                "静态文件与目录索引优先匹配。加了 --spa 后，未命中的页面导航请求会自动回退到 index.html（静态资源文件不会回退）。未匹配的 API 请求与 WebSocket 流量自动隧道转发到本地 3000 端口；若本地后端掉线，API 接口返回 503。",
                "Static files and directory indexes match first. With --spa enabled, unmatched navigation requests fall back to index.html (assets do not fall back). Unmatched API routes and WebSocket requests tunnel to local port 3000. If the local backend goes offline, API routes return 503."
              )}
            </p>

            <details class="doc-details">
              <summary>{t("高级发布参数速查", "Advanced Publish Parameters Quick Reference")}</summary>
              <div class="doc-details-content">
                <dl class="doc-definitions">
                  <dt><code>--isolated auto|on|off</code></dt>
                  <dd>{t("跨源隔离开关。默认 auto，探测到 WebAssembly / SharedArrayBuffer 多线程时自动启用隔离响应头；普通网页无需额外配置。", "Cross-Origin Isolation. Defaults to auto, enabling WebAssembly / SharedArrayBuffer multithreading headers when detected; ordinary web pages need no extra config.")}</dd>
                  <dt><code>--spa</code></dt>
                  <dd>{t("单页应用导航回退。HTML 页面未命中时返回 index.html，缺失的静态资源不回退。", "Single-page application navigation fallback for HTML routes. Missing assets will not fall back to index.html.")}</dd>
                  <dt><code>-y / --yes</code></dt>
                  <dd>{t("跳过发布前检查（如缺少 index.html 警告），但不跳过文件大小与账号配额限制。", "Bypass build checks (e.g. missing index.html warning) during publish, without bypassing file size or account quota limits.")}</dd>
                  <dt><code>--no-qr</code></dt>
                  <dd>{t("关闭终端中的 ASCII 二维码打印，不影响 URL 与卡片生成。", "Suppress ASCII QR code art in terminal output; URLs and generated cards remain unaffected.")}</dd>
                </dl>
                <p>
                  {t(
                    "端口分享专用于实时联调，不接收版本说明、封面、摘要、广场招募或 SPA 参数。纯数字会被解析为端口号（1–65535）；若要发布名为纯数字的目录，请使用 ./5173。",
                    "Port sharing is designed for real-time debugging and does not accept version notes, covers, summaries, Plaza recruiting, or SPA flags. Bare numbers represent ports (1–65535); to publish a directory named with numbers, pass ./5173."
                  )}
                </p>
              </div>
            </details>
          </Chapter>

          <Chapter name="share">
            <h3>{t("双域架构：邀请与试玩解耦", "Dual-Domain Architecture: Separation of Invitation & Play")}</h3>
            <p>
              {t(
                "playtest 采用严格的双域隔离架构，确保沙箱安全与最佳玩家体验：主域负责邀请、元信息、名额招募与反馈收集；独立子域运行不可信的游戏逻辑：",
                "playtest uses a strict dual-domain architecture to ensure sandbox security and optimal player UX: root domain serves invitations, metadata, seats recruitment, and feedback; isolated subdomains run untrusted game logic:"
              )}
            </p>
            <div class="doc-addresses">
              <div class="doc-address-card">
                <span>{t("玩家邀请卡（主域）", "Player Invitation Card (Root Domain)")}</span>
                <code>https://playtest.run/p/&lt;slug&gt;</code>
              </div>
              <div class="doc-address-card">
                <span>{t("独立运行沙箱（子域）", "Isolated Game Sandbox (Subdomain)")}</span>
                <code>https://&lt;slug&gt;.playtest.run</code>
              </div>
            </div>

            <h3>{t("下载高清分享邀请卡", "Download High-Resolution Share Cards")}</h3>
            <CommandBlock command="playtest card ./dist --out ./invite.png" label={t("单独下载邀请卡图片", "Download invite card separately")} lang={lang} />
            <p>
              {t(
                "默认发布不写图片（Default publish does not save an image），保持终端操作轻快。发到社交平台、社群或论坛前，运行 playtest card 即可生成 PNG 邀请卡，或在发布时加上 --card ./invite.png。玩家扫码即可直达试玩。",
                "Default publish does not save an image. This keeps terminal workflows fast and lightweight. When sharing to social media, chat groups, or forums, run playtest card to generate a PNG invite card, or add --card ./invite.png to your publish command. Players scan to play directly."
              )}
            </p>

            <h3>{t("发布到广场并招募测试者", "Publish to Plaza & Recruit Playtesters")}</h3>
            <CommandBlock command="playtest ./dist --seats 10" label={t("发布到广场招募 10 位测试者", "Publish to Plaza recruiting 10 playtesters")} lang={lang} />
            <p>
              {t(
                "传入 --seats 10 会自动把作品挂上广场并开放 10 个测试名额，无需额外写 --public。若想在广场展示但不限名额，直接加 --public。",
                "Passing --seats 10 automatically lists your work on the Plaza with 10 recruitment seats, without needing --public. If you want to list on the Plaza without limiting seats, pass --public directly."
              )}
            </p>
            <Note title={t("不上广场 ≠ 私密访问", "Unlisted ≠ Private access")} lang={lang}>
              <p>
                {t(
                  "不上广场仅代表作品不出现在公开信息流中；持有完整链接的任何人依然能够直接访问试玩。如需从广场撤下，请运行 playtest unlist ./dist，这并非密码保护。",
                  "Unlisted simply means the work will not appear in the public Plaza feed. Anyone with the direct link can still open and test the build. To remove a work from the Plaza, run playtest unlist ./dist; this is not password protection."
                )}
              </p>
            </Note>

            <details class="doc-details">
              <summary>{t("展示元信息配置选项（自动持久化）", "Showcase Metadata Options (Persisted Automatically)")}</summary>
              <div class="doc-details-content">
                <dl class="doc-definitions">
                  <dt><code>-n / --name</code></dt>
                  <dd>{t("玩家看到的展示名称。默认取目录名，下次发布可随时更新。", "Display name seen by players. Defaults to directory name, update anytime on next publish.")}</dd>
                  <dt><code>--summary</code></dt>
                  <dd>{t("作品长期摘要，140 字以内。保存一次后后续发布无需重复传入。", "Long-term work summary, up to 140 characters. Saved once, no need to re-enter on every publish.")}</dd>
                  <dt><code>-m / --note</code></dt>
                  <dd>{t("这一版更新了什么、希望大家重点测什么，280 字以内。显示在邀请页面与更新通知中。", "What's new in this version and what to focus on testing, up to 280 characters. Shown on door page and notifications.")}</dd>
                  <dt><code>--cover</code></dt>
                  <dd>{t("封面图片路径（PNG / JPEG / WebP，2 MB 以内）。更新时不传则沿用已有封面。", "Cover image path (PNG / JPEG / WebP, under 2 MB). Reuses existing cover on updates if omitted.")}</dd>
                  <dt><code>--community</code></dt>
                  <dd>{t("创作者社群链接（Discord、QQ 群、微信群加群链接等），展示在邀请页底部。", "Developer community link (Discord, Telegram, WeChat/QQ group URL), displayed at the bottom of the invitation page.")}</dd>
                </dl>
              </div>
            </details>
          </Chapter>

          <Chapter name="manage">
            <p>
              {t(
                "管理命令既可接收本地发布目录，也可直接接收远程作品 slug。只读命令与卡片生成会自动推断目标；删除、撤下与回滚必须显式指定。",
                "Management commands accept either a local publish directory or a remote work slug. Read-only and card commands infer targets automatically; delete, unlist, and rollback require explicit targets."
              )}
            </p>
            <div class="doc-table-wrap">
              <table>
                <thead>
                  <tr>
                    <th>{t("命令", "Command")}</th>
                    <th>{t("用途", "Purpose")}</th>
                  </tr>
                </thead>
                <tbody>
                  <tr><td><code>whoami</code></td><td>{t("查看当前登录身份、配额与本机记录过的作品", "View current login identity, quotas, and locally recorded works")}</td></tr>
                  <tr><td><code>ls</code></td><td>{t("列出当前身份下的所有作品；未登录时列出本机记录", "List all works under current identity; lists local works if unauthenticated")}</td></tr>
                  <tr><td><code>open [target]</code></td><td>{t("打印分享链接并在系统默认浏览器中打开邀请页", "Print share link and open invitation page in system default browser")}</td></tr>
                  <tr><td><code>files [target] --version v3</code></td><td>{t("查看版本物料清单、体积与哈希；不传版本为当前活跃版", "Inspect file manifest, sizes, and hashes; omit version for current active build")}</td></tr>
                  <tr><td><code>versions [target]</code></td><td>{t("查看所有已发布版本，并标出当前生效版本", "View all published versions, marking the currently active build")}</td></tr>
                  <tr><td><code>card [target] --out invite.png</code></td><td>{t("下载远程高清邀请卡并保存到本地图片", "Download remote invite card and save as local image")}</td></tr>
                </tbody>
              </table>
            </div>

            <CommandBlock command="playtest ls" label={t("列出作品", "List works")} lang={lang} />
            <CommandBlock command="playtest open ./dist" label={t("在浏览器打开邀请页", "Open invitation in browser")} lang={lang} />
            <CommandBlock command="playtest versions ./dist" label={t("查看所有版本", "View all versions")} lang={lang} />

            <h3>{t("版本秒级回滚", "Instant Version Rollback")}</h3>
            <CommandBlock command="playtest rollback ./dist v3" label={t("快速回滚", "Instant rollback")} lang={lang} />
            <p>
              {t(
                "瞬间把线上活跃版本切回历史任一版本（参数支持 3 或 v3）。由于所有历史物料与清单在服务器均完整保留，回滚不需要重新上传任何文件，秒级即刻生效。",
                "Instantly switch active build to an earlier published version (arguments accept 3 or v3). Because all historical files and manifests remain stored on the server, rollback requires zero re-uploading and takes effect immediately."
              )}
            </p>
            <p>
              <strong>{t("注意", "Note")}</strong>: {t("回滚操作立刻生效，没有第二次确认（no second confirmation）；建议先通过 playtest versions 核对目标版本号。", "Rollback executes immediately with no second confirmation; always verify target versions with playtest versions first.")}
            </p>

            <h3>{t("撤下广场 vs 彻底删除", "Unlisting vs Deleting")}</h3>
            <CommandBlock command="playtest unlist ./dist" label={t("从广场撤下（分享链接依然有效）", "Unlist from Plaza (share link remains active)")} lang={lang} />
            <CommandBlock command="playtest rm ./dist" label={t("彻底删除作品（分享链接立即失效）", "Delete work (share link becomes invalid)")} lang={lang} />
            <p>
              <strong>{t("从广场撤下 (unlist)", "Unlist (unlist)")}</strong>: {t("仅从公共广场撤出，已有直达链接依然有效，历史统计数据完整保留。非常适合阶段性测试结束时使用。", "Removes the work from the public Plaza; existing direct links remain valid and historical analytics are preserved. Perfect when testing phases conclude.")}
            </p>
            <p>
              <strong>{t("彻底删除 (rm)", "Delete (rm)")}</strong>: {t("永久销毁作品及其所有版本物料，链接立即失效。终端会提示二次确认，可传 -y 跳过确认。自动化脚本必须显式确认。", "Permanently destroys the work and all its versions; links become invalid immediately. Terminal prompts for confirmation; pass -y to skip. Automated scripts must pass explicit confirmation.")}
            </p>
            <Note title={t("同一作品，本地状态不随机器走", "Same work, local state across devices")} lang={lang}>
              <p>
                {t(
                  "控制台与 CLI 共用同一个账号管理作品，但本地目录与 slug 的映射关系保存在各自电脑上。换电脑时，使用 slug 或 --to 显式指定已有作品。",
                  "Console and CLI use the same account to manage works, but local directory mappings stay on each machine. When switching computers, use the slug or --to to target existing works."
                )}
              </p>
            </Note>
          </Chapter>

          <Chapter name="automation">
            <h3>{t("机器可读输出 (--json)", "Machine-Readable Output (--json)")}</h3>
            <CommandBlock command="playtest ./dist --json" label={t("CI/CD 自动化集成", "CI/CD automated integration")} lang={lang} />
            <p>
              {t(
                "传入 --json 后，一次性命令保证向 stdout 输出且仅输出单行纯 JSON，适合 jq 或脚本直接解析。进度条与交互提示会改走 stderr。",
                "When --json is passed, one-off commands guarantee a single line of pure JSON to stdout, ideal for parsing with jq or scripts. Progress bars and human-readable hints go to stderr."
              )}
            </p>
            <div class="doc-table-wrap">
              <table>
                <thead>
                  <tr>
                    <th>{t("JSON 字段", "JSON Key Field")}</th>
                    <th>{t("含义与指引", "Description & Guidance")}</th>
                  </tr>
                </thead>
                <tbody>
                  <tr><td><code>ok / action</code></td><td>{t("执行是否成功的布尔值，以及具体执行的操作动作", "Boolean success indicator and specific action executed")}</td></tr>
                  <tr><td><code>slug / url / version</code></td><td>{t("作品唯一 slug、默认邀请 URL、当前版本号", "Unique work slug, default invitation URL, version number")}</td></tr>
                  <tr><td><code>qr_text / card_url</code></td><td>{t("终端二维码文本内容、远程高清邀请卡图片 URL", "Terminal QR code text content, remote high-res invite card URL")}</td></tr>
                  <tr><td><code>card_path</code></td><td>{t("本地保存的邀请卡绝对路径（仅在显式要求生成本地卡片时存在）", "Local absolute path of saved invite card image (only when explicitly requested)")}</td></tr>
                  <tr><td><code>expires_at / console_url</code></td><td>{t("匿名作品到期时间戳、创作者控制台地址", "Expiry timestamp for anonymous works, creator console URL")}</td></tr>
                  <tr><td><code>elapsed_ms / timings</code></td><td>{t("整体耗时毫秒数，以及扫描与上传的耗时分解", "Total milliseconds elapsed and breakdown between scan and upload")}</td></tr>
                  <tr><td><code>findings</code></td><td>{t("构建物料检查结果与提示；建议在 CI 脚本中校验", "Build check findings and warnings; recommended to verify in scripts")}</td></tr>
                </tbody>
              </table>
            </div>
            <Note title={t("长连接命令采用流式 JSON 事件", "Stream JSON events for long-running commands")} lang={lang}>
              <p>
                {t(
                  "端口分享与混合模式会按行持续输出 JSON 事件（online, players, reconnecting, stopped）。请勿将整个标准输出当作单个 JSON 对象解析。open --json 不会打开浏览器；rm --json 必须显式配合 -y。",
                  "Port sharing and hybrid mode stream JSON events line-by-line (online, players, reconnecting, stopped). Do not parse the entire stdout as a single object. open --json does not launch a browser; rm --json requires -y."
                )}
              </p>
            </Note>

            <h3>{t("连接 AI 编程助手 (MCP Server)", "Connect AI Coding Assistants (MCP Server)")}</h3>
            <CommandBlock command="playtest mcp --setup" label={t("打印 MCP 配置片段", "Print MCP configuration snippet")} lang={lang} />
            <p>
              {t(
                "在 Cursor、Claude Code、Antigravity 等编辑器中将 playtest mcp 配置为 MCP 工具，让 AI 助手直接调用 5 项内置能力：",
                "Configure playtest mcp as an MCP server in Cursor, Claude Code, Antigravity, or other editors to let AI assistants invoke 5 built-in tools directly:"
              )}
            </p>
            <p>
              <code>playtest_upload</code> {t("（部署构建产物）", "(deploy build artifacts)")}、
              <code>playtest_share</code> {t("（端口穿透）", "(port tunneling)")}、
              <code>playtest_list</code> {t("（列出作品）", "(list works)")}、
              <code>playtest_site</code> {t("（查询作品详情与反馈）", "(query work details & feedback)")}、
              <code>playtest_card</code> {t("（生成邀请卡并在对话内直接预览）", "(generate invite cards with instant preview in chat)")}。
            </p>

            <h3>{t("自定义 API 地址与自托管", "Custom API Endpoint & Self-Hosting")}</h3>
            <CommandBlock command="playtest --api http://localhost:8787 ls --json" label={t("连接自托管实例", "Connect to self-hosted instance")} lang={lang} />
            <p>
              {t(
                "API 地址解析优先级：--api 命令行参数 → PLAYTEST_API 环境变量 → 本地配置文件 → 官方默认服务。--api 与 --json 放在子命令前后均可生效。",
                "API URLs resolve in priority: --api CLI flag → PLAYTEST_API env var → local config file → official default service. --api and --json can be placed before or after subcommands."
              )}
            </p>

            <details class="doc-details">
              <summary>{t("CLI 退出码与本地配置文件", "CLI Exit Codes & Local Config File")}</summary>
              <div class="doc-details-content">
                <p>
                  <strong>{t("退出码速查", "Exit Codes Quick Reference")}</strong>: {t("0 成功 · 1 非预期失败 · 2 参数错误 · 3 认证失败 · 4 网络错误 · 5 服务端错误 · 6 产物或输入校验失败 · 7 超出配额。在 JSON 模式下，直接读取 code 与 hint 字段。", "0 Success · 1 Unexpected failure · 2 Usage error · 3 Auth failure · 4 Network error · 5 Server error · 6 Input or build error · 7 Quota exceeded. In JSON mode, read code and hint fields for details.")}
                </p>
                <p>
                  {t(
                    "本地配置文件路径：macOS / Linux 位于 ~/.config/playtest/config.json，Windows 位于 %APPDATA%\\playtest\\config.json。内含登录凭据、API 地址与本地目录映射，切勿提交至公开仓库或分享给玩家。",
                    "Local config file path: macOS / Linux at ~/.config/playtest/config.json, Windows at %APPDATA%\\playtest\\config.json. Contains login tokens, API URL, and local path mappings; never commit to public repositories or share with players."
                  )}
                </p>
              </div>
            </details>
          </Chapter>

          <Chapter name="troubleshoot">
            <div class="doc-faq">
              <details>
                <summary>{t("“这个目录还没发布过”，可我明明发布了？", "“This directory hasn't been published yet”, but I already published it?")}</summary>
                <div class="doc-faq-content">
                  <p>
                    {t(
                      "检查终端当前的工作目录。如果你是在上一层目录运行的 playtest ./dist，请使用 playtest open ./dist；或者 cd dist 后再运行 playtest open。也可以通过 playtest ls 查看当前名下的所有 slug，直接以 slug 为参数操作。CLI 从不跨层级乱猜未指定的目录。",
                      "Check your current terminal working directory. If you ran playtest ./dist from a parent directory, use playtest open ./dist; or cd dist and run playtest open. You can also run playtest ls to view all work slugs and operate by slug. The CLI never guesses unspecified targets."
                    )}
                  </p>
                </div>
              </details>

              <details>
                <summary>{t("端口没有监听，或者本地后端突然掉线？", "Port is not listening, or local backend suddenly went offline?")}</summary>
                <div class="doc-faq-content">
                  <p>
                    {t(
                      "在运行分享命令前，请确认本地服务已启动并在监听对应端口。分享过程中请保持终端常开且电脑不休眠。在混合模式下，静态页面能打开不代表后端连通；若本地服务退出，API 接口请求将返回 503。",
                      "Ensure your local server is running before sharing ports. The CLI checks the port, connects to the tunnel, and keeps running in foreground. Keep your terminal open and computer awake during sharing. In hybrid mode, static pages loading does not mean the backend is connected; if the local server exits, API requests will return 503."
                    )}
                  </p>
                </div>
              </details>

              <details>
                <summary>{t("匿名链接过期了，或者登录凭据失效？", "Anonymous link expired, or login session invalidated?")}</summary>
                <div class="doc-faq-content">
                  <p>
                    {t(
                      "匿名作品通常有 24 小时有效期。过期后重新发布会分配全新的 slug 和链接，旧链接不会被复活。如需永久保留作品，请在过期前运行 playtest login 登录此设备。随时可用 playtest whoami 查看终端认证状态。",
                      "Anonymous works typically expire after 24 hours. Re-publishing after expiration assigns a fresh slug and link; the old link will not be revived. To keep works permanently, run playtest login before expiration to authenticate this device. Check terminal auth status with playtest whoami."
                    )}
                  </p>
                </div>
              </details>

              <details>
                <summary>{t("缺少 index.html，或构建产物校验未通过？", "Missing index.html, or build artifact check failed?")}</summary>
                <div class="doc-faq-content">
                  <p>
                    {t(
                      "确保选择的目录是包含 index.html 的完整 Web 导出文件夹。CLI 不代跑构建命令。如果你的目录结构特殊但能在浏览器运行，可传 -y 跳过物料校验（此参数不跳过文件大小与配额限制）。",
                      "Ensure the chosen folder is a complete web export directory containing index.html. The CLI does not execute build steps. If your folder structure is non-standard but runs in browsers, pass -y to bypass the check (this does not bypass file size or quota limits)."
                    )}
                  </p>
                </div>
              </details>

              <details>
                <summary>{t("发布后本地怎么没有邀请卡图片文件？", "Why is there no local invite card image file after publishing?")}</summary>
                <div class="doc-faq-content">
                  <p>
                    {t(
                      "默认发布不写图片（Default publish does not save an image）。这是为了保证终端操作的轻快。如果需要下载 PNG 邀请卡，请单独运行 playtest card ./dist --out ./invite.png，或在发布时附加 --card ./invite.png。若下载失败请检查网络与目录写入权限；发布作品与保存本地图片是分开的步骤。",
                      "Default publish does not save an image (默认发布不写图片). This ensures terminal operations remain lightweight and fast. To download a PNG invite card locally, run playtest card ./dist --out ./invite.png, or pass --card ./invite.png during publish. Check network and directory write permissions if download fails; publishing the work and saving the local card are separate steps."
                    )}
                  </p>
                </div>
              </details>

              <details>
                <summary>{t("不上广场是不是等于只有我能玩？", "Does unlisting mean only I can access the build?")}</summary>
                <div class="doc-faq-content">
                  <p>
                    {t(
                      "不是。不上广场 ≠ 私密访问（Unlisted ≠ Private access）。不上广场仅代表作品不会出现在公开广场列表里；任何拿到直链的人都可以直接在浏览器中打开试玩。如需彻底注销访问权，请使用 playtest rm ./dist 删除作品；unlist 只是移出展示墙，并非设置密码。",
                      "No. Unlisted ≠ Private access (不上广场 ≠ 私密访问). Unlisting only removes the work from the public Plaza feed; anyone with the direct link can still open and play. To revoke access completely, run playtest rm ./dist to delete the work; unlist hides it from the showcase rather than securing it behind passwords."
                    )}
                  </p>
                </div>
              </details>

              <details>
                <summary>{t("以前写好的自动化脚本带 --gate 怎么报错了？", "Older automation scripts with --gate now fail with an error?")}</summary>
                <div class="doc-faq-content">
                  <p>
                    {t(
                      "--gate 这个选项已撤出（--gate option has been removed）。系统现已全面统一为双域规范：主域承载邀请与名额招募，子域承载隔离试玩，不再需要 once / always / never 模式切换。从脚本中移除 --gate 参数后重新执行即可。",
                      "--gate option has been removed (--gate 这个选项已撤出). The system now standardizes on a clean dual-domain architecture: root domain hosts invitations and playtester intake, while subdomains sandbox gameplay, eliminating the need for once / always / never mode toggles. Remove --gate from your scripts and retry."
                    )}
                  </p>
                </div>
              </details>

              <details>
                <summary>{t("我本地装的 CLI 行为和文档说的不一致？", "My installed CLI behavior differs from this documentation?")}</summary>
                <div class="doc-faq-content">
                  <p>
                    {t(
                      "在终端运行 playtest --version 和 playtest --help，核对 GitHub Releases 上的最新发布说明。本文档描述的是平台最新版本的契约；本地仓库代码修改并不会自动更新系统里已安装的二进制包。",
                      "Run playtest --version and playtest --help in your terminal and compare with GitHub Releases. This documentation describes the latest platform release conventions; local source edits do not automatically update installed binaries."
                    )}
                  </p>
                </div>
              </details>
            </div>

            <div class="doc-closing">
              <span class="doc-kicker">{t("回到最简单的起点", "Back to the simplest step")}</span>
              <h3>{t("先发给一个人。听听他们怎么说。", "Send it to one person. Hear what they say.")}</h3>
              <CommandBlock command="playtest ./dist" label={t("随时出发", "Ready to go")} lang={lang} />
              <p>{t("更多高级选项与用法示例，在终端随时运行 playtest --help 查看。", "For more options and usage examples, run playtest --help anytime.")}</p>
            </div>
          </Chapter>
        </article>

        <aside class="docs-toc">
          <div class="docs-toc-head">{t("目录", "Table of Contents")}</div>
          <nav aria-label={t("文档章节", "Documentation chapters")}>{links}</nav>
        </aside>
      </div>

      <footer class="docs-footer">
        <a href={href({ name: "docs", section: "start" })} onClick={() => locate("start")}>
          <span>{t("回到顶部 ↑", "Back to top ↑")}</span>
        </a>
      </footer>
    </div>
  </LangContext.Provider>
  );
}

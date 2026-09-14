import type { ComponentChildren } from "preact";
import { useEffect, useRef, useState } from "preact/hooks";
import { DOC_SECTIONS, href, type DocSection } from "../router";

const chapters: Record<DocSection, { title: string; description: string }> = {
  start: { title: "Quickstart", description: "From a playable build to your first shareable link." },
  publish: { title: "Publish & Update", description: "Choose the right mode and keep the same link across iterations." },
  share: { title: "Share & Recruit", description: "Send to friends or let the community discover your work." },
  manage: { title: "Manage Works", description: "Open, inspect, and rollback with clear target resolution." },
  automation: { title: "Scripts & AI", description: "Integrate publishing into your CI/CD pipelines and AI workflows." },
  troubleshoot: { title: "Troubleshooting", description: "Identify issues fast and take the shortest recovery path." },
};

export function CommandBlock({ command, label = "Terminal" }: { command: string; label?: string }) {
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
      setErrorMsg("Unable to copy automatically, please select and copy manually.");
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
          aria-label={`Copy command: ${command}`}
        >
          {copied ? (
            <>
              <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
                <polyline points="20 6 9 17 4 12" />
              </svg>
              <span>Copied</span>
            </>
          ) : (
            <>
              <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
                <rect x="9" y="9" width="13" height="13" rx="2" ry="2" />
                <path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1" />
              </svg>
              <span>Copy</span>
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

function Note({ title, children }: { title: string; children: ComponentChildren }) {
  return (
    <aside class="doc-note">
      <div class="doc-note-tag" aria-hidden="true">
        <svg width="11" height="11" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round">
          <circle cx="12" cy="12" r="10" />
          <line x1="12" y1="16" x2="12" y2="12" />
          <line x1="12" y1="8" x2="12.01" y2="8" />
        </svg>
        <span>Key Principle</span>
      </div>
      <strong>{title}</strong>
      <div class="doc-note-body">{children}</div>
    </aside>
  );
}

function Chapter({ name, children }: { name: DocSection; children: ComponentChildren }) {
  const number = String(DOC_SECTIONS.indexOf(name) + 1).padStart(2, "0");
  return (
    <section class="doc-section" id={`doc-${name}`} aria-labelledby={`doc-title-${name}`}>
      <header class="doc-section-head">
        <span class="doc-number">{number}</span>
        <div>
          <h2 id={`doc-title-${name}`} tabIndex={-1}>{chapters[name].title}</h2>
          <p>{chapters[name].description}</p>
        </div>
      </header>
      {children}
    </section>
  );
}

export function DocsPage({ section }: { section: DocSection }) {
  const [active, setActive] = useState<DocSection>(section);
  const mobileMenu = useRef<HTMLDetailsElement>(null);

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
    document.title = "Documentation · playtest";
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
  }, []);

  const links = DOC_SECTIONS.map((name, index) => (
    <a
      key={name}
      href={href({ name: "docs", section: name })}
      aria-current={active === name ? "location" : undefined}
      onClick={() => locate(name)}
    >
      <span>{String(index + 1).padStart(2, "0")}</span>
      {chapters[name].title}
    </a>
  ));

  return (
    <div class="docs-page">
      <header class="docs-hero">
        <div class="docs-eyebrow">PLAYTEST · USER GUIDE</div>
        <h1 id="docs-title" tabIndex={-1}>From playable, to played.</h1>
        <p>Hand your build directory or local port to playtest, get an instant shareable link.<br class="docs-desktop-break" />Frictionless publishing, effortless playtester recruitment, and real feedback for your next version.</p>
        <div class="docs-hero-meta">
          <span>No sign-up for first publish</span>
          <i aria-hidden="true" />
          <span>Zero-friction browser play</span>
          <i aria-hidden="true" />
          <span>Incremental hash updates on the same link</span>
        </div>
      </header>

      <details class="docs-mobile-menu" ref={mobileMenu}>
        <summary>
          <span>Current Chapter: <b>{chapters[active].title}</b></span>
          <span aria-hidden="true">Table of Contents ⌄</span>
        </summary>
        <nav aria-label="Documentation chapters (mobile)">{links}</nav>
      </details>

      <div class="docs-layout">
        <article class="docs-article" aria-label="playtest user guide">
          <Chapter name="start">
            <div class="doc-start-command">
              <span class="doc-kicker">Quickstart: Publish with a single command</span>
              <CommandBlock command="playtest ./dist" label="Terminal" />
              <p>Replace <code>./dist</code> with your exported web build directory. Once completed, copy the shareable link or scan the terminal QR code with your phone to test immediately.</p>
            </div>

            <div class="doc-step">
              <div class="doc-step-head">
                <span class="doc-step-badge">1</span>
                <h3>Install CLI</h3>
              </div>
              <p>Install via one-line command (macOS &amp; Linux):</p>
              <CommandBlock command="curl -fsSL https://playtest.run/install.sh | bash" label="One-line install (macOS &amp; Linux)" />
              <p>Or download pre-built binaries manually for your OS (macOS, Linux, Windows <code>playtest.exe</code>) from <a href="https://github.com/roviix/playtest.run/releases" target="_blank" rel="noreferrer">GitHub Releases ↗</a>.</p>
              <p>Open your terminal and verify the installation:</p>
              <CommandBlock command="playtest --version" label="Verify version" />
              <p>Standalone binary, works out of the box with zero runtime dependencies. If your terminal cannot find the command, check that the binary's directory is included in your PATH environment variable.</p>
            </div>

            <div class="doc-step">
              <div class="doc-step-head">
                <span class="doc-step-badge">2</span>
                <h3>Prepare Web Build Artifacts</h3>
              </div>
              <p>First run your project's build command (e.g. <code>npm run build</code>) or use your game engine's (Godot / Unity / Cocos / Phaser) Web export feature. You should obtain an output directory containing <code>index.html</code>.</p>
              <Note title="Upload build exports, not source repository">
                <p>Specify export folders like <code>dist</code> or <code>build</code>. The CLI focuses on distribution and will not execute project build scripts. Scans do not filter by <code>.gitignore</code>; dotfiles and symlinks are automatically skipped.</p>
              </Note>
            </div>

            <div class="doc-step">
              <div class="doc-step-head">
                <span class="doc-step-badge">3</span>
                <h3>Publish & Get Share Link</h3>
              </div>
              <p>On your first publish, the CLI automatically provisions anonymous credentials and prints the work name, version, share URL, terminal QR code, and results entry point:</p>
              <div class="doc-output">
                <div class="doc-output-head"><span aria-hidden="true" />Terminal Output Example</div>
                <pre>{'Published "Tiny Planet" v1\nhttps://playtest.run/p/brisk-otter-41\n\n[Terminal QR Code]\nAnonymous link expires at scheduled time\nView playtest results: console URL printed in terminal'}</pre>
              </div>
              <p>Default publish does not save an image or list on the public Plaza. Anonymous links typically remain valid for 24 hours (refer to terminal prompt), ideal for instant friend sharing or mobile device testing.</p>
            </div>

            <h3>Link an Account Anytime to Preserve Works</h3>
            <p>When you need to preserve works long-term or manage across devices, run <code>playtest login</code> in your terminal. Open the prompted playtest URL, sign in with email or GitHub, verify the code and authorize. Any unexpired anonymous works on this device will be linked to your account.</p>
            <p>Browser, CLI, following, and publishing share one account with separate credentials. Signing out of the browser will not log out the CLI. Automation access tokens can be created or revoked in Account settings—never share them with players.</p>
          </Chapter>

          <Chapter name="publish">
            <h3>Choose Publishing Mode for Your Stack</h3>
            <p>playtest natively supports static directory hosting, local port tunneling, and hybrid front-end/back-end deployments:</p>
            <div class="doc-table-wrap">
              <table>
                <thead>
                  <tr>
                    <th>Artifact Form</th>
                    <th>Recommended Command</th>
                    <th>Behavior on Terminal Close</th>
                  </tr>
                </thead>
                <tbody>
                  <tr>
                    <td>Built web directory (HTML5 / Web)</td>
                    <td><code>playtest ./dist</code></td>
                    <td>Static work stays live, accessible directly in browser</td>
                  </tr>
                  <tr>
                    <td>Running local dev server</td>
                    <td><code>playtest 5173</code></td>
                    <td>Tunnel disconnects, live service stops</td>
                  </tr>
                  <tr>
                    <td>Static frontend + local API</td>
                    <td><code>playtest ./dist --backend 3000</code></td>
                    <td>Static pages stay live; backend API returns 503</td>
                  </tr>
                </tbody>
              </table>
            </div>
            <p>Ensure your local server is running before sharing ports. The CLI checks the port, connects to the tunnel, and keeps running in foreground. Automatically reconnects on network interruptions; press Ctrl-C to exit anytime.</p>

            <h3>Version Iteration: Keep the Same Link</h3>
            <CommandBlock command={'playtest ./dist -m "Fixed mobile controls, please test again"'} label="Publish new version" />
            <p>Running the publish command again in the same directory hashes file contents and <strong>only uploads modified files incrementally</strong>. The version increments automatically (v1 → v2), and the shareable link remains identical—players always see the latest build.</p>
            <CommandBlock command="playtest ./dist --to brisk-otter-41" label="Update specific work from a new folder" />
            <p>If you changed output folders or build environments, pass <code>--to &lt;slug&gt;</code> to specify the target work. If you intend to create a brand new work with a fresh link, use <code>--to new</code>.</p>
            <Note title="Current directory, no guessing">
              <p>After running <code>playtest ./dist</code>, use <code>playtest open ./dist</code>, or <code>cd dist</code> and run <code>playtest open</code>. The CLI never guesses subdirectories or recently touched works.</p>
            </Note>

            <h3>Hybrid Front-End / Back-End & SPA Routing</h3>
            <CommandBlock command="playtest ./dist --backend 3000 --spa" label="Static frontend + local backend tunnel" />
            <p>Static files and directory indexes match first. With <code>--spa</code> enabled, unmatched navigation requests fall back to <code>index.html</code> (assets do not fall back). Unmatched API routes and WebSocket requests tunnel to local port 3000. If the local backend goes offline, API routes return 503.</p>

            <details class="doc-details">
              <summary>Advanced Publish Parameters Quick Reference</summary>
              <div class="doc-details-content">
                <dl class="doc-definitions">
                  <dt><code>--isolated auto|on|off</code></dt>
                  <dd>Cross-Origin Isolation. Defaults to auto, enabling WebAssembly / SharedArrayBuffer multithreading headers when detected; ordinary web pages need no extra config.</dd>
                  <dt><code>--spa</code></dt>
                  <dd>Single-page application navigation fallback for HTML routes. Missing assets will not fall back to index.html.</dd>
                  <dt><code>-y / --yes</code></dt>
                  <dd>Bypass build checks (e.g. missing index.html warning) during publish, without bypassing file size or account quota limits.</dd>
                  <dt><code>--no-qr</code></dt>
                  <dd>Suppress ASCII QR code art in terminal output; URLs and generated cards remain unaffected.</dd>
                </dl>
                <p>Port sharing is designed for real-time debugging and does not accept version notes, covers, summaries, Plaza recruiting, or SPA flags. Bare numbers represent ports (1–65535); to publish a directory named with numbers, pass <code>./5173</code>.</p>
              </div>
            </details>
          </Chapter>

          <Chapter name="share">
            <h3>Dual-Domain Architecture: Separation of Invitation & Play</h3>
            <p>playtest uses a strict dual-domain architecture to ensure sandbox security and optimal player UX: root domain serves invitations, metadata, seats recruitment, and feedback; isolated subdomains run untrusted game logic:</p>
            <div class="doc-addresses">
              <div class="doc-address-card">
                <span>Player Invitation Card (Root Domain)</span>
                <code>https://playtest.run/p/&lt;slug&gt;</code>
              </div>
              <div class="doc-address-card">
                <span>Isolated Game Sandbox (Subdomain)</span>
                <code>https://&lt;slug&gt;.playtest.run</code>
              </div>
            </div>

            <h3>Download High-Resolution Share Cards</h3>
            <CommandBlock command="playtest card ./dist --out ./invite.png" label="Download invite card separately" />
            <p>Default publish does not save an image. This keeps terminal workflows fast and lightweight. When sharing to social media, chat groups, or forums, run <code>playtest card</code> to generate a PNG invite card, or add <code>--card ./invite.png</code> to your publish command. Players scan to play directly.</p>

            <h3>Publish to Plaza & Recruit Playtesters</h3>
            <CommandBlock command="playtest ./dist --seats 10" label="Publish to Plaza recruiting 10 playtesters" />
            <p>Passing <code>--seats 10</code> automatically lists your work on the Plaza with 10 recruitment seats, without needing <code>--public</code>. If you want to list on the Plaza without limiting seats, pass <code>--public</code> directly.</p>
            <Note title="Unlisted ≠ Private access">
              <p>Unlisted simply means the work will not appear in the public Plaza feed. Anyone with the direct link can still open and test the build. To remove a work from the Plaza, run <code>playtest unlist ./dist</code>; this is not password protection.</p>
            </Note>

            <details class="doc-details">
              <summary>Showcase Metadata Options (Persisted Automatically)</summary>
              <div class="doc-details-content">
                <dl class="doc-definitions">
                  <dt><code>-n / --name</code></dt>
                  <dd>Display name seen by players. Defaults to directory name, update anytime on next publish.</dd>
                  <dt><code>--summary</code></dt>
                  <dd>Long-term work summary, up to 140 characters. Saved once, no need to re-enter on every publish.</dd>
                  <dt><code>-m / --note</code></dt>
                  <dd>What's new in this version and what to focus on testing, up to 280 characters. Shown on door page and notifications.</dd>
                  <dt><code>--cover</code></dt>
                  <dd>Cover image path (PNG / JPEG / WebP, under 2 MB). Reuses existing cover on updates if omitted.</dd>
                  <dt><code>--community</code></dt>
                  <dd>Developer community link (Discord, Telegram, WeChat/QQ group URL), displayed at the bottom of the invitation page.</dd>
                </dl>
              </div>
            </details>
          </Chapter>

          <Chapter name="manage">
            <p>Management commands accept either a local publish directory or a remote work slug. Read-only and card commands infer targets automatically; delete, unlist, and rollback require explicit targets.</p>
            <div class="doc-table-wrap">
              <table>
                <thead>
                  <tr>
                    <th>Command</th>
                    <th>Purpose</th>
                  </tr>
                </thead>
                <tbody>
                  <tr><td><code>whoami</code></td><td>View current login identity, quotas, and locally recorded works</td></tr>
                  <tr><td><code>ls</code></td><td>List all works under current identity; lists local works if unauthenticated</td></tr>
                  <tr><td><code>open [target]</code></td><td>Print share link and open invitation page in system default browser</td></tr>
                  <tr><td><code>files [target] --version v3</code></td><td>Inspect file manifest, sizes, and hashes; omit version for current active build</td></tr>
                  <tr><td><code>versions [target]</code></td><td>View all published versions, marking the currently active build</td></tr>
                  <tr><td><code>card [target] --out invite.png</code></td><td>Download remote invite card and save as local image</td></tr>
                </tbody>
              </table>
            </div>

            <CommandBlock command="playtest ls" label="List works" />
            <CommandBlock command="playtest open ./dist" label="Open invitation in browser" />
            <CommandBlock command="playtest versions ./dist" label="View all versions" />

            <h3>Instant Version Rollback</h3>
            <CommandBlock command="playtest rollback ./dist v3" label="Instant rollback" />
            <p>Instantly switch active build to an earlier published version (arguments accept <code>3</code> or <code>v3</code>). Because all historical files and manifests remain stored on the server, rollback requires zero re-uploading and takes effect immediately.</p>
            <p><strong>Note</strong>: Rollback executes immediately with no second confirmation (没有第二次确认); always verify target versions with <code>playtest versions</code> first.</p>

            <h3>Unlisting vs Deleting</h3>
            <CommandBlock command="playtest unlist ./dist" label="Unlist from Plaza (share link remains active)" />
            <CommandBlock command="playtest rm ./dist" label="Delete work (share link becomes invalid)" />
            <p><strong>Unlist (unlist)</strong>: Removes the work from the public Plaza; existing direct links remain valid and historical analytics are preserved. Perfect when testing phases conclude.</p>
            <p><strong>Delete (rm)</strong>: Permanently destroys the work and all its versions; links become invalid immediately. Terminal prompts for confirmation; pass <code>-y</code> to skip. Automated scripts must pass explicit confirmation.</p>
            <Note title="Same work, local state across devices">
              <p>Console and CLI use the same account to manage works, but local directory mappings stay on each machine. When switching computers, use the slug or <code>--to</code> to target existing works.</p>
            </Note>
          </Chapter>

          <Chapter name="automation">
            <h3>Machine-Readable Output (--json)</h3>
            <CommandBlock command="playtest ./dist --json" label="CI/CD automated integration" />
            <p>When <code>--json</code> is passed, one-off commands guarantee a single line of pure JSON to stdout, ideal for parsing with <code>jq</code> or scripts. Progress bars and human-readable hints go to stderr.</p>
            <div class="doc-table-wrap">
              <table>
                <thead>
                  <tr>
                    <th>JSON Key Field</th>
                    <th>Description & Guidance</th>
                  </tr>
                </thead>
                <tbody>
                  <tr><td><code>ok / action</code></td><td>Boolean success indicator and specific action executed</td></tr>
                  <tr><td><code>slug / url / version</code></td><td>Unique work slug, default invitation URL, version number</td></tr>
                  <tr><td><code>qr_text / card_url</code></td><td>Terminal QR code text content, remote high-res invite card URL</td></tr>
                  <tr><td><code>card_path</code></td><td>Local absolute path of saved invite card image (only when explicitly requested)</td></tr>
                  <tr><td><code>expires_at / console_url</code></td><td>Expiry timestamp for anonymous works, creator console URL</td></tr>
                  <tr><td><code>elapsed_ms / timings</code></td><td>Total milliseconds elapsed and breakdown between scan and upload</td></tr>
                  <tr><td><code>findings</code></td><td>Build check findings and warnings; recommended to verify in scripts</td></tr>
                </tbody>
              </table>
            </div>
            <Note title="Stream JSON events for long-running commands">
              <p>Port sharing and hybrid mode stream JSON events line-by-line (online, players, reconnecting, stopped). Do not parse the entire stdout as a single object. <code>open --json</code> does not launch a browser; <code>rm --json</code> requires <code>-y</code>.</p>
            </Note>

            <h3>Connect AI Coding Assistants (MCP Server)</h3>
            <CommandBlock command="playtest mcp --setup" label="Print MCP configuration snippet" />
            <p>Configure <code>playtest mcp</code> as an MCP server in Cursor, Claude Code, Antigravity, or other editors to let AI assistants invoke 5 built-in tools directly:</p>
            <p><code>playtest_upload</code> (deploy build artifacts), <code>playtest_share</code> (port tunneling), <code>playtest_list</code> (list works), <code>playtest_site</code> (query work details & feedback), <code>playtest_card</code> (generate invite cards with instant preview in chat).</p>

            <h3>Custom API Endpoint & Self-Hosting</h3>
            <CommandBlock command="playtest --api http://localhost:8787 ls --json" label="Connect to self-hosted instance" />
            <p>API URLs resolve in priority: <code>--api</code> CLI flag → <code>PLAYTEST_API</code> env var → local config file → official default service. <code>--api</code> and <code>--json</code> can be placed before or after subcommands.</p>

            <details class="doc-details">
              <summary>CLI Exit Codes & Local Config File</summary>
              <div class="doc-details-content">
                <p><strong>Exit Codes Quick Reference</strong>: 0 Success · 1 Unexpected failure · 2 Usage error · 3 Auth failure · 4 Network error · 5 Server error · 6 Input or build error · 7 Quota exceeded. In JSON mode, read <code>code</code> and <code>hint</code> fields for details.</p>
                <p>Local config file path: macOS / Linux at <code>~/.config/playtest/config.json</code>, Windows at <code>%APPDATA%\playtest\config.json</code>. Contains login tokens, API URL, and local path mappings; never commit to public repositories or share with players.</p>
              </div>
            </details>
          </Chapter>

          <Chapter name="troubleshoot">
            <div class="doc-faq">
              <details>
                <summary>“This directory hasn't been published yet”, but I already published it?</summary>
                <div class="doc-faq-content">
                  <p>Check your current terminal working directory. If you ran <code>playtest ./dist</code> from a parent directory, use <code>playtest open ./dist</code>; or <code>cd dist</code> and run <code>playtest open</code>. You can also run <code>playtest ls</code> to view all work slugs and operate by slug. The CLI never guesses unspecified targets.</p>
                </div>
              </details>

              <details>
                <summary>Port is not listening, or local backend suddenly went offline?</summary>
                <div class="doc-faq-content">
                  <p>Ensure your local service has started and is listening on the expected port before running the share command. Keep your terminal open and computer awake during sharing. In hybrid mode, static pages loading does not mean the backend is connected; if the local server exits, API requests will return 503.</p>
                </div>
              </details>

              <details>
                <summary>Anonymous link expired, or login session invalidated?</summary>
                <div class="doc-faq-content">
                  <p>Anonymous works typically expire after 24 hours. Re-publishing after expiration assigns a fresh slug and link; the old link will not be revived. To keep works permanently, run <code>playtest login</code> before expiration to authenticate this device. Check terminal auth status with <code>playtest whoami</code>.</p>
                </div>
              </details>

              <details>
                <summary>Missing index.html, or build artifact check failed?</summary>
                <div class="doc-faq-content">
                  <p>Ensure the chosen folder is a complete web export directory containing <code>index.html</code>. The CLI does not execute build steps. If your folder structure is non-standard but runs in browsers, pass <code>-y</code> to bypass the check (this does not bypass file size or quota limits).</p>
                </div>
              </details>

              <details>
                <summary>Why is there no local invite card image file after publishing?</summary>
                <div class="doc-faq-content">
                  <p>Default publish does not save an image (默认发布不写图片). This ensures terminal operations remain lightweight and fast. To download a PNG invite card locally, run <code>playtest card ./dist --out ./invite.png</code>, or pass <code>--card ./invite.png</code> during publish. Check network and directory write permissions if download fails; publishing the work and saving the local card are separate steps.</p>
                </div>
              </details>

              <details>
                <summary>Does unlisting mean only I can access the build?</summary>
                <div class="doc-faq-content">
                  <p>No. Unlisted ≠ Private access (不上广场 ≠ 私密访问). Unlisting only removes the work from the public Plaza feed; anyone with the direct link can still open and play. To revoke access completely, run <code>playtest rm ./dist</code> to delete the work; <code>unlist</code> hides it from the showcase rather than securing it behind passwords.</p>
                </div>
              </details>

              <details>
                <summary>Older automation scripts with --gate now fail with an error?</summary>
                <div class="doc-faq-content">
                  <p>--gate option has been removed (--gate 这个选项已撤出). The system now standardizes on a clean dual-domain architecture: root domain hosts invitations and playtester intake, while subdomains sandbox gameplay, eliminating the need for once / always / never mode toggles. Remove <code>--gate</code> from your scripts and retry.</p>
                </div>
              </details>

              <details>
                <summary>My installed CLI behavior differs from this documentation?</summary>
                <div class="doc-faq-content">
                  <p>Run <code>playtest --version</code> and <code>playtest --help</code> in your terminal and compare with GitHub Releases. This documentation describes the latest platform release conventions; local source edits do not automatically update installed binaries.</p>
                </div>
              </details>
            </div>

            <div class="doc-closing">
              <span class="doc-kicker">Back to the simplest step</span>
              <h3>Send it to one person. Hear what they say.</h3>
              <CommandBlock command="playtest ./dist" label="Ready to go" />
              <p>For more options and usage examples, run <code>playtest --help</code> anytime.</p>
            </div>
          </Chapter>
        </article>

        <aside class="docs-toc">
          <div class="docs-toc-head">Table of Contents</div>
          <nav aria-label="Documentation chapters">{links}</nav>
          <div class="docs-toc-foot">
            <span>Look up as needed, no need to memorize.</span>
            <code>playtest --help</code>
            <a href={href({ name: "sites" })}>
              <span>Back to my projects</span>
              <span aria-hidden="true">↗</span>
            </a>
          </div>
        </aside>
      </div>

      <footer class="docs-footer">
        <a href={href({ name: "docs", section: "start" })} onClick={() => locate("start")}>
          <span>Back to top ↑</span>
        </a>
      </footer>
    </div>
  );
}

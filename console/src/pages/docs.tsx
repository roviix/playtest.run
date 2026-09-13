import type { ComponentChildren } from "preact";
import { useEffect, useRef, useState } from "preact/hooks";
import { DOC_SECTIONS, href, type DocSection } from "../router";

const chapters: Record<DocSection, { title: string; description: string }> = {
  start: { title: "快速开始", description: "从一个能玩的版本，到第一条分享链接。" },
  publish: { title: "发布与更新", description: "选对发布方式，让下一版沿用同一个链接。" },
  share: { title: "分享与找人测", description: "发给朋友，或者让更多人发现你的作品。" },
  manage: { title: "管理作品", description: "打开、检查、回滚，每一步都有明确的对象。" },
  automation: { title: "脚本与 AI 助手", description: "把同一套发布能力接进你的工作流。" },
  troubleshoot: { title: "常见问题", description: "先辨认问题，再走最短的恢复路径。" },
};

export function CommandBlock({ command, label = "终端" }: { command: string; label?: string }) {
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
      setErrorMsg("无法自动复制，请手动选中复制。");
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
          aria-label={`复制命令：${command}`}
        >
          {copied ? (
            <>
              <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
                <polyline points="20 6 9 17 4 12" />
              </svg>
              <span>已复制</span>
            </>
          ) : (
            <>
              <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
                <rect x="9" y="9" width="13" height="13" rx="2" ry="2" />
                <path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1" />
              </svg>
              <span>复制</span>
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
        <span>关键原则</span>
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
    document.title = "使用文档 · playtest";
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
        <div class="docs-eyebrow">PLAYTEST · 使用指南</div>
        <h1 id="docs-title" tabIndex={-1}>从能玩，到有人玩。</h1>
        <p>把构建目录或本地端口交给 playtest，拿到一条可以直接分享给玩家的体验链接。<br class="docs-desktop-break" />极简发布、轻松招募试玩者，再带着真实反馈做出下一版。</p>
        <div class="docs-hero-meta">
          <span>首次发布无需注册</span>
          <i aria-hidden="true" />
          <span>玩家点开即玩无门槛</span>
          <i aria-hidden="true" />
          <span>增量哈希比对同链更新</span>
        </div>
      </header>

      <details class="docs-mobile-menu" ref={mobileMenu}>
        <summary>
          <span>当前章节：<b>{chapters[active].title}</b></span>
          <span aria-hidden="true">展开目录 ⌄</span>
        </summary>
        <nav aria-label="文档章节（手机）">{links}</nav>
      </details>

      <div class="docs-layout">
        <article class="docs-article" aria-label="playtest 使用指南">
          <Chapter name="start">
            <div class="doc-start-command">
              <span class="doc-kicker">极速上手：一条命令直接发布</span>
              <CommandBlock command="playtest ./dist" label="终端" />
              <p>将 <code>./dist</code> 换成你已构建好的导出目录。命令完成后，即可复制分享链接，或用手机扫描终端打印的二维码立即体验。</p>
            </div>

            <div class="doc-step">
              <div class="doc-step-head">
                <span class="doc-step-badge">1</span>
                <h3>下载与安装 CLI</h3>
              </div>
              <p>前往 <a href="https://github.com/roviix/playtest.run/releases" target="_blank" rel="noreferrer">GitHub Releases ↗</a> 选择与你的系统和处理器架构匹配的文件，解压后将 <code>playtest</code> 可执行文件放入系统的 PATH 路径中。Windows 用户请使用 <code>playtest.exe</code>。</p>
              <p>打开终端，确认安装成功：</p>
              <CommandBlock command="playtest --version" label="验证版本" />
              <p>单一二进制，开箱即用，无需配置复杂的运行时。如果终端提示找不到命令，请检查可执行文件所在目录是否已正确加入 PATH 环境变量。</p>
            </div>

            <div class="doc-step">
              <div class="doc-step-head">
                <span class="doc-step-badge">2</span>
                <h3>准备网页构建产物</h3>
              </div>
              <p>先使用项目本身的构建命令（如 <code>npm run build</code>）或游戏引擎（Godot / Unity / Cocos / Phaser 等）的 Web 导出功能。通常会得到一个包含 <code>index.html</code> 的输出目录。</p>
              <Note title="上传构建导出物，不是整个源码仓库">
                <p>请指定 <code>dist</code>、<code>build</code> 等导出目录。CLI 专注于作品分发，不会替你执行项目的构建脚本；扫描时不按 <code>.gitignore</code> 过滤，点开头的隐藏文件及符号链接会自动跳过。</p>
              </Note>
            </div>

            <div class="doc-step">
              <div class="doc-step-head">
                <span class="doc-step-badge">3</span>
                <h3>发布并获取分享链接</h3>
              </div>
              <p>首次发布时，CLI 会自动为你申请匿名身份凭据，并在终端完整打印作品名、版本、分享网址、终端二维码与结果查阅入口：</p>
              <div class="doc-output">
                <div class="doc-output-head"><span aria-hidden="true" />终端成功输出示例</div>
                <pre>{'已发布《小小星球》v1\nhttps://playtest.run/p/brisk-otter-41\n\n[终端二维码]\n匿名链接将在指定时间到期\n查看试玩结果：终端中的控制台地址'}</pre>
              </div>
              <p>默认发布不写图片，也不会自动上公开广场。匿名链接通常有效 24 小时（以终端实际到期提示为准），适合立即发给朋友或在手机上实机扫码测试。</p>
            </div>

            <h3>随时绑定账号保留作品</h3>
            <p>当需要长期保留作品或跨设备管理时，在终端运行 <code>playtest login</code>，按提示在 GitHub 网页上完成一次授权绑定即可。绑定后作品不再受 24 小时有效期限制。</p>
            <p><strong>注意</strong>：浏览器控制台与终端 CLI 分别独立保存身份，终端登录不会自动同步到浏览器。网页端支持 GitHub 快捷登录，也可以在登录弹窗选择输入访问令牌（可在本机 <code>~/.config/playtest/config.json</code> 中找到）。访问令牌属于管理凭据，切勿发给玩家。</p>
          </Chapter>

          <Chapter name="publish">
            <h3>根据你的产物形态选择发布方式</h3>
            <p>playtest 原生支持静态目录托管、本地端口直连转发以及前后端混合部署三种形态：</p>
            <div class="doc-table-wrap">
              <table>
                <thead>
                  <tr>
                    <th>产物形态</th>
                    <th>推荐命令</th>
                    <th>关闭终端后的表现</th>
                  </tr>
                </thead>
                <tbody>
                  <tr>
                    <td>构建好的网页目录 (HTML5 / Web)</td>
                    <td><code>playtest ./dist</code></td>
                    <td>静态作品持续在线，可在浏览器直接访问</td>
                  </tr>
                  <tr>
                    <td>正在运行的本地开发服务</td>
                    <td><code>playtest 5173</code></td>
                    <td>端口穿透连接断开，服务随之中断</td>
                  </tr>
                  <tr>
                    <td>静态前端 ＋ 本地后端 API</td>
                    <td><code>playtest ./dist --backend 3000</code></td>
                    <td>静态页面保留，后端接口返回 503</td>
                  </tr>
                </tbody>
              </table>
            </div>
            <p>端口分享前请确保本地开发服务已经启动；CLI 会自动检查端口、连接隧道并保持前台运行。网络波动时会自动尝试重连，按 Ctrl-C 即可随时退出。</p>

            <h3>版本迭代：沿用同一个链接</h3>
            <CommandBlock command={'playtest ./dist -m "修复了移动端操作，请再试一次"'} label="发布新版本" />
            <p>在同一个发布目录下再次执行发布命令，CLI 会自动比对文件内容哈希，<strong>仅增量上传发生变化的文件</strong>。作品版本自动递增（v1 → v2），原有的分享链接保持不变，玩家打开始终看到最新内容。</p>
            <CommandBlock command="playtest ./dist --to brisk-otter-41" label="更换了导出目录，更新指定作品" />
            <p>如果更换了打包目录或构建机器，使用 <code>--to &lt;作品标识&gt;</code> 明确更新目标。如果明确希望创建一个全新的作品和独立链接，可使用 <code>--to new</code>。</p>
            <Note title="当前目录，不是自动识别项目">
              <p>运行过 <code>playtest ./dist</code> 后，请使用 <code>playtest open ./dist</code>，或者进入 dist 目录后直接运行 <code>playtest open</code>。CLI 不会自动猜测子目录或最近操作过的作品。</p>
            </Note>

            <h3>前后端混合与单页应用路由</h3>
            <CommandBlock command="playtest ./dist --backend 3000 --spa" label="静态前端 + 本地后端穿透" />
            <p>系统会优先匹配静态文件和目录索引；开启 <code>--spa</code> 后，未命中的页面导航请求会自动回退到 <code>index.html</code>（静态资源请求不回退）；仍未命中的 API 路径及 WebSocket 请求则通过隧道转发至本地 3000 端口。若本地后端离线，对应接口将返回 503。</p>

            <details class="doc-details">
              <summary>发布常用高级参数速查</summary>
              <div class="doc-details-content">
                <dl class="doc-definitions">
                  <dt><code>--isolated auto|on|off</code></dt>
                  <dd>跨源隔离（Cross-Origin Isolation）。默认 auto，自动检测 WebAssembly / SharedArrayBuffer 多线程需求并开启；普通页面无需额外配置。</dd>
                  <dt><code>--spa</code></dt>
                  <dd>开启前端路由的单页应用导航回退；仅对 HTML 导航生效，缺失的静态资源不会误回退。</dd>
                  <dt><code>-y / --yes</code></dt>
                  <dd>发布时绕过导出物阻断性检查（如缺少 index.html 警告），但不绕过文件大小与配额限制。</dd>
                  <dt><code>--no-qr</code></dt>
                  <dd>不在终端打印二维码字符画；不影响生成的网址与邀请卡。</dd>
                </dl>
                <p>端口分享模式聚焦即时调试，不接受版本说明、封面、长期简介、广场招募或 SPA 回退选项。纯数字表示端口（1–65535）；若要发布纯数字命名的目录请显式写为 <code>./5173</code>。</p>
              </div>
            </details>
          </Chapter>

          <Chapter name="share">
            <h3>双域名架构：邀请与游玩分离</h3>
            <p>playtest 采用严格的双域名设计保障安全隔离与最佳体验：主域名承载作品邀请函，展示介绍、收集试玩者名额与反馈；独立子域名沙箱安全运行游戏逻辑：</p>
            <div class="doc-addresses">
              <div class="doc-address-card">
                <span>分享给玩家的邀请函（主域）</span>
                <code>https://playtest.run/p/&lt;作品标识&gt;</code>
              </div>
              <div class="doc-address-card">
                <span>运行作品的独立沙箱（子域）</span>
                <code>https://&lt;作品标识&gt;.playtest.run</code>
              </div>
            </div>

            <h3>下载高清分享邀请卡</h3>
            <CommandBlock command="playtest card ./dist --out ./invite.png" label="单独下载邀请卡" />
            <p>默认发布不写图片。这是为了保证终端操作轻量迅速。当你需要将精美的卡片发到微信群、QQ 群或社交平台时，可以运行 <code>playtest card</code> 单独生成 PNG 邀请卡，或者在发布命令中加上 <code>--card ./invite.png</code>。玩家长按扫码即可直达作品。</p>

            <h3>公开到广场与招募试玩者</h3>
            <CommandBlock command="playtest ./dist --seats 10" label="公开到广场并招募 10 人" />
            <p>传入 <code>--seats 10</code> 会自动将作品展示到广场并标明招募 10 位试玩者，无需重复指定 <code>--public</code>。如果只想展示在广场而不限制招募人数，直接传入 <code>--public</code> 即可。</p>
            <Note title="不上广场 ≠ 私密访问">
              <p>不上广场仅代表作品不会在公开广场列表中展示。任何持有链接的玩家依然可以直接打开并体验作品。如果需要从广场撤下，请运行 <code>playtest unlist ./dist</code>；这并不是密码保护。</p>
            </Note>

            <details class="doc-details">
              <summary>作品展示资料配置（设置后自动保存）</summary>
              <div class="doc-details-content">
                <dl class="doc-definitions">
                  <dt><code>-n / --name</code></dt>
                  <dd>玩家看到的作品名。省略时默认使用目录名，后续发布可随时更新。</dd>
                  <dt><code>--summary</code></dt>
                  <dd>作品的长期简介，最多 140 字。设置一次后自动保留，无需每次输入。</dd>
                  <dt><code>-m / --note</code></dt>
                  <dd>当前版本更新了什么、希望大家重点测什么，最多 280 字。用于邀请函首屏与关注通知。</dd>
                  <dt><code>--cover</code></dt>
                  <dd>封面图片路径（PNG / JPEG / WebP，2 MB 以内）。后续更新省略时沿用已有封面。</dd>
                  <dt><code>--community</code></dt>
                  <dd>开发者社群或群聊链接（支持 QQ 群、微信群二维码页、Discord 等 HTTP 链接），将在邀请函底部提供入口。</dd>
                </dl>
              </div>
            </details>
          </Chapter>

          <Chapter name="manage">
            <p>管理命令支持以本地发布目录或线上作品标识作为参数。只读和取卡命令可省略目标；删除、下架和回滚必须明确指定。</p>
            <div class="doc-table-wrap">
              <table>
                <thead>
                  <tr>
                    <th>常用命令</th>
                    <th>具体用途</th>
                  </tr>
                </thead>
                <tbody>
                  <tr><td><code>whoami</code></td><td>查看当前登录身份、配额与本机记录的作品数</td></tr>
                  <tr><td><code>ls</code></td><td>列出当前身份下的所有作品；未登录时列出本机记录作品</td></tr>
                  <tr><td><code>open [目标]</code></td><td>打印分享链接并在默认系统浏览器中打开邀请函</td></tr>
                  <tr><td><code>files [目标] --version v3</code></td><td>查看指定版本的文件清单、体积与哈希；省略版本查看线上当前版</td></tr>
                  <tr><td><code>versions [目标]</code></td><td>查看该作品的所有历史发布版本，标明当前生效版本</td></tr>
                  <tr><td><code>card [目标] --out invite.png</code></td><td>将线上当前的邀请卡下载并保存为本地图片</td></tr>
                </tbody>
              </table>
            </div>

            <CommandBlock command="playtest ls" label="列出名下作品" />
            <CommandBlock command="playtest open ./dist" label="在浏览器中打开邀请函" />
            <CommandBlock command="playtest versions ./dist" label="查看所有历史版本" />

            <h3>秒级回滚到历史版本</h3>
            <CommandBlock command="playtest rollback ./dist v3" label="版本秒级回滚" />
            <p>将线上生效的版本瞬间切回已有的历史版本（参数写 <code>3</code> 或 <code>v3</code> 均可）。由于历史文件与版本清单完整保留在服务器上，回滚无需重新上传任何文件，立即可用。</p>
            <p><strong>注意</strong>：回滚立即执行，没有第二次确认；请在操作前使用 <code>playtest versions</code> 仔细核对目标版本号。</p>

            <h3>下架与删除的区别</h3>
            <CommandBlock command="playtest unlist ./dist" label="从广场下架（保留分享链接）" />
            <CommandBlock command="playtest rm ./dist" label="删除作品（分享链接失效）" />
            <p><strong>下架（unlist）</strong>：仅从公开广场中撤出，已有分享链接依然有效，历史版本与试玩数据完整保留。适合内测阶段性结束。</p>
            <p><strong>删除（rm）</strong>：彻底销毁作品及其所有版本，分享链接立即失效打不开。终端会要求输入确认，添加 <code>-y</code> 可跳过确认；脚本模式下必须显式指定确认。</p>
            <Note title="同一份作品，不同设备的本机状态">
              <p>控制台和终端使用同一账号管理作品，但目录映射保存在本机。换电脑后可用作品标识，或通过 <code>--to</code> 明确更新已有作品。</p>
            </Note>
          </Chapter>

          <Chapter name="automation">
            <h3>机器读取模式（--json）</h3>
            <CommandBlock command="playtest ./dist --json" label="CI/CD 自动化集成" />
            <p>追加 <code>--json</code> 参数时，一次性命令的 stdout 保证仅输出单行纯 JSON 对象，便于脚本和 <code>jq</code> 等工具解析；进度条与人读提示均输出至 stderr，互不干扰。</p>
            <div class="doc-table-wrap">
              <table>
                <thead>
                  <tr>
                    <th>JSON 关键字段</th>
                    <th>说明与读取建议</th>
                  </tr>
                </thead>
                <tbody>
                  <tr><td><code>ok / action</code></td><td>布尔值指示是否成功，以及具体执行的操作类型</td></tr>
                  <tr><td><code>slug / url / version</code></td><td>作品唯一标识、默认分享地址（邀请函）、本次版本号</td></tr>
                  <tr><td><code>qr_text / card_url</code></td><td>终端二维码文本内容、远端邀请卡高清预览地址</td></tr>
                  <tr><td><code>card_path</code></td><td>本地邀请卡图片保存成功的绝对路径（仅在显式请求保存时出现）</td></tr>
                  <tr><td><code>expires_at / console_url</code></td><td>匿名作品的到期时间戳、创作者结果查看入口</td></tr>
                  <tr><td><code>elapsed_ms / timings</code></td><td>总耗时（毫秒）、文件扫描与网络上传的分阶段耗时分析</td></tr>
                  <tr><td><code>findings</code></td><td>导出物检查结果与操作告警；即使发布成功也建议脚本检查此项</td></tr>
                </tbody>
              </table>
            </div>
            <Note title="持续运行的命令需逐行流式读取">
              <p>端口分享和混合模式持续输出 JSON 事件，包括 online、players、reconnecting、stopped；混合模式先有上传结果。不要把整个 stdout 当作一个对象。<code>open --json</code> 不弹浏览器，<code>rm --json</code> 要加 <code>-y</code>。</p>
            </Note>

            <h3>连接 AI 编程助手（MCP Server）</h3>
            <CommandBlock command="playtest mcp --setup" label="打印 MCP 配置片段" />
            <p>在 Cursor、Claude Code、Antigravity 等编辑器中将 <code>playtest mcp</code> 配置为 MCP 服务后，AI 助手可在对话中直接调用 5 大内置工具：</p>
            <p><code>playtest_upload</code>（发布构建产物）、<code>playtest_share</code>（端口穿透分享）、<code>playtest_list</code>（列出名下作品）、<code>playtest_site</code>（查询作品详情与反馈）、<code>playtest_card</code>（生成分享邀请卡）。卡片可以直接在对话中返回预览。</p>

            <h3>指定 API 地址与自托管部署</h3>
            <CommandBlock command="playtest --api http://localhost:8787 ls --json" label="连接自托管实例" />
            <p>服务地址按以下优先级依次解析：命令行参数 <code>--api</code> → 环境变量 <code>PLAYTEST_API</code> → 本机配置文件记录 → 官方默认服务。<code>--api</code> 与 <code>--json</code> 可写在子命令前后。</p>

            <details class="doc-details">
              <summary>CLI 退出码与本机配置文件说明</summary>
              <div class="doc-details-content">
                <p><strong>退出码速查</strong>：0 成功 · 1 未预期失败 · 2 用法错误 · 3 身份失效 · 4 网络问题 · 5 服务端错误 · 6 输入或导出物问题 · 7 配额耗尽。在 JSON 模式下可进一步读取 <code>code</code> 与 <code>hint</code> 字段。</p>
                <p>本机配置文件路径：macOS / Linux 位于 <code>~/.config/playtest/config.json</code>，Windows 位于 <code>%APPDATA%\playtest\config.json</code>。其中包含登录令牌、API 地址和本地目录映射记录；切勿提交至公开代码仓库或泄露给玩家。</p>
              </div>
            </details>
          </Chapter>

          <Chapter name="troubleshoot">
            <div class="doc-faq">
              <details>
                <summary>“这个目录还没发过”，但我明明已经发布过了？</summary>
                <div class="doc-faq-content">
                  <p>请确认当前所在的终端工作目录。如果在父级目录运行过 <code>playtest ./dist</code>，请使用 <code>playtest open ./dist</code>；或者先 <code>cd dist</code> 再运行 <code>playtest open</code>。也可以先运行 <code>playtest ls</code> 查看所有作品标识，再通过标识操作。CLI 不会猜测未指定的目标。</p>
                </div>
              </details>

              <details>
                <summary>端口没有监听，或本地后端突然离线？</summary>
                <div class="doc-faq-content">
                  <p>请确认本地服务已先启动并正在监听对应端口，然后再运行分享命令。分享期间请保持终端运行与电脑唤醒。在前后端混合模式下，静态页面正常加载不代表后端接通；若本地后端异常退出，接口请求将收到 503 错误。</p>
                </div>
              </details>

              <details>
                <summary>匿名链接到期，或登录身份失效？</summary>
                <div class="doc-faq-content">
                  <p>匿名作品通常在 24 小时后到期失效。失效后重新发布可能会分配全新的作品标识与链接，旧链接不会因此复活。如需长期保留，请在有效期内运行 <code>playtest login</code> 绑定 GitHub 账号。浏览器与 CLI 身份各自独立，终端身份状态可用 <code>playtest whoami</code> 查看。</p>
                </div>
              </details>

              <details>
                <summary>找不到 index.html，或者导出物检查不通过？</summary>
                <div class="doc-faq-content">
                  <p>请确认选择的目录是包含 <code>index.html</code> 的完整 Web 导出目录。CLI 本身不执行项目构建。如果你确定目录结构特殊但能在浏览器中正常运行，可以传入 <code>-y</code> 强制跳过检查；但这不会绕过单文件体积与账号配额限制。</p>
                </div>
              </details>

              <details>
                <summary>发布成功后，为什么本地没有看到邀请卡图片文件？</summary>
                <div class="doc-faq-content">
                  <p>默认发布不写图片。这是为了保证终端轻量快速。需要保存邀请卡 PNG 图片时，请显式运行 <code>playtest card ./dist --out ./invite.png</code>，或者在发布时添加 <code>--card ./invite.png</code>。若下载失败，请检查网络连接与目标目录写权限；作品发布与本地图片保存是两个独立结果。</p>
                </div>
              </details>

              <details>
                <summary>不上广场，是不是就只有我一个人能访问？</summary>
                <div class="doc-faq-content">
                  <p>不是。不上广场 ≠ 私密访问。不上广场仅代表作品不进入公开广场列表，任何拿到链接的玩家依然可以直接打开试玩。如果需要使链接失效，请使用 <code>playtest rm ./dist</code> 删除作品；<code>unlist</code> 是下架展示而非私密保护。</p>
                </div>
              </details>

              <details>
                <summary>旧自动化脚本中使用 --gate 选项，为什么现在报错？</summary>
                <div class="doc-faq-content">
                  <p>--gate 这个选项已撤出。现在的体系全面采用清晰的双域名规范：主域统一提供邀请函与试玩登记，子域负责安全运行游戏，无需再通过 once / always / never 进行模式切换。请从脚本中移除 <code>--gate</code> 及其参数后重试。</p>
                </div>
              </details>

              <details>
                <summary>我安装的 CLI 命令表现与文档中描述的不一致？</summary>
                <div class="doc-faq-content">
                  <p>请首先在终端运行 <code>playtest --version</code> 与 <code>playtest --help</code>，并与 GitHub Releases 页面进行对照。本文档描述当前项目的最新发布约定；本地源码修改不代表已发布到安装包中。</p>
                </div>
              </details>
            </div>

            <div class="doc-closing">
              <span class="doc-kicker">回到最简单的一步</span>
              <h3>先发给一个人，听听他怎么说。</h3>
              <CommandBlock command="playtest ./dist" label="随时出发" />
              <p>更多详细参数与使用示例，可随时在终端运行 <code>playtest --help</code> 查看完整帮助。</p>
            </div>
          </Chapter>
        </article>

        <aside class="docs-toc">
          <div class="docs-toc-head">阅读目录</div>
          <nav aria-label="文档章节">{links}</nav>
          <div class="docs-toc-foot">
            <span>按需查阅，不必从头记住。</span>
            <code>playtest --help</code>
            <a href={href({ name: "sites" })}>
              <span>回到我的作品</span>
              <span aria-hidden="true">↗</span>
            </a>
          </div>
        </aside>
      </div>

      <footer class="docs-footer">
        <span>playtest · 创作者使用指南</span>
        <a href={href({ name: "docs", section: "start" })} onClick={() => locate("start")}>
          <span>回到开头 ↑</span>
        </a>
      </footer>
    </div>
  );
}

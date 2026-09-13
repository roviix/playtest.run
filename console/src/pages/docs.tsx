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
  const [message, setMessage] = useState("");
  const timer = useRef<ReturnType<typeof setTimeout>>();
  useEffect(() => () => clearTimeout(timer.current), []);
  async function copy() {
    clearTimeout(timer.current);
    try {
      await navigator.clipboard.writeText(command);
      setMessage("已复制");
    } catch {
      setMessage("无法自动复制，请选中命令手动复制。");
    }
    timer.current = setTimeout(() => setMessage(""), 4000);
  }
  return <div class="doc-code">
    <div class="doc-code-bar"><span>{label}</span><button type="button" onClick={copy} aria-label={`复制命令：${command}`}>复制 <span aria-hidden="true">⧉</span></button></div>
    <pre tabIndex={0} aria-label={label}><code>{command}</code></pre>
    <span class="doc-copy-status" role="status">{message}</span>
  </div>;
}

function Note({ title, children }: { title: string; children: ComponentChildren }) {
  return <aside class="doc-note"><strong>{title}</strong><div>{children}</div></aside>;
}

function Chapter({ name, children }: { name: DocSection; children: ComponentChildren }) {
  const number = String(DOC_SECTIONS.indexOf(name) + 1).padStart(2, "0");
  return <section class="doc-section" id={`doc-${name}`} aria-labelledby={`doc-title-${name}`}>
    <header class="doc-section-head"><span class="doc-number">{number}</span><div><h2 id={`doc-title-${name}`} tabIndex={-1}>{chapters[name].title}</h2><p>{chapters[name].description}</p></div></header>
    {children}
  </section>;
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
  const links = DOC_SECTIONS.map((name, index) => <a key={name} href={href({ name: "docs", section: name })} aria-current={active === name ? "location" : undefined} onClick={() => locate(name)}><span>{String(index + 1).padStart(2, "0")}</span>{chapters[name].title}</a>);
  return <div class="docs-page">
    <header class="docs-hero">
      <div class="docs-eyebrow"><span class="docs-rule" /> PLAYTEST / 使用文档</div>
      <h1 id="docs-title" tabIndex={-1}>从能玩，到有人玩。</h1>
      <p>把目录或本地端口交给 playtest，拿到一条可以分享的链接。<br class="docs-desktop-break" />发布、找人试玩，再带着反馈做下一版。</p>
      <div class="docs-hero-meta"><span>首次发布无需登录</span><i aria-hidden="true" /><span>玩家点开就能开始</span></div>
    </header>
    <details class="docs-mobile-menu" ref={mobileMenu}><summary>阅读目录 <span>{chapters[active].title} <b aria-hidden="true">⌄</b></span></summary><nav aria-label="文档章节（手机）">{links}</nav></details>
    <div class="docs-layout">
      <article class="docs-article" aria-label="playtest 使用指南">
        <Chapter name="start">
          <div class="doc-start-command"><span class="doc-kicker">第一条命令，就这么简单</span><CommandBlock command="playtest ./dist" /><p>把 <code>./dist</code> 换成已经构建好的导出目录。发布完成后，复制链接，或用手机扫描终端二维码。</p></div>
          <h3>01 / 安装 CLI</h3>
          <p>在 <a href="https://github.com/roviix/playtest.run/releases" target="_blank" rel="noreferrer">GitHub Releases ↗</a> 选择与你的系统和处理器匹配的文件，解压后将 <code>playtest</code> 可执行文件放入 PATH。Windows 使用 <code>playtest.exe</code>。</p>
          <p>打开新的终端，确认命令可用：</p><CommandBlock command="playtest --version" />
          <p>如果终端提示找不到命令，先检查可执行文件所在目录是否已加入 PATH。下载或安装遇到问题不等于作品发布失败。</p>
          <h3>02 / 准备能玩的版本</h3>
          <p>先使用项目自己的构建命令，或游戏引擎的 Web 导出功能。通常得到一个包含 <code>index.html</code> 的目录。CLI 不替你构建，也不执行你的项目脚本。</p>
          <Note title="上传导出物，不是整个源码仓库">指定 <code>dist</code>、<code>build</code> 等构建目录。扫描不按 <code>.gitignore</code> 过滤；点开头的文件和目录跳过，符号链接不跟随。</Note>
          <h3>03 / 发出去，再回来看看</h3>
          <p>第一次使用自动申请匿名身份。终端会给出作品、版本、分享链接、二维码、到期提醒和结果入口；默认不下载图片，也不会自动上广场。</p>
          <div class="doc-output"><span>成功输出示意</span><pre>{'已发布《小小星球》v1\nhttps://playtest.run/p/brisk-otter-41\n\n[终端二维码]\n匿名链接将在指定时间到期\n查看试玩结果：终端中的控制台地址'}</pre></div>
          <p>匿名链接通常有效 24 小时，以实际到期提示为准。需要使用账号身份时运行 <code>playtest login</code>，按终端提示在 GitHub 确认；登录不是无条件永久托管的承诺。</p>
          <p>浏览器控制台和 CLI 分别保存身份，不会因为终端登录就自动登录浏览器。使用同一个 GitHub 账号，或在登录弹窗选择访问令牌。已有令牌可在本机 <code>~/.config/playtest/config.json</code> 中找到。不要把令牌发给玩家。</p>
        </Chapter>
        <Chapter name="publish">
          <h3>选择一种发布方式</h3>
          <div class="doc-table-wrap"><table><thead><tr><th>你手上有什么</th><th>命令</th><th>关闭终端后</th></tr></thead><tbody>
            <tr><td>构建好的网页目录</td><td><code>playtest ./dist</code></td><td>静态作品继续可用，受有效期限制</td></tr>
            <tr><td>正在运行的开发服务</td><td><code>playtest 5173</code></td><td>服务离线</td></tr>
            <tr><td>静态前端＋本地后端</td><td><code>playtest ./dist --backend 3000</code></td><td>页面保留，后端功能不可用</td></tr>
          </tbody></table></div>
          <p>端口分享前先启动服务；CLI 会检查端口、连接转发，并持续运行。断线会尝试重连，按 Ctrl-C 停止。本机休眠和网络策略都会影响可用性。</p>
          <h3>下一版，仍然是同一个链接</h3>
          <CommandBlock command={'playtest ./dist -m "修复了移动端操作，请再试一次"'} />
          <p>同一个已记录的发布目录，默认更新原作品，只上传服务器缺少的文件内容。原作品仍有效时沿用链接；匿名作品已过期时，不要期待旧链接保留。</p>
          <CommandBlock command="playtest ./dist --to brisk-otter-41" label="换了导出目录，更新指定作品" />
          <p>把示例标识换成你的作品标识。明确想要一个新作品、新链接时，使用 <code>--to new</code>。</p>
          <Note title="当前目录，不是自动识别项目">运行过 <code>playtest ./dist</code> 后，使用 <code>playtest open ./dist</code>，或者进入 dist 后运行 <code>playtest open</code>。CLI 不猜子目录，也不猜最近发布的作品。</Note>
          <h3>带后端的版本</h3><CommandBlock command="playtest ./dist --backend 3000 --spa" />
          <p>先匹配静态文件和目录索引；开启 SPA 时，导航请求可以回退到 index.html；仍未命中才转到本地后端。资源请求不回退，不强制使用 /api 前缀，也不承诺所有 WebSocket 请求优先。后端离线时，相应请求返回 503。</p>
          <details class="doc-details"><summary>运行参数与模式限制</summary><dl class="doc-definitions">
            <dt><code>--isolated auto|on|off</code></dt><dd>目录发布的跨源隔离。默认 auto，按导出物检查决定；不需要时不用填写。</dd>
            <dt><code>--spa</code></dt><dd>前端路由的导航回退；不是所有缺失资源都返回 HTML。</dd>
            <dt><code>-y / --yes</code></dt><dd>发布时绕过导出物阻断性检查，不绕过身份、大小、配额和服务端校验。</dd>
            <dt><code>--no-qr</code></dt><dd>关闭终端二维码；不改变分享链接。</dd>
          </dl><p>端口分享不接受版本说明、封面、简介、社群、广场招募、SPA、显式隔离 on/off 或强制上传选项。需要这些能力时，先构建再发布目录。</p><p>纯数字表示端口（1–65535）；发布数字命名的目录请写 <code>./5173</code>。当前没有 <code>--watch</code>；内容变更后再次发布。</p></details>
        </Chapter>
        <Chapter name="share">
          <h3>先把链接发给一个人</h3>
          <p>默认分享地址是主域邀请函，介绍作品、展示版本并提供开始入口。作品子域负责运行游戏；<code>playtest open</code> 打开的是邀请函，不是绕过它的调试捷径。</p>
          <div class="doc-addresses"><div><span>分享给玩家</span><code>https://playtest.run/p/&lt;作品标识&gt;</code></div><div><span>运行作品</span><code>https://&lt;作品标识&gt;.playtest.run</code></div></div>
          <h3>需要一张图片，再下载邀请卡</h3><CommandBlock command="playtest card ./dist --out ./invite.png" />
          <p>默认发布不写图片。也可以在发布时指定 <code>--card ./invite.png</code>；<code>--card -</code> 仍表示不下载。保存路径相对于当前工作目录，同名文件会覆盖。</p>
          <p>单独运行 <code>playtest card</code> 会读取当前目录的发布记录，并在当前目录保存「作品名-邀请卡.png」。发布时取卡失败不会撤销作品，但会明确提示图片没有保存。</p>
          <h3>让更多人发现它</h3><CommandBlock command="playtest ./dist --seats 10" />
          <p>这表示展示到广场并招募 10 位试玩者，不必重复写 <code>--public</code>。只想展示、不设招募名额时用 <code>--public</code>。名额不是平台保证带来的人数，留名加入也不等于页面访问次数。</p>
          <Note title="不上广场 ≠ 私密访问">持有链接的人仍能打开作品。停止传入 <code>--public</code> 不会自动下架；需要撤下时运行 <code>playtest unlist ./dist</code>。这不是密码保护。</Note>
          <details class="doc-details"><summary>作品资料：哪些只需设置一次？</summary><dl class="doc-definitions">
            <dt><code>-n / --name</code></dt><dd>玩家看到的作品名。省略时根据目录等信息确定，需要固定名称时显式指定。</dd>
            <dt><code>--summary</code></dt><dd>作品长期简介，最多 140 字。省略时沿用已有简介。</dd>
            <dt><code>-m / --note</code></dt><dd>当前版本改了什么、希望重点测什么，最多 280 字；不是长期简介。</dd>
            <dt><code>--cover</code></dt><dd>PNG / JPEG / WebP，2 MB 以内。后续省略时沿用已有封面。</dd>
            <dt><code>--community</code></dt><dd>开发者群或社区的 HTTP(S) 地址，不是裸群号。省略时不修改已有群链接。</dd>
          </dl></details>
        </Chapter>
        <Chapter name="manage">
          <p>以下命令中的目标可以是已发布目录或作品标识，不是完整分享网址。只读和取卡命令可省略目标；删除、下架和回滚必须明确指定。</p>
          <CommandBlock command={'playtest ls\nplaytest open ./dist\nplaytest versions ./dist'} label="找到作品，打开，再看版本" />
          <div class="doc-table-wrap"><table><thead><tr><th>命令</th><th>做什么</th></tr></thead><tbody>
            <tr><td><code>whoami</code></td><td>查看当前身份和账号信息</td></tr><tr><td><code>ls</code></td><td>列出当前身份的作品；无身份时返回空列表</td></tr><tr><td><code>open [目标]</code></td><td>打印分享链接并打开浏览器</td></tr><tr><td><code>files [目标] --version v3</code></td><td>查看指定版本文件；省略版本时看线上当前版本</td></tr><tr><td><code>versions [目标]</code></td><td>查看历史，标明当前版本</td></tr><tr><td><code>card [目标] --out invite.png</code></td><td>下载当前邀请卡</td></tr>
          </tbody></table></div>
          <h3>回滚，不重新上传文件</h3><CommandBlock command="playtest rollback ./dist v3" />
          <p>切回已经存在的版本。版本写 <code>3</code> 或 <code>v3</code> 都可以。回滚立即执行，没有第二次确认；请先用 versions 看清要回到哪一版。</p>
          <h3>下架与删除，是两件事</h3><CommandBlock command="playtest unlist ./dist" label="从广场拿下来，分享链接保留" /><CommandBlock command="playtest rm ./dist" label="删除作品，终端会要求确认" />
          <p>下架直接执行，不删除作品。删除会使链接失效；加 <code>-y</code> 可跳过确认，脚本模式下必须显式提供确认，不会替你猜。</p>
          <Note title="看到的是同一份作品，不一定是同一份本机状态">控制台和终端使用同一账号管理作品，但目录映射保存在本机。换电脑后可用作品标识，或通过 <code>--to</code> 明确更新已有作品。</Note>
        </Chapter>
        <Chapter name="automation">
          <h3>机器读结果，人读说明</h3><CommandBlock command="playtest ./dist --json" />
          <p>一次性命令的 stdout 只有一个 JSON 对象，进度和说明走 stderr。默认上传的人类模式 stdout 也只有分享链接，可重定向到文件。</p>
          <div class="doc-table-wrap"><table><thead><tr><th>字段</th><th>读取方式</th></tr></thead><tbody>
            <tr><td><code>ok / action</code></td><td>操作是否成功，以及操作类型</td></tr><tr><td><code>slug / url / version</code></td><td>作品标识、默认分享地址、版本</td></tr><tr><td><code>qr_text / card_url</code></td><td>二维码文本、邀请卡地址</td></tr><tr><td><code>card_path</code></td><td>只有文件保存成功时出现</td></tr><tr><td><code>expires_at / console_url</code></td><td>可选的到期时间、结果入口</td></tr><tr><td><code>elapsed_ms / timings</code></td><td>包含附加等待的总耗时、上传阶段耗时</td></tr><tr><td><code>findings</code></td><td>检查结果和附加操作警告；成功后也要留意</td></tr>
          </tbody></table></div>
          <Note title="持续运行的命令要逐行读取">端口分享和混合模式持续输出 JSON 事件，包括 online、players、reconnecting、stopped；混合模式先有上传结果。不要把整个 stdout 当作一个对象。<code>open --json</code> 不弹浏览器，<code>rm --json</code> 要加 <code>-y</code>。</Note>
          <h3>让 AI 助手直接发布</h3><CommandBlock command="playtest mcp --setup" label="打印 MCP 配置片段，按编辑器要求配置" />
          <p>编辑器通过 <code>playtest mcp</code> 启动服务。五个工具是 <code>playtest_upload</code>、<code>playtest_share</code>、<code>playtest_list</code>、<code>playtest_site</code>、<code>playtest_card</code>。图片可以直接返回对话，但不会因此保存本地 PNG；隧道随 MCP 进程存活。</p>
          <h3>连接自托管服务</h3><CommandBlock command="playtest --api http://localhost:8787 ls --json" />
          <p>地址优先级：<code>--api</code> → <code>PLAYTEST_API</code> → 本机保存的地址 → 默认服务。<code>--api</code> 与 <code>--json</code> 可写在子命令前后。</p>
          <details class="doc-details"><summary>退出码与本机配置</summary><p>0 成功 · 1 未预期失败 · 2 用法错误 · 3 身份失效 · 4 网络问题 · 5 服务端错误 · 6 输入或导出物问题 · 7 配额耗尽。失败 JSON 中读取 code、message 和可能存在的 hint。</p><p>macOS / Linux 配置在 <code>~/.config/playtest/config.json</code>；Windows 在 <code>%APPDATA%\playtest\config.json</code>。其中包含令牌、API 地址和目录记录；不要提交仓库、截图公开或发给玩家。</p></details>
        </Chapter>
        <Chapter name="troubleshoot">
          <div class="doc-faq">
            <details><summary>“这个目录还没发过”，但我明明已经发布了？</summary><p>确认发布的是当前目录还是它的子目录。发布过 ./dist 时，用 <code>playtest open ./dist</code>。也可先运行 <code>playtest ls</code>，再用作品标识；CLI 不猜最近作品。</p></details>
            <details><summary>端口没有监听，或后端突然离线</summary><p>先启动本地服务，确认端口，再运行分享命令。保持终端运行和电脑唤醒。混合模式下静态页面可用不代表后端接通；检查后端连接提示与 503 错误。</p></details>
            <details><summary>匿名链接到期，或登录身份失效</summary><p>查看实际到期提示。匿名作品过期后再次发布可能拿到新链接，旧链接不会因此复活。登录失效时重新运行 <code>playtest login</code>。浏览器和 CLI 身份分别保存，可用 <code>playtest whoami</code> 检查终端身份。</p></details>
            <details><summary>找不到 index.html，或者导出物检查不通过</summary><p>确认选择的是完整 Web 导出目录，资源未遗漏。CLI 不构建项目。理解检查原因并确定要继续时才使用 <code>-y</code>；它不绕过大小和配额限制。</p></details>
            <details><summary>已发布成功，为什么没有邀请卡文件？</summary><p>默认不下载。需要时运行 <code>playtest card ./dist --out invite.png</code>。若显式下载失败，检查网络、目标目录和写权限；作品链接与图片保存是两个结果。</p></details>
            <details><summary>不上广场，是不是就只有我能访问？</summary><p>不是。不上广场只影响被发现，持有链接者仍然可以访问。<code>unlist</code> 不是私密访问设置，删除则会让链接失效。</p></details>
            <details><summary>旧脚本使用 --gate，为什么现在报错？</summary><p>这个选项已撤出。主域展示邀请函，子域运行作品，不再用 once / always / never 控制。移除 <code>--gate</code> 和它的值后重试。</p></details>
            <details><summary>我装的 CLI 和这里写的不一样</summary><p>先运行 <code>playtest --version</code> 与 <code>playtest --help</code>，对照 Releases。本文描述当前项目约定；本地源码改动不代表已经发布到安装包。不要将文档示例当成已安装版本的能力检测。</p></details>
          </div>
          <div class="doc-closing"><span class="doc-kicker">回到最简单的一步</span><h3>先发给一个人，听听他怎么说。</h3><CommandBlock command="playtest ./dist" /><p>更多精确参数可以随时运行 <code>playtest --help</code> 查看。</p></div>
        </Chapter>
      </article>
      <aside class="docs-toc"><span class="doc-kicker">阅读目录</span><nav aria-label="文档章节">{links}</nav><div class="docs-toc-foot"><span>按需阅读，不必从头记住。</span><code>playtest --help</code><a href={href({ name: "sites" })}>回到我的作品 <span aria-hidden="true">↗</span></a></div></aside>
    </div>
    <footer class="docs-footer"><span>playtest / 使用指南</span><a href={href({ name: "docs", section: "start" })} onClick={() => locate("start")}>回到开头 ↑</a></footer>
  </div>;
}

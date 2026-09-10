/**
 * playtest.js —— 「加一行脚本才有的」那一层（DESIGN §3.5、§4.6）。
 *
 * 边界很硬，写在这里免得后来的人往里加东西：
 *
 * - 只收四样：JS 错误、加载用时、开发者自己打的点、最后一次输入。没有热图、没有漏斗、
 *   没有会话录像，那些在 DESIGN §3.12 的「明确不做」里。
 * - 不收集玩家身份、不做跨站的任何事、不种第三方 cookie。会话 id 来自同源的
 *   `/_playtest/me`（边缘发的、HttpOnly cookie 的一个只读投影），拿不到就在 localStorage
 *   里生成一个只在这一个站有意义的随机数。
 * - 只往一个地址发：`/_playtest/me` 告诉我们的那个 api，或者 `<script data-api=…>` 写死的那个。
 * - 出任何问题都闭嘴：这是别人的游戏，我们的统计不能变成他们的 bug。所有回调裹 try/catch，
 *   服务端说不认识我们（4xx）就永久停发。
 *
 * 反馈按钮不设任何必填项——每加一个必填字段填写率就掉一截（DESIGN §3.5）。
 */

type Me = {
  sid: string;
  slug: string;
  version: number;
  isolated: boolean;
  api: string;
};

type Sample = {
  ts: string;
  kind: 'load' | 'input' | 'error' | 'event';
  name?: string;
  data?: unknown;
};

(function () {
  const win = window as unknown as Record<string, unknown>;
  if (win.playtest) return;

  /** 一批最多这么多条，和服务端的上限一致。 */
  const MAX_BATCH = 20;
  /** 合并这么久再发。玩家在玩，网络让给游戏。 */
  const FLUSH_MS = 1000;
  /** 同一个错误最多报几次；一整页最多报几个错。循环里报错的游戏不能把我们变成 DDoS。 */
  const PER_FINGERPRINT = 5;
  const MAX_ERRORS = 20;
  /** 堆栈截到这里。再长的部分对定位没有帮助，只会撑大每一条记录。 */
  const MAX_STACK = 2048;

  const openedAt = Date.now();
  const script = document.currentScript as HTMLScriptElement | null;
  const forcedApi = script?.dataset.api?.replace(/\/+$/, '');

  let api = '';
  let sid = '';
  let slug = '';
  /** 边缘认出了我们（`/_playtest/me` 回了 200）。只有这时同源才有 `/_playtest/follow`。 */
  let onEdge = false;
  let dead = false;
  let queue: Sample[] = [];
  let timer = 0;
  let seen: Record<string, number> = {};
  let errors = 0;
  let lastInput = 0;
  let sentInput = 0;

  // ------------------------------------------------------------ 发送

  function send(path: string, body: unknown, beacon?: boolean): Promise<boolean> {
    if (dead || !api) return Promise.resolve(false);
    const text = JSON.stringify(body);
    const url = api + path;
    // text/plain 是 sendBeacon 唯一能发的、也是不触发预检的类型。服务端不看 Content-Type。
    const type = 'text/plain;charset=UTF-8';
    if (beacon && navigator.sendBeacon) {
      try {
        return Promise.resolve(navigator.sendBeacon(url, new Blob([text], { type })));
      } catch (_) {
        // 继续往下走 fetch。
      }
    }
    return fetch(url, {
      method: 'POST',
      body: text,
      headers: { 'content-type': type },
      mode: 'cors',
      credentials: 'omit',
      keepalive: true,
    }).then(
      (res) => {
        // 4xx 是「我们和服务端对不上」（作品不在了、会话 id 不认）。再发一百次也一样，停。
        if (res.status >= 400 && res.status !== 429) dead = true;
        return res.ok;
      },
      () => false,
    );
  }

  function flush(beacon?: boolean) {
    if (timer) {
      clearTimeout(timer);
      timer = 0;
    }
    if (!queue.length || !sid || !slug) return;
    const events = queue.slice(0, 50);
    queue = queue.slice(events.length);
    send('/v1/ingest/events', { session: sid, slug: slug, events: events }, beacon);
  }

  function push(kind: Sample['kind'], name?: string, data?: unknown, at?: number) {
    if (dead) return;
    const one: Sample = { ts: new Date(at || Date.now()).toISOString(), kind: kind };
    if (name) one.name = name.slice(0, 200);
    if (data) one.data = data;
    queue.push(one);
    if (queue.length >= MAX_BATCH) flush();
    else if (!timer) timer = setTimeout(flush, FLUSH_MS) as unknown as number;
  }

  // ------------------------------------------------------------ 我是谁

  function anonId(): string {
    const key = 'playtest.sid';
    try {
      const kept = localStorage.getItem(key);
      if (kept && /^[0-9a-f]{32}$/.test(kept)) return kept;
    } catch (_) {
      /* 无痕模式下 localStorage 会抛，那就每次都是新的。 */
    }
    const bytes = new Uint8Array(16);
    crypto.getRandomValues(bytes);
    let out = '';
    for (let i = 0; i < bytes.length; i++) out += (bytes[i] + 0x100).toString(16).slice(1);
    try {
      localStorage.setItem(key, out);
    } catch (_) {
      /* 同上。 */
    }
    return out;
  }

  function configure(me: Me | null) {
    if (me && me.sid) {
      sid = me.sid;
      slug = me.slug;
      api = forcedApi || me.api || '';
      onEdge = !!me.slug;
    } else {
      // 开发者自己托管、或者门禁页被关掉了：会话 id 只能自己生成，slug 只能从域名猜。
      sid = anonId();
      slug = location.hostname.split('.')[0];
      api = forcedApi || '';
    }
    if (!api) {
      dead = true;
      return;
    }
    mountButton();
    flush();
  }

  fetch('/_playtest/me', { credentials: 'same-origin' })
    .then((res) => (res.status === 200 ? res.json() : null))
    .then(configure, () => configure(null));

  // ------------------------------------------------------------ 错误

  function fileOf(url?: string): string {
    if (!url) return '';
    try {
      return new URL(url, location.href).pathname.split('/').pop() || url;
    } catch (_) {
      return url;
    }
  }

  function report(message: string, stack?: string, file?: string, line?: number, col?: number) {
    if (errors >= MAX_ERRORS) return;
    if (!file && stack) {
      // 堆栈第一帧：`at update (https://…/main.js:412:9)`。
      const m = stack.match(/((?:[a-z]+:)?\/\/[^\s)]+|\/[^\s):]+):(\d+):(\d+)/);
      if (m) {
        file = m[1];
        line = +m[2];
        col = +m[3];
      }
    }
    const where = fileOf(file);
    const text = (message || '未命名的错误').slice(0, 200);
    // 点名册里同一个错误要能合并成一行，所以指纹里不放列号——同一行代码打包前后列号会变。
    const fingerprint = where ? text + ' @ ' + where + ':' + (line || 0) : text;
    const times = (seen[fingerprint] = (seen[fingerprint] || 0) + 1);
    if (times > PER_FINGERPRINT) return;
    errors++;
    push('error', fingerprint, {
      message: text,
      file: where,
      line: line || 0,
      col: col || 0,
      stack: stack ? stack.slice(0, MAX_STACK) : undefined,
    });
  }

  addEventListener('error', (e: ErrorEvent) => {
    try {
      // 资源加载失败（img / script 的 error）没有 message，那一层由边缘看得见，这里不管。
      if (e.message) report(e.message, e.error && e.error.stack, e.filename, e.lineno, e.colno);
    } catch (_) {
      /* 我们自己不能成为第二个错误。 */
    }
  });

  addEventListener('unhandledrejection', (e: PromiseRejectionEvent) => {
    try {
      const why = e.reason;
      const message = why && why.message ? why.message : String(why);
      report('未处理的 Promise 拒绝：' + message, why && why.stack);
    } catch (_) {
      /* 同上。 */
    }
  });

  // ------------------------------------------------------------ 加载分阶段

  function timing() {
    const data: Record<string, number> = { ms: Math.round(performance.now()) };
    try {
      const nav = performance.getEntriesByType('navigation')[0] as PerformanceNavigationTiming;
      if (nav) {
        data.ttfb = Math.round(nav.responseStart);
        data.dom = Math.round(nav.domContentLoadedEventEnd);
        data.load = Math.round(nav.loadEventEnd);
      }
      const paint = performance.getEntriesByName('first-contentful-paint')[0];
      if (paint) data.fcp = Math.round(paint.startTime);
    } catch (_) {
      /* 老浏览器没有这些条目，ms 一个数也够用。 */
    }
    // `ms` 是 load 之后第一次 requestAnimationFrame 的时刻，服务端拿它当 load_ms。
    // 它不等于引擎画出第一帧：wasm 编译和资源解包多半还在后面。想要更准的点，
    // 在游戏自己的第一帧里调 playtest.event("first_frame")。
    push('load', undefined, data);
  }

  function afterLoad() {
    requestAnimationFrame(() => setTimeout(timing, 0));
  }
  if (document.readyState === 'complete') afterLoad();
  else addEventListener('load', afterLoad);

  // ------------------------------------------------------------ 最后一次输入

  const note = () => {
    lastInput = Date.now();
  };
  ['pointerdown', 'keydown', 'touchstart'].forEach((type) =>
    addEventListener(type, note, { capture: true, passive: true }),
  );

  /** 只在离开时补一条：每次输入都发就成了行为记录，那是我们明确不做的东西。 */
  function leaving() {
    if (lastInput && lastInput !== sentInput) {
      sentInput = lastInput;
      push('input', undefined, undefined, lastInput);
    }
    flush(true);
  }
  addEventListener('pagehide', leaving);
  addEventListener('visibilitychange', () => {
    if (document.visibilityState === 'hidden') leaving();
  });

  // ------------------------------------------------------------ 反馈按钮

  function el<K extends keyof HTMLElementTagNameMap>(
    tag: K,
    css: string,
    text?: string,
  ): HTMLElementTagNameMap[K] {
    const node = document.createElement(tag);
    node.style.cssText = css;
    if (text) node.textContent = text;
    return node;
  }

  const FONT =
    "font:14px/1.5 -apple-system,BlinkMacSystemFont,'PingFang SC','Microsoft YaHei',sans-serif;";
  // 简写 font 会重置字号，所以想要小字得把 font-size 写在 FONT 后面。
  const SMALL = FONT + 'font-size:12px;';
  // 右下角，离边一点点。整块只有一个按钮宽，游戏中间不会被挡住。
  const ANCHOR = 'position:fixed;right:12px;bottom:12px;z-index:2147483000;' + FONT;

  function mountButton() {
    if (!document.body) {
      addEventListener('DOMContentLoaded', mountButton);
      return;
    }
    if (document.getElementById('playtest-feedback')) return;

    const button = el(
      'button',
      ANCHOR +
        'padding:6px 12px;border:0;border-radius:16px;background:rgba(17,17,17,.72);color:#fff;' +
        'cursor:pointer;box-shadow:0 1px 6px rgba(0,0,0,.25);-webkit-tap-highlight-color:transparent;',
      '反馈',
    );
    button.id = 'playtest-feedback';
    button.type = 'button';
    button.setAttribute('aria-label', '给开发者留一句话');
    button.onclick = () => {
      button.style.display = 'none';
      openPanel(() => {
        button.style.display = '';
      });
    };
    document.body.appendChild(button);
  }

  function openPanel(onClose: () => void) {
    const panel = el(
      'div',
      ANCHOR +
        'width:260px;max-width:calc(100vw - 24px);padding:12px;border-radius:12px;' +
        'background:#fff;color:#111;box-shadow:0 4px 24px rgba(0,0,0,.25);',
    );
    const box = el(
      'textarea',
      'width:100%;height:72px;box-sizing:border-box;padding:8px;border:1px solid #ddd;' +
        'border-radius:8px;resize:none;' +
        FONT,
    ) as HTMLTextAreaElement;
    box.placeholder = '哪里卡住了？一句话就行';
    box.maxLength = 2000;

    const row = el('div', 'display:flex;gap:8px;margin-top:8px;');
    const cancel = el(
      'button',
      'flex:0 0 auto;padding:6px 10px;border:0;border-radius:8px;background:#f2f2f2;' +
        'color:#555;cursor:pointer;' +
        FONT,
      '取消',
    );
    const send0 = el(
      'button',
      'flex:1;padding:6px 10px;border:0;border-radius:8px;background:#111;color:#fff;' +
        'cursor:pointer;' +
        FONT,
      '发送',
    );
    cancel.type = 'button';
    send0.type = 'button';

    function close() {
      panel.remove();
      onClose();
    }
    cancel.onclick = close;
    send0.onclick = () => {
      const text = box.value.trim();
      if (!text) {
        box.focus();
        return;
      }
      send0.disabled = true;
      send0.textContent = '发送中';
      send('/v1/ingest/feedback', {
        session: sid,
        slug: slug,
        text: text,
        seconds_in: Math.round((Date.now() - openedAt) / 1000),
      }).then((ok) => {
        if (ok) {
          landing(panel, close);
          return;
        }
        panel.textContent = '没发出去，等下再试一次';
        panel.style.width = 'auto';
        setTimeout(close, 1600);
      });
    };

    row.appendChild(cancel);
    row.appendChild(send0);
    panel.appendChild(box);
    panel.appendChild(row);
    document.body.appendChild(panel);
    box.focus();
  }

  // ------------------------------------------------------------ 玩后的落点

  /**
   * 根域：去掉当前主机名的第一段。`brisk-otter-41.playtest.run` → `playtest.run`，
   * 本机 `xxx.localhost:8443` → `localhost:8443`。
   *
   * 域名不写死在这个文件里。SDK 只认它现在所在的这个域，和 `/_playtest/me` 告诉它的那个
   * api 地址——写死一个域名就意味着自托管的人拿到的是我们的广场。
   */
  function plazaHref(): string {
    const rest = location.host.split('.').slice(1).join('.');
    return rest ? location.protocol + '//' + rest + '/' : '';
  }

  function hidden(form: HTMLFormElement, name: string, value: string) {
    const field = document.createElement('input');
    field.type = 'hidden';
    field.name = name;
    field.value = value;
    form.appendChild(field);
  }

  /**
   * 反馈发出去之后的那一屏（DESIGN §4.6）：谢谢、有新版本时告诉我、看看别的作品。
   * 不往作品画面里注入任何东西的前提下，这是唯一能放社交层的位置，而且它是开发者自己勾的。
   *
   * 「告诉我」只做邮箱，不做浏览器通知：浏览器通知要在作品自己的域上注册一个 Service Worker，
   * 而一个作用域只能有一个——我们注册就等于把开发者自己的 SW 顶掉，他的离线缓存和更新逻辑
   * 会跟着坏。多一个通知渠道换不来这个代价，想用浏览器通知的玩家在门禁页和广场上还能选。
   *
   * 表单是原生的、整页 POST 给同源的边缘，边缘转给控制面之后渲染一页结果再给「返回」。
   * 玩家的邮箱不经过我们的脚本，也从不直接交给另一个域（DESIGN §4.1）。
   */
  function landing(panel: HTMLElement, close: () => void) {
    panel.textContent = '';
    panel.appendChild(el('div', 'font-weight:600;margin-bottom:8px;', '谢谢，开发者会看到。'));

    if (onEdge && slug) {
      const form = el('form', 'display:flex;gap:6px;margin:0;');
      form.method = 'post';
      form.action = '/_playtest/follow';
      hidden(form, 'target', 'site:' + slug);
      hidden(form, 'from', 'sdk');
      // 同源的相对路径：边缘那一页上的「返回」把玩家送回他刚才在玩的这一页。
      hidden(form, 'to', location.pathname + location.search);

      const email = el(
        'input',
        'flex:1;min-width:0;padding:6px 8px;border:1px solid #ddd;border-radius:8px;' + FONT,
      );
      email.type = 'email';
      email.name = 'email';
      email.placeholder = '有新版本时告诉我';

      const tell = el(
        'button',
        'flex:0 0 auto;padding:6px 10px;border:0;border-radius:8px;background:#111;color:#fff;' +
          'cursor:pointer;' +
          FONT,
        '告诉我',
      );
      tell.type = 'submit';

      form.appendChild(email);
      form.appendChild(tell);
      panel.appendChild(form);
    }

    const foot = el('div', 'display:flex;align-items:baseline;gap:8px;margin-top:10px;');
    const plaza = plazaHref();
    if (plaza) {
      const link = el('a', SMALL + 'color:#666;text-decoration:underline;', '看看别的作品');
      link.href = plaza;
      link.target = '_blank';
      link.rel = 'noreferrer';
      foot.appendChild(link);
    }
    const done = el(
      'button',
      SMALL + 'margin-left:auto;padding:0;border:0;background:none;color:#999;cursor:pointer;',
      '关掉',
    );
    done.type = 'button';
    done.onclick = close;
    foot.appendChild(done);
    panel.appendChild(foot);
  }

  // ------------------------------------------------------------ 给开发者的三个口子

  win.playtest = {
    /** 标记一个进度里程碑：playtest.event("level_done", {level: 3}) */
    event: (name: string, data?: unknown) => push('event', String(name), data),
    /** 手动打开反馈框，开发者想自己放入口时用。 */
    feedback: () => mountButton(),
    /** 这次打开的会话 id，和控制台里那一行是同一个。 */
    session: () => sid,
  };
})();

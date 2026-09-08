// 四项探针：门禁页点「开始」之后，游戏那一侧还有没有用户手势。
//
// 同一份逻辑要能在三种地方跑：新文档 load 时、点击回调里、iframe 里，
// 所以这里只暴露一个 window.gateAudioProbe.run(label, opts)，谁调都一样。
//
// 结果同时写三处：页面正文（人看）、document.title（Playwright 取原文）、
// window.__probeResult（Playwright 取结构化数据）；在 iframe 里再 postMessage 给父页。

(() => {
  const SAMPLE_RATE = 8000;
  const TONE_HZ = 440;
  // 放够长，才能在 play() 之后隔 200ms 回头看它是不是真在往前走。
  const TONE_SECONDS = 1.0;

  // 极短的 wav，就地拼出来再转 base64——不联网、不落一个几 KB 的 blob 在仓库里。
  function toneDataUrl() {
    const n = Math.floor(SAMPLE_RATE * TONE_SECONDS);
    const bytes = new Uint8Array(44 + n);
    const view = new DataView(bytes.buffer);
    const ascii = (at, s) => {
      for (let i = 0; i < s.length; i++) bytes[at + i] = s.charCodeAt(i);
    };
    ascii(0, 'RIFF');
    view.setUint32(4, 36 + n, true);
    ascii(8, 'WAVEfmt ');
    view.setUint32(16, 16, true); // fmt 块长度
    view.setUint16(20, 1, true); // PCM
    view.setUint16(22, 1, true); // 单声道
    view.setUint32(24, SAMPLE_RATE, true);
    view.setUint32(28, SAMPLE_RATE, true); // 每秒字节数
    view.setUint16(32, 1, true); // 块对齐
    view.setUint16(34, 8, true); // 位深
    ascii(36, 'data');
    view.setUint32(40, n, true);
    for (let i = 0; i < n; i++) {
      const fade = Math.min(1, (n - i) / (SAMPLE_RATE * 0.05));
      bytes[44 + i] = 128 + Math.round(100 * fade * Math.sin((2 * Math.PI * TONE_HZ * i) / SAMPLE_RATE));
    }
    let bin = '';
    for (const b of bytes) bin += String.fromCharCode(b);
    return 'data:audio/wav;base64,' + btoa(bin);
  }

  const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

  // 被挡住的 resume() 不会 reject，它就那么一直挂着——这正是「the resume call was
  // failing quietly」。所以每个 await 都要有个上限，否则探针自己先卡死。
  const RESUME_TIMEOUT = 2000;
  function within(promise, ms, onTimeout) {
    return Promise.race([promise, sleep(ms).then(() => onTimeout)]);
  }

  // (d) Chromium 有 navigator.userActivation，WebKit 到今天还没有。
  function activation() {
    const ua = navigator.userActivation;
    if (!ua) return null;
    return { isActive: ua.isActive, hasBeenActive: ua.hasBeenActive };
  }

  const fmtAct = (a) => (a ? `${a.isActive}/${a.hasBeenActive}` : '无此API');

  function frameKind() {
    if (window.top === window) return 'top';
    try {
      return window.parent.location.origin === location.origin ? 'iframe同源' : 'iframe跨源';
    } catch {
      return 'iframe跨源';
    }
  }

  async function run(label, opts = {}) {
    const r = {
      label,
      href: location.href,
      referrer: document.referrer || '(空)',
      frame: frameKind(),
      actBefore: activation(),
    };

    // V5：模拟游戏加载慢，先干等再碰音频，看瞬时激活过没过期。
    if (opts.delayMs) {
      r.delayMs = opts.delayMs;
      await sleep(opts.delayMs);
      r.actAfterDelay = activation();
    }

    // (a) AudioContext 的 state，resume() 前后各读一次。
    let ctx = null;
    const Ctor = window.AudioContext || window.webkitAudioContext;
    if (!Ctor) {
      r.acError = '没有 AudioContext';
    } else {
      try {
        ctx = new Ctor();
        r.acStateNew = ctx.state;
        const settled = await within(
          ctx.resume().then(() => 'resolved', (e) => 'rejected:' + e.name),
          RESUME_TIMEOUT,
          `一直挂着(>${RESUME_TIMEOUT}ms 没 resolve 也没 reject)`
        );
        r.resume = settled;
        r.acStateAfterResume = ctx.state;
      } catch (e) {
        r.acError = e.name + ': ' + e.message;
      }
    }

    // (b) 真出 0.5 秒声音。判据不是 state 这个字符串，是 currentTime 走不走——
    // 挂起的上下文时钟是冻住的，这比读 state 更难骗。
    if (ctx) {
      try {
        const gain = ctx.createGain();
        gain.gain.value = 0.15;
        gain.connect(ctx.destination);
        const osc = ctx.createOscillator();
        osc.type = 'sine';
        osc.frequency.value = 660;
        osc.connect(gain);
        osc.start();
        osc.stop(ctx.currentTime + 0.5);
        // 第一次用声卡有冷启动，只采样一次会把「刚开始渲染」误判成「被挡住」，
        // 所以是等它走起来，最多等 1.5 秒。
        const t0 = ctx.currentTime;
        const until = Date.now() + 1500;
        let advance = 0;
        while (Date.now() < until && advance <= 0.05) {
          await sleep(50);
          advance = ctx.currentTime - t0;
        }
        r.clockAdvance = +advance.toFixed(3);
        r.clockWaitedMs = 1500 - (until - Date.now());
        r.acStateAfterOsc = ctx.state;
        r.oscHeard = ctx.state === 'running' && advance > 0.05;
      } catch (e) {
        r.oscError = e.name + ': ' + e.message;
        r.oscHeard = false;
      }
    } else {
      r.oscHeard = false;
    }

    // (c) <audio> 元素的 play()：resolve 还是 NotAllowedError。
    // resolve 不等于真在放——有的浏览器先答应下来，元数据到了发现有声再悄悄暂停，
    // 所以后面还要看 currentTime 有没有往前走。
    try {
      const el = document.createElement('audio');
      el.src = toneDataUrl();
      el.setAttribute('playsinline', '');
      document.body.appendChild(el);
      r.audioPlay = await within(
        el.play().then(() => 'ok', (e) => e.name || String(e)),
        RESUME_TIMEOUT,
        `一直挂着(>${RESUME_TIMEOUT}ms)`
      );
      await sleep(200);
      r.audioAdvanced = +el.currentTime.toFixed(3);
      r.audioPaused = el.paused;
      r.audioHeard = r.audioPlay === 'ok' && !el.paused && el.currentTime > 0;
    } catch (e) {
      r.audioPlay = e.name || String(e);
      r.audioHeard = false;
    }

    r.actAfter = activation();

    if (opts.bonus) r.bonus = await bonus();

    r.title =
      `[${label}] ac=${r.acStateNew || '-'}→${r.acStateAfterResume || '-'}(resume ${r.resume || '-'}) ` +
      `osc=${r.oscHeard ? '响' : '哑'}(${r.acStateAfterOsc || '-'},+${r.clockAdvance ?? 0}s) ` +
      `audio=${r.audioPlay}/${r.audioHeard ? '响' : '哑'} act=${fmtAct(r.actBefore)}→${fmtAct(r.actAfter)} ` +
      `frame=${r.frame} ref=${r.referrer}`;

    publish(r);
    return r;
  }

  // 附带观察：全屏与横屏锁定，和音频一样吃用户手势。
  async function bonus() {
    const b = {};
    try {
      await document.documentElement.requestFullscreen();
      b.fullscreen = 'ok';
    } catch (e) {
      b.fullscreen = (e && e.name ? e.name + ': ' : '') + (e && e.message ? e.message : String(e));
    }
    try {
      if (!screen.orientation || !screen.orientation.lock) {
        b.orientation = '无此 API';
      } else {
        await screen.orientation.lock('landscape');
        b.orientation = 'ok';
      }
    } catch (e) {
      b.orientation = (e && e.name ? e.name + ': ' : '') + (e && e.message ? e.message : String(e));
    }
    // 别把后面的变体留在全屏里。
    try {
      if (document.fullscreenElement) await document.exitFullscreen();
    } catch {
      /* 退不出去就算了，这里只是收尾 */
    }
    return b;
  }

  function publish(r) {
    window.__probeResult = r;
    document.title = r.title;
    const box = document.getElementById('out') || document.body;
    const pre = document.createElement('pre');
    pre.className = 'probe';
    pre.textContent = [
      r.title,
      '',
      `(a) new AudioContext().state = ${r.acStateNew}；resume() ${r.resume}；之后 state = ${r.acStateAfterResume}`,
      `(b) OscillatorNode 0.5s：${r.oscHeard ? '真的在响' : '哑的'}` +
        `（出声后 state=${r.acStateAfterOsc}，等了 ${r.clockWaitedMs}ms，currentTime 走了 ${r.clockAdvance}s）` +
        (r.oscError ? `；报错 ${r.oscError}` : ''),
      `(c) <audio>.play() = ${r.audioPlay}；200ms 后 ${r.audioHeard ? '真的在放' : '哑的'}` +
        `（paused=${r.audioPaused}，currentTime=${r.audioAdvanced}）`,
      `(d) navigator.userActivation：探针开始时 ${fmtAct(r.actBefore)}，结束时 ${fmtAct(r.actAfter)}` +
        (r.actAfterDelay ? `，干等 ${r.delayMs}ms 之后 ${fmtAct(r.actAfterDelay)}` : ''),
      '',
      `document.referrer = ${r.referrer}`,
      `所处上下文 = ${r.frame}`,
      `location = ${r.href}`,
      r.bonus ? `附带观察：requestFullscreen = ${r.bonus.fullscreen}；orientation.lock = ${r.bonus.orientation}` : '',
    ]
      .filter(Boolean)
      .join('\n');
    box.appendChild(pre);
    if (window.top !== window) {
      try {
        window.parent.postMessage({ __probe: r }, '*');
      } catch {
        /* 跨源也能 postMessage，这里只是保险 */
      }
    }
    // 结果自己送回服务器。跑批的那一侧只从服务器取，全程不碰 page.evaluate。
    try {
      fetch('/report', {
        method: 'POST',
        headers: { 'content-type': 'application/json' },
        body: JSON.stringify(r),
        keepalive: true,
      });
    } catch {
      /* 取不回去也不影响页面上那份 */
    }
  }

  window.gateAudioProbe = { run };
})();

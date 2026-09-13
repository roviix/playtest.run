(() => {
  const page = document.querySelector('body.media-page');
  if (!page) return;
  const panel = document.getElementById('chat-panel');
  const focus = document.querySelector('.media-focus');
  const narrow = matchMedia('(max-width:1099px)');
  let previousFocus;
  let previousScroll = 0;
  let pushedFeedback = false;
  const isOverlay = () => panel && narrow.matches && (page.classList.contains('feedback-open') || location.hash === '#chat-panel');
  const closeFeedback = () => {
    page.classList.remove('feedback-open');
    panel?.removeAttribute('role');
    panel?.removeAttribute('aria-modal');
    document.querySelector('main.media-card')?.removeAttribute('inert');
    previousFocus?.focus({ preventScroll: true });
    window.scrollTo(0, previousScroll);
  };
  const openFeedback = () => {
    previousFocus = document.activeElement;
    previousScroll = window.scrollY;
    page.classList.remove('media-focused');
    if (focus) { focus.setAttribute('aria-pressed', 'false'); focus.textContent = page.classList.contains('article-page') ? '专注阅读' : '专注观看'; }
    if (narrow.matches) {
      page.classList.add('feedback-open');
      panel.setAttribute('role', 'dialog');
      panel.setAttribute('aria-modal', 'true');
      document.querySelector('main.media-card')?.setAttribute('inert', '');
      if (location.hash !== '#chat-panel') { history.pushState(null, '', '#chat-panel'); pushedFeedback = true; }
    }
    panel.querySelector('.chat-input')?.focus({ preventScroll: true });
  };
  if (panel) {
    document.querySelectorAll('a[href="#chat-panel"]').forEach(link => link.addEventListener('click', event => { event.preventDefault(); openFeedback(); }));
    const dismiss = () => {
      closeFeedback();
      if (pushedFeedback) { pushedFeedback = false; history.back(); }
      else if (location.hash === '#chat-panel') history.replaceState(null, '', location.pathname + location.search);
    };
    panel.querySelector('.chat-back')?.addEventListener('click', event => { event.preventDefault(); dismiss(); });
    window.addEventListener('popstate', () => { if (location.hash !== '#chat-panel') { pushedFeedback = false; closeFeedback(); } else openFeedback(); });
    panel.addEventListener('keydown', event => {
      if (!isOverlay()) return;
      if (event.key === 'Escape') { event.preventDefault(); dismiss(); }
      if (event.key === 'Tab') {
        const controls = [...panel.querySelectorAll('a,button,input,textarea')].filter(control => !control.disabled && control.getClientRects().length);
        const first = controls[0];
        const last = controls.at(-1);
        if (event.shiftKey && document.activeElement === first) { event.preventDefault(); last?.focus(); }
        else if (!event.shiftKey && document.activeElement === last) { event.preventDefault(); first?.focus(); }
      }
    });
    narrow.addEventListener('change', () => { if (!narrow.matches && isOverlay()) closeFeedback(); else if (!narrow.matches) { page.classList.remove('feedback-open'); panel.removeAttribute('role'); panel.removeAttribute('aria-modal'); document.querySelector('main.media-card')?.removeAttribute('inert'); } });
    if (location.hash === '#chat-panel') openFeedback();
  }
  if (focus) {
    focus.hidden = false;
    focus.addEventListener('click', () => {
      const focused = page.classList.toggle('media-focused');
      focus.setAttribute('aria-pressed', String(focused));
      focus.textContent = focused ? '展开反馈' : (page.classList.contains('article-page') ? '专注阅读' : '专注观看');
    });
  }
  const video = document.querySelector('.video-body video');
  if (video) {
    const status = document.querySelector('.video-status');
    const retry = document.querySelector('.video-retry');
    const fail = () => { status.hidden = false; status.textContent = '视频暂时无法播放，可能是网络或浏览器格式支持问题。可以重新加载后再试。'; retry.hidden = false; };
    video.addEventListener('error', fail);
    video.addEventListener('waiting', () => { status.hidden = false; status.textContent = '正在缓冲视频…'; });
    for (const event of ['canplay', 'playing']) video.addEventListener(event, () => { status.hidden = true; retry.hidden = true; });
    retry.textContent = '重新加载视频';
    retry.addEventListener('click', () => { retry.hidden = true; status.hidden = false; status.textContent = '正在重新加载，请稍候…'; video.load(); });
    if (video.error) fail();
  }
  const article = document.querySelector('.article-body[data-reader-slug]');
  if (!article) return;
  article.querySelectorAll('img').forEach(image => {
    const fail = () => { if (image.hidden) return; image.hidden = true; const message = document.createElement('p'); message.className = 'image-status'; message.textContent = image.alt ? `图片暂时无法加载：${image.alt}` : '这张图片暂时无法加载，其他正文仍可继续阅读。'; image.after(message); };
    image.addEventListener('error', fail);
    if (image.complete && !image.naturalWidth) fail();
  });
  const resume = document.querySelector('.reader-resume');
  if (!resume) return;
  const continueButton = resume.querySelector('.reader-continue-btn, .reader-continue');
  const clearButton = resume.querySelector('.reader-clear-btn, .reader-clear');
  const chapterId = article.dataset.readerChapter;
  const key = chapterId ? `pt-reading:${article.dataset.readerSlug}:${chapterId}` : `pt-reading:${article.dataset.readerSlug}`;
  const version = Number(article.dataset.readerVersion);
  let stored;
  let storage;
  let saveEnabled = false;
  let timer;
  try {
    storage = window.localStorage;
    const value = storage.getItem(key);
    if (value) {
      try { stored = JSON.parse(value); } catch { storage.removeItem(key); }
      if (stored && (!Number.isInteger(stored.version) || typeof stored.anchor !== 'string' || !/^(article-content|section-[a-f0-9]{16}-\d+)$/.test(stored.anchor) || !Number.isFinite(stored.offset) || stored.offset < 0 || stored.offset > 20000 || !Number.isFinite(stored.at) || Date.now() - stored.at > 90 * 86400000)) { stored = null; storage.removeItem(key); }
    }
  } catch {}
  const target = stored && document.getElementById(stored.anchor);
  if (stored && storage) {
    const compatible = target && (target === article || article.contains(target)) && (stored.version === version || stored.anchor !== 'article-content');
    if (compatible && !location.hash) {
      resume.hidden = false;
      if (continueButton) continueButton.hidden = false;
      if (clearButton) clearButton.hidden = false;
    }
  }
  continueButton?.addEventListener('click', () => {
    if (!target) return;
    saveEnabled = true;
    const offset = stored.version === version ? stored.offset : 0;
    const top = window.scrollY + target.getBoundingClientRect().top - 124 + offset;
    window.scrollTo({ top, behavior: 'instant' });
    target.setAttribute('tabindex', '-1'); target.focus({ preventScroll: true });
    resume.hidden = true;
  });
  clearButton?.addEventListener('click', () => {
    try { storage?.removeItem(key); } catch {}
    stored = null; saveEnabled = false; clearTimeout(timer);
    resume.hidden = true;
  });
  const save = () => {
    if (!storage || !saveEnabled || isOverlay() || 124 - article.getBoundingClientRect().top < 120) return;
    let anchor = article;
    for (const heading of article.querySelectorAll('h1[id],h2[id],h3[id],h4[id],h5[id],h6[id]')) if (heading.getBoundingClientRect().top <= 124) anchor = heading;
    const position = { version, chapterId, anchor: anchor.id, offset: Math.min(20000, Math.max(0, 124 - anchor.getBoundingClientRect().top)), at: Date.now() };
    try {
      storage.setItem(key, JSON.stringify(position));
      if (chapterId) storage.setItem(`pt-reading:${article.dataset.readerSlug}:last-chapter`, chapterId);
      clearButton.hidden = false;
    } catch { storageFailed(); }
  };
  for (const event of ['wheel', 'touchmove']) window.addEventListener(event, () => { saveEnabled = true; }, { passive: true });
  window.addEventListener('keydown', event => { if (['ArrowDown', 'PageDown', 'End', ' '].includes(event.key) && !event.target.closest('input,textarea,button')) saveEnabled = true; });
  document.querySelector('.article-toc')?.addEventListener('click', event => { if (event.target.closest('a')) saveEnabled = true; });
  window.addEventListener('scroll', () => { clearTimeout(timer); timer = setTimeout(save, 250); }, { passive: true });
  window.addEventListener('pagehide', save);
})();

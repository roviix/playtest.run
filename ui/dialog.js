(() => {
  document.querySelectorAll('dialog').forEach(dialog => {
    if (!dialog.showModal) return;
    const hash = '#' + dialog.id;
    const triggers = Array.from(document.querySelectorAll('a[href], [data-dialog]')).filter(link => link.getAttribute('href') === hash || link.dataset.dialog === dialog.id);
    let trigger;
    const open = () => {
      if (dialog.open) return;
      trigger = document.activeElement === document.body ? triggers[0] : document.activeElement;
      document.querySelectorAll("dialog[open]").forEach(other => { if (other !== dialog) other.close(); });
      dialog.showModal();
    };
    triggers.forEach(link => link.addEventListener('click', event => { event.preventDefault(); open(); }));
    dialog.querySelectorAll('[data-close-dialog], a[href="#"]').forEach(link => link.addEventListener('click', event => { event.preventDefault(); dialog.close(); }));
    dialog.addEventListener('click', event => {
      const rect = dialog.getBoundingClientRect();
      if (event.target === dialog && (event.clientX < rect.left || event.clientX > rect.right || event.clientY < rect.top || event.clientY > rect.bottom)) dialog.close();
    });
    dialog.addEventListener('cancel', event => { event.preventDefault(); dialog.close(); });
    dialog.addEventListener('close', () => {
      if (location.hash === hash) history.replaceState(null, '', location.pathname + location.search);
      if (document.querySelector('dialog[open]')) return;
      const parent = trigger?.closest('dialog');
      if (parent && !parent.open) parent.showModal();
      trigger?.focus();
    });
    if (location.hash === hash) open();
  });
  const chatForm = document.getElementById('chat-form');
  if (chatForm) {
    const stickers = document.getElementById('chat-stickers');
    if (stickers) {
      stickers.addEventListener('click', e => {
        const btn = e.target.closest('.sticker-btn');
        if (!btn) return;
        const sticker = btn.dataset.sticker;
        const input = chatForm.querySelector('input[name="feedback"]');
        if (input) {
          if (!input.value.trim()) {
            input.value = sticker;
            chatForm.requestSubmit ? chatForm.requestSubmit() : chatForm.dispatchEvent(new Event('submit', { cancelable: true }));
          } else {
            input.value = input.value.trim() + ' ' + sticker;
            input.focus();
          }
        }
      });
    }
    chatForm.addEventListener('submit', async function(e) {
      e.preventDefault();
      const input = this.querySelector('input[name="feedback"]');
      const text = input?.value.trim();
      if (!text) return;
      const stream = document.getElementById('chat-stream');
      const empty = stream?.querySelector('.chat-empty');
      if (empty) empty.remove();
      const isSticker = /^(?:🎮|🎨|🎵|💡|🐛|☕|📚|🧠|🔥|✨|🔍|💭|🎬|🍿|👏)/u.test(text);
      const bubbleClass = isSticker ? 'chat-bubble sticker-bubble' : 'chat-bubble';
      const msg = document.createElement('div');
      msg.className = 'chat-msg';
      msg.innerHTML = '<div class="chat-avatar">我</div><div class="chat-content"><div class="voice"><p class="' + bubbleClass + '">「' + text.replace(/&/g,'&amp;').replace(/</g,'&lt;').replace(/>/g,'&gt;') + '」<cite>刚刚 · 已送达</cite></p></div></div>';
      stream?.appendChild(msg);
      stream?.scrollTo({ top: stream.scrollHeight, behavior: 'smooth' });
      input.value = '';
      try {
        await fetch(this.action || window.location.href, {
          method: 'POST',
          body: new URLSearchParams(new FormData(this))
        });
      } catch {}
    });
  }
})();

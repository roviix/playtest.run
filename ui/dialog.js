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
    const stream = document.getElementById('chat-stream');
    const stickers = document.getElementById('chat-stickers');
    const emojiToggle = document.getElementById('chat-emoji-toggle');
    const emojiPopover = document.getElementById('chat-emoji-popover');
    const feedbackStatus = document.querySelector('.feedback-status');
    const isSticker = text => /^(?:🎮|🎨|🎵|💡|🐛|☕|📚|🧠|🔥|✨|🔍|💭|🎬|🍿|👏)/u.test(text);

    const esc = s => String(s || '').replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;').replace(/"/g, '&quot;');

    const SELF_STORAGE_KEY = 'pt_self_chat';
    const getSelfKeys = () => {
      try {
        return new Set(JSON.parse(sessionStorage.getItem(SELF_STORAGE_KEY) || '[]'));
      } catch {
        return new Set();
      }
    };
    const addSelfKey = key => {
      try {
        const set = getSelfKeys();
        set.add(key);
        sessionStorage.setItem(SELF_STORAGE_KEY, JSON.stringify([...set].slice(-50)));
      } catch {}
    };

    const seenKeys = new Set();
    const selfKeys = getSelfKeys();
    document.querySelectorAll('.chat-msg').forEach(msg => {
      const key = msg.dataset.msgKey;
      if (key) {
        seenKeys.add(key);
        if (selfKeys.has(key)) {
          msg.classList.add('self');
          const author = msg.querySelector('.chat-author');
          if (author) author.textContent = '我';
          const avatar = msg.querySelector('.chat-avatar');
          if (avatar) avatar.textContent = '我';
        }
      }
    });

    const appendMessage = ({ who, version, time, text, avatar, isSelf }) => {
      const key = `${who}:${version}:${text}`;
      if (seenKeys.has(key)) return false;
      seenKeys.add(key);

      const empty = stream?.querySelector('.chat-empty');
      if (empty) empty.remove();

      const bubbleClass = isSticker(text) ? 'chat-bubble sticker-bubble' : 'chat-bubble';
      const msg = document.createElement('div');
      msg.className = isSelf ? 'chat-msg self' : 'chat-msg';
      msg.dataset.msgKey = key;

      const avatarHtml = isSelf ? '我' : (avatar || `<div class="chat-avatar-char">${esc((who || '?').slice(0, 1))}</div>`);
      const authorText = isSelf ? '我' : esc(who);
      const versionBadge = version ? `<span class="chat-badge">v${version}</span>` : '';
      const timeText = esc(time || '刚刚');

      msg.innerHTML = `<div class="chat-avatar">${avatarHtml}</div><div class="chat-content voice"><div class="chat-meta"><span class="chat-author">${authorText}</span>${versionBadge}<span class="chat-time">${timeText}</span></div><p class="${bubbleClass}">${esc(text)}</p></div>`;

      stream?.appendChild(msg);
      stream?.scrollTo({ top: stream.scrollHeight, behavior: 'smooth' });
      return true;
    };

    if (emojiToggle && emojiPopover) {
      emojiToggle.addEventListener('click', e => {
        e.stopPropagation();
        emojiPopover.hidden = !emojiPopover.hidden;
      });

      emojiPopover.addEventListener('click', e => {
        const btn = e.target.closest('button');
        if (!btn) return;
        e.stopPropagation();
        const emoji = btn.dataset.emoji || btn.textContent.trim();
        const input = chatForm.querySelector('input[name="feedback"]');
        if (input && emoji) {
          const start = input.selectionStart ?? input.value.length;
          const end = input.selectionEnd ?? input.value.length;
          const val = input.value;
          input.value = val.slice(0, start) + emoji + val.slice(end);
          const newPos = start + emoji.length;
          input.setSelectionRange(newPos, newPos);
          input.focus();
        }
        emojiPopover.hidden = true;
      });

      document.addEventListener('click', e => {
        if (!emojiPopover.hidden && !emojiPopover.contains(e.target) && e.target !== emojiToggle) {
          emojiPopover.hidden = true;
        }
      });

      document.addEventListener('keydown', e => {
        if (e.key === 'Escape' && !emojiPopover.hidden) {
          emojiPopover.hidden = true;
        }
      });
    }

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

      if (emojiPopover) emojiPopover.hidden = true;

      const stampEl = document.querySelector('.stamp');
      const verMatch = stampEl ? stampEl.textContent.match(/v(\d+)/) : null;
      const version = verMatch ? parseInt(verMatch[1], 10) : '';

      const key = `我:${version}:${text}`;
      addSelfKey(key);

      appendMessage({
        who: '我',
        version,
        time: '刚刚',
        text,
        avatar: '我',
        isSelf: true,
      });

      const countEl = document.querySelector('.chat-count');
      if (countEl) {
        const current = parseInt(countEl.textContent, 10) || 0;
        countEl.textContent = current + 1;
      }

      const formData = new FormData(this);
      input.value = '';

      try {
        const actionUrl = this.getAttribute('action') || window.location.href;
        const resp = await fetch(actionUrl, {
          method: 'POST',
          headers: { 'Accept': 'application/json' },
          body: new URLSearchParams(formData)
        });
        if (!resp.ok) {
          if (feedbackStatus) {
            feedbackStatus.textContent = '发送未成功，请稍后重试';
            feedbackStatus.hidden = false;
          }
        } else {
          if (feedbackStatus) feedbackStatus.hidden = true;
        }
      } catch {
        if (feedbackStatus) {
          feedbackStatus.textContent = '网络中断，请稍后重试';
          feedbackStatus.hidden = false;
        }
      }
    });

    let polling = false;
    const pollChat = async () => {
      if (document.visibilityState !== 'visible' || polling) return;
      polling = true;
      try {
        const pollUrl = (chatForm.action || window.location.pathname) + '?chat=1';
        const resp = await fetch(pollUrl, {
          headers: { 'Accept': 'application/json' },
          cache: 'no-store'
        });
        if (resp.ok) {
          const data = await resp.json();
          if (data.ok && Array.isArray(data.messages)) {
            const currentSelfKeys = getSelfKeys();
            for (const m of data.messages) {
              const msgKey = `${m.who}:${m.version}:${m.text}`;
              const isSelf = currentSelfKeys.has(msgKey) || currentSelfKeys.has(`我:${m.version}:${m.text}`);
              appendMessage({
                who: isSelf ? '我' : m.who,
                version: m.version,
                time: m.time,
                text: m.text,
                avatar: isSelf ? '我' : m.avatar,
                isSelf
              });
            }
            if (data.count !== undefined) {
              const countEl = document.querySelector('.chat-count');
              if (countEl) countEl.textContent = data.count;
            }
          }
        }
      } catch {}
      finally {
        polling = false;
      }
    };

    setInterval(pollChat, 3000);
  }
})();

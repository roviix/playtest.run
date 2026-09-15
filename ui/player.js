(() => {
  const dialog = document.querySelector('#publish-dialog');
  const status = dialog?.querySelector('[role="status"]');
  let copyGeneration = 0;
  const resetCopy = (copy) => {
    copyGeneration++;
    if (status) status.textContent = '';
    if (copy) {
      delete copy.dataset.copied;
      copy.setAttribute('aria-label', 'Copy command');
      copy.title = 'Copy command';
      const ct = copy.querySelector('.copy-text, .hero-copy-text');
      if (ct) ct.textContent = 'Copy';
    }
  };
  if (dialog) {
    dialog.addEventListener('close', () => {
      dialog.querySelectorAll('[data-copy-command], [data-hero-copy]').forEach(resetCopy);
    });
    dialog.querySelectorAll('input[type="radio"]').forEach(input => input.addEventListener('change', () => {
      dialog.querySelectorAll('[data-copy-command], [data-hero-copy]').forEach(resetCopy);
    }));
  }
  document.querySelectorAll('[data-copy-command], [data-hero-copy]').forEach(copy => {
    copy.hidden = false;
    copy.addEventListener('click', async () => {
      const generation = copyGeneration;
      const copyText = copy.querySelector('.copy-text, .hero-copy-text');
      let text = copy.dataset.copyText;
      if (!text && dialog) {
        const panel = Array.from(dialog.querySelectorAll('.codebox')).find(item => getComputedStyle(item).display !== 'none');
        text = panel?.dataset.copyText || panel?.querySelector('code')?.textContent;
      }
      text = text || '';
      const succeed = () => {
        if (generation !== copyGeneration) return;
        copy.dataset.copied = 'true';
        copy.setAttribute('aria-label', 'Command copied');
        copy.title = 'Copied';
        if (copyText) copyText.textContent = 'Copied';
        if (status) {
          status.dataset.state = 'success';
          status.textContent = 'Command copied.';
        }
      };
      try {
        if (navigator.clipboard && navigator.clipboard.writeText) {
          await navigator.clipboard.writeText(text);
          succeed();
        } else {
          throw new Error('no clipboard API');
        }
      } catch {
        try {
          const ta = document.createElement('textarea');
          ta.value = text;
          ta.style.position = 'fixed';
          ta.style.opacity = '0';
          document.body.appendChild(ta);
          ta.select();
          document.execCommand('copy');
          ta.remove();
          succeed();
        } catch {
          if (generation !== copyGeneration) return;
          resetCopy(copy);
          if (status) {
            status.dataset.state = 'error';
            status.textContent = "Couldn't copy. Select the command and copy it manually.";
          }
        }
      }
    });
  });
})();

(() => {
  window.addEventListener('keydown', (e) => {
    if (e.key === '/' && !['INPUT', 'TEXTAREA', 'SELECT'].includes(document.activeElement?.tagName)) {
      const searchInput = document.querySelector('.discover-search input[name="q"]');
      if (searchInput) {
        e.preventDefault();
        searchInput.focus();
        searchInput.select();
      }
    }
  });
})();

(() => {
  const form = document.querySelector('form.start');
  if (!form) return;
  const btn = form.querySelector('.start-btn');

  const reportStart = () => {
    try {
      const data = new FormData(form);
      const action = form.action || form.getAttribute('action');
      if (!action) return;
      if (navigator.sendBeacon) {
        navigator.sendBeacon(action, data);
      } else {
        fetch(action, { method: 'POST', body: data, keepalive: true }).catch(() => {});
      }
    } catch (_) {}
  };

  if (btn) {
    btn.addEventListener('click', () => {
      reportStart();
    });
  }

  form.addEventListener('submit', (e) => {
    e.preventDefault();
    reportStart();
    const targetUrl = form.dataset.targetUrl || btn?.getAttribute('href');
    if (targetUrl) {
      window.open(targetUrl, '_blank', 'noopener');
    }
  });
})();

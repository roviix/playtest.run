(() => {
  const dialog = document.querySelector('#publish-dialog');
  if (!dialog || !dialog.showModal) return;
  const copy = dialog.querySelector('[data-copy-command]');
  const copyText = copy?.querySelector('.copy-text');
  const status = dialog.querySelector('[role="status"]');
  let copyGeneration = 0;
  const resetCopy = () => {
    copyGeneration++;
    if (status) status.textContent = '';
    if (copy) {
      delete copy.dataset.copied;
      copy.setAttribute('aria-label', '复制命令');
      copy.title = '复制命令';
      if (copyText) copyText.textContent = '复制';
    }
  };
  dialog.addEventListener('close', resetCopy);
  if (copy) {
    copy.hidden = false;
    copy.addEventListener('click', async () => {
      const generation = copyGeneration;
      const panel = Array.from(dialog.querySelectorAll('.codebox')).find(item => getComputedStyle(item).display !== 'none');
      try {
        await navigator.clipboard.writeText(panel.querySelector('code').textContent);
        if (generation !== copyGeneration) return;
        copy.dataset.copied = 'true';
        copy.setAttribute('aria-label', '已复制命令');
        copy.title = '已复制';
        if (copyText) copyText.textContent = '已复制';
        status.dataset.state = 'success';
        status.textContent = '已复制命令。';
      } catch {
        if (generation !== copyGeneration) return;
        resetCopy();
        status.dataset.state = 'error';
        status.textContent = '复制失败，请选中命令手动复制。';
      }
    });
  }
  dialog.querySelectorAll('input[type="radio"]').forEach(input => input.addEventListener('change', () => {
    resetCopy();
  }));
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

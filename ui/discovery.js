(() => {
  const status = document.querySelector('[data-collection-status]');
  const prompt = document.querySelector('#challenge-prompt');
  document.querySelectorAll('[data-copy-prompt]').forEach(button => {
    button.hidden = false;
    button.addEventListener('click', async () => {
      try {
        await navigator.clipboard.writeText(prompt.textContent);
        status.textContent = '题目已复制，随时可以开始创作。';
      } catch {
        status.textContent = '无法自动复制，请选中题目手动复制。';
      }
    });
  });
  document.querySelectorAll('[data-share-collection]').forEach(button => {
    button.hidden = false;
    button.addEventListener('click', async () => {
      const url = new URL(location.pathname, location.origin).href;
      try {
        if (navigator.share) await navigator.share({ title: document.title, url });
        else {
          await navigator.clipboard.writeText(url);
          status.textContent = '合集链接已复制，可以发给朋友。';
        }
      } catch (error) {
        if (error.name !== 'AbortError') status.textContent = '无法自动分享，请复制浏览器地址栏里的链接。';
      }
    });
  });
  document.querySelectorAll('a[href="#participate"]').forEach(link => link.addEventListener('click', () => {
    const details = document.getElementById('participate');
    if (details) details.open = true;
  }));
})();

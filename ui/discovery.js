(() => {
  const status = document.querySelector('[data-collection-status]');
  const prompt = document.querySelector('#challenge-prompt');
  document.querySelectorAll('[data-copy-prompt]').forEach(button => {
    button.hidden = false;
    button.addEventListener('click', async () => {
      try {
        await navigator.clipboard.writeText(prompt.textContent);
        status.textContent = 'Prompt copied. Start whenever you like.';
      } catch {
        status.textContent = "Couldn't copy. Select the prompt and copy it manually.";
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
          status.textContent = 'Collection link copied. Send it to a friend.';
        }
      } catch (error) {
        if (error.name !== 'AbortError') status.textContent = "Couldn't share. Copy the link from the address bar.";
      }
    });
  });
  document.querySelectorAll('a[href="#participate"]').forEach(link => link.addEventListener('click', () => {
    const details = document.getElementById('participate');
    if (details) details.open = true;
  }));
})();

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
})();

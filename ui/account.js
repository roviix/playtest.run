(() => {
  const dialog = document.getElementById('account-login');
  if (!dialog) return;
  const form = dialog.querySelector('[data-account-email]');
  const github = dialog.querySelector('[data-account-github]');
  const message = dialog.querySelector('[data-account-message]');
  let identity;
  let pending = location.pathname + location.search;
  let trigger;
  let sending = false;
  const load = async () => {
    const response = await fetch('/v1/account', { credentials: 'same-origin' });
    if (!response.ok) throw new Error("Can't connect right now. Try again.");
    identity = await response.json();
    form.hidden = !identity.email_available;
    github.hidden = !identity.github_available;
    if (!identity.email_available && !identity.github_available) message.textContent = 'Sign-in is unavailable. Projects still open without it.';
    document.querySelectorAll('[data-account-login]').forEach(link => {
      if (identity.account && link.classList.contains('account')) {
        link.querySelector('.account-name').textContent = identity.account.me.display_name;
        link.href = '/console/#/token';
      }
    });
  };
  void load().catch(() => {});
  const open = async (element, returnTo) => {
    trigger = element;
    pending = returnTo;
    message.textContent = '';
    github.href = '/v1/login/github/start?return_to=' + encodeURIComponent(pending);
    document.querySelectorAll('dialog[open]').forEach(other => { if (other !== dialog) other.close(); });
    if (!dialog.open) dialog.showModal();
    form.querySelector('input').focus();
    if (!identity) {
      form.hidden = true;
      github.hidden = true;
      message.textContent = 'Loading sign-in options…';
      try { await load(); if (identity.email_available || identity.github_available) message.textContent = ''; }
      catch (error) { message.textContent = error.message; }
    }
  };
  document.querySelectorAll('[data-account-login], [data-manage]').forEach(link => {
    link.addEventListener('click', event => {
      if (event.button !== 0 || event.metaKey || event.ctrlKey || event.shiftKey || event.altKey) return;
      if (identity?.account) return;
      event.preventDefault();
      const returnTo = link.hasAttribute('data-manage') ? new URL(link.href).pathname + new URL(link.href).hash : location.pathname + location.search;
      void open(link, returnTo);
    });
  });
  dialog.addEventListener('close', () => trigger?.focus());
  form.addEventListener('submit', async event => {
    event.preventDefault();
    if (sending) return;
    sending = true;
    const button = form.querySelector('button');
    button.disabled = true;
    button.textContent = 'Sending…';
    message.textContent = '';
    try {
      const response = await fetch('/v1/account/email', { method: 'POST', credentials: 'same-origin', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify({ email: form.elements.email.value, return_to: pending }) });
      const result = await response.json();
      if (!response.ok) throw new Error(result.message || "Couldn't send. Try again.");
      message.textContent = result.message;
      button.textContent = 'Send again';
    } catch (error) { message.textContent = error.message || "Couldn't send. Try again."; button.textContent = 'Retry'; }
    finally { sending = false; button.disabled = false; }
  });
})();

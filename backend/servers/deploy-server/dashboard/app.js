// backend/servers/deploy-server/dashboard/app.js
//
// Token handling notes (security):
// - The bearer token is held only in this module's closure (memory). We do NOT
//   persist to sessionStorage/localStorage — that minimizes the blast radius if
//   any future XSS reaches /dashboard. Cost: refresh = re-prompt for the token.
// - prompt() returns null on Cancel and "" on empty input. Both abort with a
//   visible error rather than firing requests with `Bearer null` / `Bearer `.
const TOKEN = (() => {
  const t = window.prompt('API token:');
  return t && t.trim() ? t.trim() : null;
})();

function escape(s) { return String(s ?? '').replace(/[&<>"']/g, c => ({'&':'&amp;','<':'&lt;','>':'&gt;','"':'&quot;',"'":'&#39;'}[c])); }

function showError(msg) {
  const el = document.getElementById('lastrefresh');
  if (el) el.textContent = `Error: ${msg}`;
}

if (!TOKEN) {
  showError('No API token provided — refresh and enter a token to continue.');
} else {
  const auth = { 'Authorization': `Bearer ${TOKEN}` };

  async function load() {
    try {
      const wts = await fetch('/api/worktrees', { headers: auth }).then(r => r.json());
      document.querySelector('#worktrees tbody').innerHTML = (Array.isArray(wts) ? wts : []).map(w => `
        <tr>
          <td><code>${escape(w.name)}</code></td>
          <td>${escape(w.branch)}</td>
          <td>${escape(w.state)}</td>
          <td>${escape(w.backend_mode)}</td>
          <td>${(w.urls && w.urls.ppt) ? `<a href="${escape(w.urls.ppt)}" target="_blank" rel="noopener noreferrer">${escape(w.urls.ppt)}</a>` : ''}</td>
          <td>${escape(w.created_at)}</td>
        </tr>
      `).join('');

      const audit = await fetch('/api/audit?limit=100', { headers: auth }).then(r => r.json());
      document.querySelector('#audit tbody').innerHTML = (Array.isArray(audit) ? audit : []).map(a => `
        <tr>
          <td>${new Date(a.ts * 1000).toLocaleString()}</td>
          <td><code>${escape(a.caller_kind)}:${escape(a.caller_id)}</code></td>
          <td>${escape(a.endpoint)}</td>
          <td>${escape(a.result)}</td>
          <td>${a.duration_ms ?? ''}</td>
        </tr>
      `).join('');

      document.getElementById('lastrefresh').textContent = `Refreshed ${new Date().toLocaleTimeString()}`;
    } catch (e) {
      showError(e.message);
    }
  }

  load();
  setInterval(load, 30000);
}

// backend/servers/deploy-server/dashboard/app.js
const TOKEN = sessionStorage.getItem('ppt-deploy-token') || prompt('API token:');
sessionStorage.setItem('ppt-deploy-token', TOKEN);
const auth = { 'Authorization': `Bearer ${TOKEN}` };

function escape(s) { return String(s ?? '').replace(/[&<>"']/g, c => ({'&':'&amp;','<':'&lt;','>':'&gt;','"':'&quot;',"'":'&#39;'}[c])); }

async function load() {
  try {
    const wts = await fetch('/api/worktrees', { headers: auth }).then(r => r.json());
    document.querySelector('#worktrees tbody').innerHTML = (Array.isArray(wts) ? wts : []).map(w => `
      <tr>
        <td><code>${escape(w.name)}</code></td>
        <td>${escape(w.branch)}</td>
        <td>${escape(w.state)}</td>
        <td>${escape(w.backend_mode)}</td>
        <td>${(w.urls && w.urls.ppt) ? `<a href="${escape(w.urls.ppt)}" target="_blank">${escape(w.urls.ppt)}</a>` : ''}</td>
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
    document.getElementById('lastrefresh').textContent = `Error: ${e.message}`;
  }
}

load();
setInterval(load, 30000);

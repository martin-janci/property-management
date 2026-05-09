// @ts-nocheck — this file runs in the browser via esm.sh; not type-checked by tsc.

// Supply-chain note: imports below are pinned to specific versions. Browser
// ES module spec does not support SRI on `import` statements (only on `<script>`
// tags), so we cannot enforce integrity at the language level. To upgrade:
//   1. Bump the version pin below.
//   2. Run the SRI hash generation script in
//      docs/superpowers/plans/2026-05-08-screen-map-phase-3a.md Task 3 Step 1
//      to compute new hashes.
//   3. Update the `expected SRI` comment next to each import below.
//   4. (Optional) Add the integrity tag to `index.html`'s <script> shell when esm.sh
//      ships compatible bundle artifacts.
//
// Currently pinned:
// - preact@10.24.3 — expected SRI: sha384-/pFbBxO2jKQVyiPGNHNeHzL8Q4alRjcIErmT6JlDV+yLFBlNWgBelAz7anPGMNc+
// - preact@10.24.3/hooks — expected SRI: sha384-AYscPVYcC3uHOEqxBLdXyhBqOQu+DajjMpCRAKrSreyAXZj/DpB80GkmGcyVMdFN
// - htm@3.1.1 — expected SRI: sha384-sSiaM0en9Bz57PyCPaF5zeh8OaFdFmSQRw0ynngAWx8Kz51qoisBTGbQRBTrDa6G
import htm from 'https://esm.sh/htm@3.1.1';
import { h, render } from 'https://esm.sh/preact@10.24.3';
import { useEffect, useState } from 'https://esm.sh/preact@10.24.3/hooks';

const html = htm.bind(h);
const TOKEN = window.__SESSION_TOKEN__;

async function api(path, init) {
  const sep = path.includes('?') ? '&' : '?';
  const res = await fetch(`${path}${sep}session=${TOKEN}`, init);
  return res.json();
}

function App() {
  const [session, setSession] = useState(null);
  const [currentId, setCurrentId] = useState(null);
  useEffect(() => {
    api('/api/session').then((s) => {
      setSession(s);
      setCurrentId(s.screens[s.currentIdx]?.id ?? null);
    });
  }, []);
  if (!session) return html`<div class="topbar"><span>loading…</span></div>`;
  if (!currentId) return html`<div class="topbar"><span>review complete</span></div>`;
  return html`<${ScreenView}
    screenId=${currentId}
    total=${session.screens.length}
    onNext=${(nextId) => setCurrentId(nextId)}
  />`;
}

function ScreenView({ screenId, total, onNext }) {
  const [screen, setScreen] = useState(null);
  const [decisions, setDecisions] = useState({});
  const [generalNote, setGeneralNote] = useState('');
  const [previewMode, setPreviewMode] = useState('local');
  useEffect(() => {
    setDecisions({});
    setGeneralNote('');
    api(`/api/screens/${encodeURIComponent(screenId)}`).then(setScreen);
  }, [screenId]);
  if (!screen) return html`<div class="topbar"><span>loading screen…</span></div>`;
  const featureItems = parseChecklist(screen.body);
  async function saveAndNext() {
    const decArr = featureItems.map((f) => ({
      itemKey: f.key,
      ok: !!decisions[f.key]?.ok,
      note: decisions[f.key]?.note,
    }));
    const r = await api(`/api/screens/${encodeURIComponent(screenId)}/review`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ decisions: decArr, generalNote }),
    });
    if (r.done) {
      await api('/api/session/finish', { method: 'POST' });
      window.close();
    } else {
      onNext(r.nextScreenId);
    }
  }
  const previewSrc =
    previewMode === 'local'
      ? screen.previewUrls.local
      : previewMode === 'staging'
        ? screen.previewUrls.staging
        : null;
  return html`
    <div class="topbar">
      <span class="progress">${screen.frontmatter.id} (${screen.frontmatter.product})</span>
      <span>${total} screens total</span>
    </div>
    <div class="layout">
      <div class="left">
        <div class="metadata">${formatStatus(screen.frontmatter)}</div>
        <h3>Functionality</h3>
        ${featureItems.map((f) => html`<${ChecklistRow} key=${f.key} item=${f} state=${decisions[f.key]} onChange=${(s) => setDecisions((prev) => ({ ...prev, [f.key]: s }))} />`)}
        <h3>General note for this screen</h3>
        <textarea class="general-note" value=${generalNote} onInput=${(e) => setGeneralNote(e.currentTarget.value)} />
        <button class="save-btn" onClick=${saveAndNext}>Save & Next</button>
      </div>
      <div class="right">
        <div class="preview-toggle">
          <button class=${previewMode === 'local' ? 'active' : ''} onClick=${() => setPreviewMode('local')}>Local</button>
          <button class=${previewMode === 'staging' ? 'active' : ''} onClick=${() => setPreviewMode('staging')}>Staging</button>
        </div>
        <div class="preview-pane">
          ${previewSrc ? html`<iframe src=${previewSrc}></iframe>` : html`<p>(no preview URL for this screen)</p>`}
        </div>
      </div>
    </div>
  `;
}

function ChecklistRow({ item, state, onChange }) {
  const ok = state?.ok ?? false;
  const note = state?.note ?? '';
  const cls = ok ? 'checklist-row ok' : note ? 'checklist-row note' : 'checklist-row';
  return html`<div class=${cls}>
    <input type="checkbox" checked=${ok} onChange=${(e) => onChange({ ok: e.currentTarget.checked, note })} />
    <div style=${{ flex: 1 }}>
      <div>${item.label}</div>
      <textarea placeholder="optional note" value=${note} onInput=${(e) => onChange({ ok, note: e.currentTarget.value })} />
    </div>
  </div>`;
}

function parseChecklist(body) {
  const idx = body.indexOf('## Functionality Checklist');
  if (idx < 0) return [];
  const after = body.slice(idx).split(/\r?\n/);
  const items = [];
  for (const line of after) {
    const match = line.match(/^- \[([ x])\] (.+)$/);
    if (match) {
      const label = match[2];
      const key = label
        .toLowerCase()
        .replace(/[^a-z0-9]+/g, '-')
        .replace(/^-|-$/g, '');
      items.push({ key, label });
    }
    if (line.startsWith('## ') && !line.startsWith('## Functionality')) break;
  }
  return items;
}

function formatStatus(fm) {
  const parts = [];
  for (const [p, impl] of Object.entries(fm.implementations)) {
    parts.push(`${p}: ${impl.buildStatus} / ${impl.redesignStatus}`);
  }
  return parts.join(' • ');
}

render(html`<${App} />`, document.getElementById('root'));

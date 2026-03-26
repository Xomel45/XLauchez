// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

const { invoke } = window.__TAURI__.core;
const { listen  } = window.__TAURI__.event;

// ── State ────────────────────────────────────────────────────────────────────
let allVersions      = [];
let installedVersions = [];
let allProfiles      = [];
let activeProfileId  = null;
let gameRunning      = false;
let msAuthTimer      = null;
let activeDeviceCode = null;
let currentTheme     = null;   // folder_name of the applied theme

// ── Bootstrap ────────────────────────────────────────────────────────────────
window.addEventListener('DOMContentLoaded', async () => {
  await applyTheme();
  await loadConfig();
  await loadVersions();
  registerTauriEvents();
  registerKeyBindings();
  hideSplash();
});

// ── Theme ────────────────────────────────────────────────────────────────────
async function applyTheme() {
  try {
    const theme = await invoke('theme_get_active');
    currentTheme = theme.folder_name;
    _injectTheme(theme);
  } catch (e) {
    // No theme yet — keep defaults
  }
}

function _injectTheme(theme) {
  // Inject CSS overrides
  el('theme-style').textContent = theme.css || '';

  // Main window background
  const body = document.body;
  if (theme.main_bg_data_uri) {
    body.style.backgroundImage = `url("${theme.main_bg_data_uri}")`;
    body.style.backgroundSize  = 'cover';
    body.style.backgroundPosition = 'center';
  } else {
    body.style.backgroundImage = '';
  }

  // Splash background
  const splash = el('splash');
  if (theme.splash_bg_data_uri) {
    splash.style.backgroundImage = `url("${theme.splash_bg_data_uri}")`;
    splash.classList.add('has-bg');
  } else {
    splash.style.backgroundImage = '';
    splash.classList.remove('has-bg');
  }

  // Splash text
  const texts = theme.meta?.splash_texts ?? [];
  if (texts.length) {
    el('splash-text').textContent = texts[Math.floor(Math.random() * texts.length)];
  }

  // Button labels
  const labels = theme.meta?.labels ?? {};
  const patch = (id, key) => { const e = el(id); if (e && labels[key]) e.textContent = labels[key]; };
  patch('play-btn',    'play');
  patch('install-btn', 'install');
  // nav buttons
  const navBtns = document.querySelectorAll('.nav-btn');
  if (navBtns[0] && labels['nav_play'])     navBtns[0].textContent = labels['nav_play'];
  if (navBtns[1] && labels['nav_settings']) navBtns[1].textContent = labels['nav_settings'];
}

function hideSplash() {
  const splash = el('splash');
  splash.classList.add('hidden');
  splash.addEventListener('transitionend', () => { splash.style.display = 'none'; }, { once: true });
}

async function loadThemeSelector() {
  try {
    const themes = await invoke('theme_list');
    const cfg    = await invoke('config_get');
    const sel    = el('theme-select');
    sel.innerHTML = themes.map(t =>
      `<option value="${t.folder_name}" ${t.folder_name === cfg.active_theme ? 'selected' : ''}>${esc(t.display_name)}</option>`
    ).join('');
  } catch (e) {
    log('Failed to load themes: ' + e);
  }
}

async function openThemesDir() {
  try { await invoke('theme_open_dir'); } catch (e) { log('Error: ' + e); }
}

// ── Config ───────────────────────────────────────────────────────────────────
async function loadConfig() {
  try {
    const cfg = await invoke('config_get');
    applyConfigToUI(cfg);
    updateAccountBar(cfg);
    allProfiles     = cfg.profiles ?? [];
    activeProfileId = cfg.active_profile_id ?? null;
    renderProfileSelect();
    await loadThemeSelector();
  } catch (e) {
    log('Failed to load config: ' + e);
  }
}

function applyConfigToUI(cfg) {
  el('game-dir').value   = cfg.game_dir   ?? '';
  el('java-path').value  = cfg.java_path  ?? '';
  el('min-memory').value = cfg.min_memory_mb ?? 512;
  el('max-memory').value = cfg.max_memory_mb ?? 2048;
  el('jvm-args').value   = (cfg.jvm_args ?? []).join('\n');
}

async function saveSettings() {
  try {
    const cfg = await invoke('config_get');
    const gameDirVal  = el('game-dir').value.trim();
    const javaVal     = el('java-path').value.trim();
    const chosenTheme = el('theme-select').value;
    if (gameDirVal) cfg.game_dir = gameDirVal;
    cfg.java_path     = javaVal || null;
    cfg.min_memory_mb = parseInt(el('min-memory').value) || 512;
    cfg.max_memory_mb = parseInt(el('max-memory').value) || 2048;
    cfg.jvm_args      = el('jvm-args').value.split('\n').map(s => s.trim()).filter(Boolean);

    const themeChanged = chosenTheme && chosenTheme !== currentTheme;
    if (themeChanged) {
      await invoke('theme_set_active', { folderName: chosenTheme });
    }

    await invoke('config_update', { newConfig: cfg });
    log('Settings saved.');

    if (themeChanged) {
      log('Reloading to apply theme…');
      setTimeout(() => location.reload(), 800);
    }
  } catch (e) {
    log('Save failed: ' + e);
  }
}

// ── Profiles ─────────────────────────────────────────────────────────────────
function renderProfileSelect() {
  const sel = el('profile-select');
  if (!allProfiles.length) {
    sel.innerHTML = '<option value="">No profiles — create one</option>';
    el('del-profile-btn').disabled = true;
    updatePlayButtons();
    return;
  }
  sel.innerHTML = allProfiles.map(p =>
    `<option value="${p.id}" ${p.id === activeProfileId ? 'selected' : ''}>
       ${esc(p.name)} (${esc(p.version_id)})
     </option>`
  ).join('');
  el('del-profile-btn').disabled = !activeProfileId;
  updatePlayButtons();
}

async function onProfileChange() {
  const id = el('profile-select').value;
  if (!id) return;
  activeProfileId = id;
  try {
    await invoke('profile_set_active', { profileId: id });
  } catch (e) {
    log('Error: ' + e);
  }
  updatePlayButtons();
}

async function deleteProfile() {
  if (!activeProfileId) return;
  const prof = allProfiles.find(p => p.id === activeProfileId);
  if (!prof) return;
  if (!confirm(`Delete profile "${prof.name}"?\nThe profile folder will NOT be removed from disk.`)) return;
  try {
    await invoke('profile_delete', { profileId: activeProfileId });
    await loadConfig();
    log(`Deleted profile: ${prof.name}`);
  } catch (e) {
    log('Error: ' + e);
  }
}

// ── New-profile modal ────────────────────────────────────────────────────────
function openProfileModal() {
  el('new-profile-name').value = '';
  filterNewProfileVersions();
  openModal('profile-modal');
  setTimeout(() => el('new-profile-name').focus(), 60);
}

function filterNewProfileVersions() {
  const showSnap = el('new-profile-snapshots').checked;
  const sel = el('new-profile-version');
  sel.innerHTML = allVersions
    .filter(v => showSnap || v.type === 'release')
    .map(v => `<option value="${v.id}">${v.id}</option>`)
    .join('');
}

async function confirmCreateProfile() {
  const name      = el('new-profile-name').value.trim();
  const versionId = el('new-profile-version').value;
  if (!name)      { el('new-profile-name').focus(); return; }
  if (!versionId) { log('Select a version for the profile.'); return; }
  try {
    const profile = await invoke('profile_create', { name, versionId });
    allProfiles.push(profile);
    activeProfileId = profile.id;
    renderProfileSelect();
    closeModal('profile-modal');
    log(`Created profile: ${name} (${versionId})`);
  } catch (e) {
    log('Failed to create profile: ' + e);
  }
}

// ── Versions ─────────────────────────────────────────────────────────────────
async function loadVersions() {
  try {
    installedVersions = await invoke('versions_get_installed');
    const manifest    = await invoke('versions_get_manifest');
    allVersions       = manifest.versions;
    filterVersions();
  } catch (e) {
    log('Failed to fetch versions: ' + e);
    el('version-select').innerHTML = '<option value="">Could not load versions</option>';
  }
}

function filterVersions() {
  const showSnap = el('show-snapshots').checked;
  el('version-select').innerHTML = allVersions
    .filter(v => showSnap || v.type === 'release')
    .map(v => {
      const tick = installedVersions.includes(v.id) ? ' ✓' : '';
      return `<option value="${v.id}" data-url="${v.url}">${v.id}${tick}</option>`;
    })
    .join('');
  updatePlayButtons();
}

function onVersionChange() { updatePlayButtons(); }

function updatePlayButtons() {
  const sel       = el('version-select');
  const versionId = sel.value;
  const installed = installedVersions.includes(versionId);
  el('install-btn').textContent = installed ? 'Reinstall' : 'Install';

  // Play is enabled when:  active profile exists + its version is installed + game not running
  const prof         = allProfiles.find(p => p.id === activeProfileId);
  const canPlay      = prof && installedVersions.includes(prof.version_id) && !gameRunning;
  el('play-btn').disabled = !canPlay;
  el('play-btn').textContent = gameRunning ? 'RUNNING…' : 'Play';
}

async function refreshVersions() {
  log('Refreshing…');
  installedVersions = await invoke('versions_get_installed').catch(() => installedVersions);
  const manifest    = await invoke('versions_get_manifest').catch(() => null);
  if (manifest) allVersions = manifest.versions;
  filterVersions();
  log('Done.');
}

// ── Install ───────────────────────────────────────────────────────────────────
async function installVersion() {
  const sel        = el('version-select');
  const versionId  = sel.value;
  const versionUrl = sel.options[sel.selectedIndex]?.dataset.url;
  if (!versionId || !versionUrl) { log('Select a version first.'); return; }

  el('install-btn').disabled = true;
  el('play-btn').disabled    = true;
  el('progress-container').style.display = 'block';
  setProgress(0, 'Starting…');

  try {
    await invoke('game_install_version', { versionId, versionUrl });
    installedVersions = await invoke('versions_get_installed');
    filterVersions();
    log(`✓ ${versionId} installed.`);
  } catch (e) {
    log('Install error: ' + e);
  } finally {
    el('install-btn').disabled = false;
    setTimeout(() => { el('progress-container').style.display = 'none'; }, 3000);
    updatePlayButtons();
  }
}

// ── Launch ────────────────────────────────────────────────────────────────────
async function launchGame() {
  if (!activeProfileId) { log('No profile selected.'); return; }
  const prof = allProfiles.find(p => p.id === activeProfileId);
  if (!prof) { log('Profile not found.'); return; }

  log(`Launching "${prof.name}" (${prof.version_id})…`);
  gameRunning = true;
  updatePlayButtons();

  try {
    await invoke('game_launch', { profileId: activeProfileId });
  } catch (e) {
    log('Launch error: ' + e);
    gameRunning = false;
    updatePlayButtons();
  }
}

// ── Section navigation ────────────────────────────────────────────────────────
function showSection(name, btn) {
  document.querySelectorAll('.section').forEach(s => s.classList.remove('active'));
  el('section-' + name).classList.add('active');
  document.querySelectorAll('.nav-btn').forEach(b => b.classList.remove('active'));
  if (btn) btn.classList.add('active');
}

// ── Account bar ───────────────────────────────────────────────────────────────
function updateAccountBar(cfg) {
  const active = cfg.accounts?.find(a => a.id === cfg.active_account_id);
  el('account-name').textContent = active?.username ?? 'No account';
  el('account-type').textContent = active
    ? (active.account_type === 'microsoft' ? 'Microsoft' : 'Offline')
    : 'click to add';
}

// ── Account modal ─────────────────────────────────────────────────────────────
async function openAccountModal() {
  await renderAccountsList();
  openModal('account-modal');
}

async function renderAccountsList() {
  const cfg  = await invoke('config_get');
  const list = el('accounts-list');
  if (!cfg.accounts?.length) {
    list.innerHTML = '<p class="dim">No accounts yet.</p>';
    return;
  }
  list.innerHTML = cfg.accounts.map(a => `
    <div class="account-item ${a.id === cfg.active_account_id ? 'active' : ''}"
         onclick="selectAccount('${a.id}')">
      <div class="account-item-info">
        <span class="account-item-name">${esc(a.username)}</span>
        <span class="account-item-type">${a.account_type === 'microsoft' ? 'Microsoft' : 'Offline'}</span>
      </div>
      <div onclick="event.stopPropagation()">
        <button class="btn-danger" onclick="removeAccount('${a.id}')">remove</button>
      </div>
    </div>
  `).join('');
}

async function selectAccount(id) {
  try {
    await invoke('auth_set_active', { accountId: id });
    const cfg = await invoke('config_get');
    updateAccountBar(cfg);
    await renderAccountsList();
  } catch (e) { log('Error: ' + e); }
}

async function removeAccount(id) {
  try {
    await invoke('auth_remove', { accountId: id });
    const cfg = await invoke('config_get');
    updateAccountBar(cfg);
    await renderAccountsList();
  } catch (e) { log('Error: ' + e); }
}

// ── Offline account ───────────────────────────────────────────────────────────
function openOfflineModal() {
  closeModal('account-modal');
  el('offline-username').value = '';
  openModal('offline-modal');
  setTimeout(() => el('offline-username').focus(), 60);
}

async function confirmOffline() {
  const username = el('offline-username').value.trim();
  if (!username) return;
  try {
    await invoke('auth_add_offline', { username });
    closeModal('offline-modal');
    const cfg = await invoke('config_get');
    updateAccountBar(cfg);
    log(`Added offline account: ${username}`);
  } catch (e) { log('Error: ' + e); }
}

// ── Microsoft auth ────────────────────────────────────────────────────────────
async function startMicrosoftAuth() {
  try {
    const resp = await invoke('auth_start_microsoft');
    activeDeviceCode = resp.device_code;
    el('ms-device-code').textContent = resp.user_code;
    const linkEl = el('ms-verify-url');
    linkEl.textContent = resp.verification_uri;
    linkEl.href        = resp.verification_uri;
    el('ms-status').textContent = 'Waiting for authorization…';
    el('ms-flow').style.display = 'block';
    scheduleMsPoll(resp.interval * 1000);
  } catch (e) {
    log('Microsoft auth error: ' + e);
  }
}

function scheduleMsPoll(intervalMs) {
  if (msAuthTimer) clearTimeout(msAuthTimer);
  msAuthTimer = setTimeout(() => pollMs(intervalMs), intervalMs);
}

async function pollMs(intervalMs) {
  if (!activeDeviceCode) return;
  try {
    const account = await invoke('auth_poll_microsoft', { deviceCode: activeDeviceCode });
    if (account) {
      el('ms-flow').style.display = 'none';
      cancelMsAuth();
      const cfg = await invoke('config_get');
      updateAccountBar(cfg);
      await renderAccountsList();
      log(`Microsoft account added: ${account.username}`);
    } else {
      scheduleMsPoll(intervalMs);
    }
  } catch (e) {
    el('ms-status').textContent = 'Error: ' + e;
    cancelMsAuth();
  }
}

function cancelMsAuth() {
  if (msAuthTimer) { clearTimeout(msAuthTimer); msAuthTimer = null; }
  activeDeviceCode = null;
  el('ms-flow').style.display = 'none';
}

// ── Java detect ───────────────────────────────────────────────────────────────
async function detectJava() {
  try {
    const paths = await invoke('java_detect');
    if (paths.length > 0) {
      el('java-path').value = paths[0];
      log('Java found: ' + paths[0]);
    } else {
      log('No Java installation detected. Please install Java 21.');
    }
  } catch (e) { log('Java detection failed: ' + e); }
}

// ── Tauri events ──────────────────────────────────────────────────────────────
function registerTauriEvents() {
  listen('install_progress', ({ payload }) => setProgress(payload.progress, payload.message));

  listen('game_started', () => {
    gameRunning = true;
    updatePlayButtons();
    log('Game started!');
  });

  listen('game_stopped', ({ payload }) => {
    gameRunning = false;
    updatePlayButtons();
    log(`Game stopped (exit code: ${payload ?? 'unknown'})`);
  });

  listen('game_error', ({ payload }) => {
    gameRunning = false;
    updatePlayButtons();
    log('Game error: ' + payload);
  });
}

// ── Keyboard shortcuts ────────────────────────────────────────────────────────
function registerKeyBindings() {
  document.addEventListener('keydown', e => {
    if (e.key === 'Escape') {
      ['profile-modal', 'account-modal', 'offline-modal'].forEach(closeModal);
    }
  });
  el('offline-username').addEventListener('keydown',    e => { if (e.key === 'Enter') confirmOffline(); });
  el('new-profile-name').addEventListener('keydown',    e => { if (e.key === 'Enter') confirmCreateProfile(); });
}

// ── Modal helpers ─────────────────────────────────────────────────────────────
function openModal(id)  { el(id).style.display = 'flex'; }
function closeModal(id) { el(id).style.display = 'none'; }
function overlayClose(event, modalId) {
  if (event.target === event.currentTarget) closeModal(modalId);
}

// ── Progress ──────────────────────────────────────────────────────────────────
function setProgress(fraction, message) {
  el('progress-fill').style.width = (fraction * 100) + '%';
  el('progress-msg').textContent  = message;
}

// ── Utility ───────────────────────────────────────────────────────────────────
function log(msg) {
  const area = el('log-area');
  const time = new Date().toLocaleTimeString();
  area.textContent += `[${time}] ${msg}\n`;
  area.scrollTop = area.scrollHeight;
}

function el(id) { return document.getElementById(id); }

function esc(str) {
  return str.replace(/[&<>"']/g, c => (
    { '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;' }[c]
  ));
}

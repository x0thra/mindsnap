import {
  initI18n,
  t,
  setLanguage,
  getAvailableLanguages,
  getCurrentLanguage,
  getUserLanguagePreference,
  onLanguageChange,
  applyTranslations,
} from './i18n.js';

const getInvoke = () => {
  if (window.__TAURI__?.core?.invoke) {
    return window.__TAURI__.core.invoke;
  }
  if (window.__TAURI__?.invoke) {
    return window.__TAURI__.invoke;
  }
  // Safe fallback when running in a standalone browser environment without Tauri
  return async (cmd, args) => {
    console.warn(`[Tauri not detected] Standalone browser mode: ${cmd}`, args);
    return null;
  };
};

const invoke = getInvoke();

// Disable right-click context menu across the application
document.addEventListener('contextmenu', (event) => {
  event.preventDefault();
  return false;
});

// Disable developer inspection, source viewing, and reload shortcuts
document.addEventListener('keydown', (event) => {
  // F12 (DevTools)
  if (event.key === 'F12') {
    event.preventDefault();
    return false;
  }
  // Ctrl + Shift + I / J / C (Inspect / Console / Element Picker)
  if (event.ctrlKey && event.shiftKey && ['I', 'i', 'J', 'j', 'C', 'c'].includes(event.key)) {
    event.preventDefault();
    return false;
  }
  // Ctrl + U (View Source)
  if (event.ctrlKey && (event.key === 'u' || event.key === 'U')) {
    event.preventDefault();
    return false;
  }
  // F5 or Ctrl + R / Ctrl + Shift + R (Page reload)
  if (event.key === 'F5' || (event.ctrlKey && (event.key === 'r' || event.key === 'R'))) {
    event.preventDefault();
    return false;
  }
});

function getTabTitles(tabName) {
  switch (tabName) {
    case 'dashboard':
      return {
        title: t('tabs.dashboard.title'),
        subtitle: t('tabs.dashboard.subtitle'),
      };
    case 'apps':
      return {
        title: t('tabs.apps.title'),
        subtitle: t('tabs.apps.subtitle'),
      };
    case 'settings':
      return {
        title: t('tabs.settings.title'),
        subtitle: t('tabs.settings.subtitle'),
      };
    default:
      return { title: t('brand.name'), subtitle: '' };
  }
}

const navItems = document.querySelectorAll('.nav-item');
const tabPanes = document.querySelectorAll('.tab-pane');
const pageTitle = document.getElementById('page-title');
const pageSubtitle = document.getElementById('page-subtitle');
const navAppsCount = document.getElementById('nav-apps-count');
const toastContainer = document.getElementById('toast-container');

const timerProgressCircle = document.getElementById('timer-progress');
const activeAppPill = document.getElementById('active-app-pill');
const dashboardTimer = document.getElementById('dashboard-timer');
const dashboardStatusLabel = document.getElementById('dashboard-status-label');
const statActiveApp = document.getElementById('stat-active-app');
const statActiveTitle = document.getElementById('stat-active-title');
const statThresholdVal = document.getElementById('stat-threshold-val');
const statRepeatVal = document.getElementById('stat-repeat-val');
const statAlertCount = document.getElementById('stat-alert-count');
const statAlertStatus = document.getElementById('stat-alert-status');

const blacklistSearch = document.getElementById('blacklist-search');
const btnBrowseExe = document.getElementById('btn-browse-exe');
const btnRefreshRunning = document.getElementById('btn-refresh-running');
const runningAppsList = document.getElementById('running-apps-list');
const trackedAppsList = document.getElementById('tracked-apps-list');
const trackedCountBadge = document.getElementById('tracked-count-badge');

const sliderInitial = document.getElementById('slider-initial');
const badgeInitialMin = document.getElementById('badge-initial-min');
const sliderRepeat = document.getElementById('slider-repeat');
const badgeRepeatMin = document.getElementById('badge-repeat-min');
const toggleSound = document.getElementById('toggle-sound');
const toggleMinimize = document.getElementById('toggle-minimize');
const selectLanguage = document.getElementById('select-language');

const customTitlebar = document.getElementById('custom-titlebar');
const btnWinMinimize = document.getElementById('btn-win-minimize');
const btnWinMaximize = document.getElementById('btn-win-maximize');
const btnWinClose = document.getElementById('btn-win-close');
const iconWinMaximize = document.getElementById('icon-win-maximize');
const iconWinRestore = document.getElementById('icon-win-restore');
const titlebarVersion = document.getElementById('titlebar-version');
const btnOpenGithub = document.getElementById('btn-open-github');
const aboutAppVersion = document.getElementById('about-app-version');
const testersListContainer = document.getElementById('testers-list');

const CIRCLE_CIRCUMFERENCE = 2 * Math.PI * 115;
if (timerProgressCircle) {
  timerProgressCircle.style.strokeDasharray = `${CIRCLE_CIRCUMFERENCE}`;
  timerProgressCircle.style.strokeDashoffset = `${CIRCLE_CIRCUMFERENCE}`;
}

let currentTab = 'dashboard';
let currentConfig = null;
let runningApps = [];
let trackedApps = [];
let searchFilterQuery = '';
let saveDebounceTimer = null;

function showToast(message, isSuccess = true) {
  if (!toastContainer) return;
  const toast = document.createElement('div');
  toast.className = 'toast';
  toast.innerHTML = `
    <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="${isSuccess ? '#10b981' : '#ef4444'}" stroke-width="2.5">
      ${isSuccess 
        ? '<polyline points="20 6 9 17 4 12"></polyline>' 
        : '<circle cx="12" cy="12" r="10"></circle><line x1="15" y1="9" x2="9" y2="15"></line><line x1="9" y1="9" x2="15" y2="15"></line>'}
    </svg>
    <span>${message}</span>
  `;
  toastContainer.appendChild(toast);

  setTimeout(() => {
    toast.classList.add('hide');
    setTimeout(() => toast.remove(), 260);
  }, 2300);
}

function formatDuration(totalSeconds) {
  const minutes = Math.floor(totalSeconds / 60);
  const seconds = totalSeconds % 60;
  const mm = String(minutes).padStart(2, '0');
  const ss = String(seconds).padStart(2, '0');
  return `${mm}:${ss}`;
}

function escapeHtml(str) {
  if (!str) return '';
  return str.replace(/[&<>'"]/g, 
    tag => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', "'": '&#39;', '"': '&quot;' }[tag] || tag)
  );
}

function switchTab(tabName) {
  currentTab = tabName;
  navItems.forEach((btn) => {
    btn.classList.toggle('active', btn.dataset.tab === tabName);
  });

  tabPanes.forEach((pane) => {
    pane.classList.toggle('active', pane.id === `tab-${tabName}`);
  });

  const titles = getTabTitles(tabName);
  pageTitle.textContent = titles.title;
  pageSubtitle.textContent = titles.subtitle;

  if (tabName === 'apps') {
    refreshBothAppLists();
  }
}

navItems.forEach((btn) => {
  btn.addEventListener('click', () => {
    switchTab(btn.dataset.tab);
  });
});

function renderIconHtml(item) {
  if (item.icon_base64) {
    return `<img src="${item.icon_base64}" class="app-icon-img" alt="${escapeHtml(item.display_name)}" />`;
  }
  const firstChar = (item.display_name || item.exe_name || 'A').charAt(0).toUpperCase();
  return `<div class="app-icon-fallback">${firstChar}</div>`;
}

function renderAppsView() {
  const query = searchFilterQuery.trim().toLowerCase();

  const filteredRunning = runningApps.filter((app) => {
    if (!query) return true;
    const name = (app.display_name || '').toLowerCase();
    const exe = (app.exe_name || '').toLowerCase();
    const title = (app.window_title || '').toLowerCase();
    return name.includes(query) || exe.includes(query) || title.includes(query);
  });

  runningAppsList.innerHTML = '';
  if (filteredRunning.length === 0) {
    runningAppsList.innerHTML = `
      <div class="empty-apps-msg">
        ${query ? t('apps.empty_running_search') : t('apps.empty_running')}
      </div>
    `;
  } else {
    filteredRunning.forEach((app) => {
      const isAlreadyTracked = trackedApps.some(t => (t.exe_name || '').toLowerCase() === (app.exe_name || '').toLowerCase());
      const card = document.createElement('div');
      card.className = 'app-rich-card';
      card.innerHTML = `
        ${renderIconHtml(app)}
        <div class="app-meta">
          <div class="app-name" title="${escapeHtml(app.display_name)}">${escapeHtml(app.display_name)}</div>
          <div class="app-sub" title="${escapeHtml(app.window_title || app.exe_name)}">${escapeHtml(app.exe_name)}</div>
        </div>
        ${
          isAlreadyTracked 
            ? `<span class="badge-tracked">${t('apps.badge_tracked')}</span>`
            : `<button type="button" class="btn-add-track" data-exe="${escapeHtml(app.exe_name)}">${t('apps.btn_track')}</button>`
        }
      `;

      const addBtn = card.querySelector('.btn-add-track');
      if (addBtn) {
        addBtn.addEventListener('click', () => {
          handleTrackApp(app.exe_name, app.display_name);
        });
      }

      runningAppsList.appendChild(card);
    });
  }

  const filteredTracked = trackedApps.filter((app) => {
    if (!query) return true;
    const name = (app.display_name || '').toLowerCase();
    const exe = (app.exe_name || '').toLowerCase();
    return name.includes(query) || exe.includes(query);
  });

  navAppsCount.textContent = trackedApps.length;
  trackedCountBadge.textContent = trackedApps.length;

  trackedAppsList.innerHTML = '';
  if (filteredTracked.length === 0) {
    trackedAppsList.innerHTML = `
      <div class="empty-apps-msg">
        ${query ? t('apps.empty_tracked_search') : `${t('apps.empty_tracked')}<br><span style="font-size: 0.76rem; opacity: 0.7; margin-top: 6px; display: inline-block;">${t('apps.empty_tracked_hint')}</span>`}
      </div>
    `;
  } else {
    filteredTracked.forEach((app) => {
      const card = document.createElement('div');
      card.className = 'app-rich-card';
      card.innerHTML = `
        ${renderIconHtml(app)}
        <div class="app-meta">
          <div class="app-name" title="${escapeHtml(app.display_name)}">${escapeHtml(app.display_name)}</div>
          <div class="app-sub">${escapeHtml(app.exe_name)}</div>
        </div>
        <button type="button" class="btn-remove-track" title="${t('apps.btn_untrack_tooltip')}">
          <svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <polyline points="3 6 5 6 21 6"></polyline>
            <path d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6m3 0V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2"></path>
          </svg>
        </button>
      `;

      card.querySelector('.btn-remove-track').addEventListener('click', () => {
        handleUntrackApp(app.exe_name, app.display_name);
      });

      trackedAppsList.appendChild(card);
    });
  }
}

function mergeAppsPreservingIcons(freshList, fallbackSources = []) {
  if (!Array.isArray(freshList)) return [];
  return freshList.map(item => {
    if (item.icon_base64) return item;
    for (const source of fallbackSources) {
      if (Array.isArray(source)) {
        const found = source.find(s => s.exe_name.toLowerCase() === item.exe_name.toLowerCase());
        if (found && found.icon_base64) {
          return { ...item, icon_base64: found.icon_base64 };
        }
      }
    }
    return item;
  });
}

async function handleTrackApp(exeName, displayName) {
  try {
    const updated = await invoke('add_app_to_blacklist', { exeName });
    if (updated) {
      trackedApps = mergeAppsPreservingIcons(updated, [trackedApps, runningApps]);
    }
    renderAppsView();
    showToast(t('toasts.app_added', { app: displayName || exeName }));
  } catch (error) {
    console.error('Failed to add app to tracking list:', error);
    showToast(typeof error === 'string' ? error : t('toasts.app_add_failed'), false);
  }
}

async function handleUntrackApp(exeName, displayName) {
  try {
    const updated = await invoke('remove_app_from_blacklist', { exeName });
    if (updated) {
      trackedApps = mergeAppsPreservingIcons(updated, [trackedApps, runningApps]);
    }
    renderAppsView();
    showToast(t('toasts.app_removed', { app: displayName || exeName }));
  } catch (error) {
    console.error('Failed to remove app from tracking list:', error);
    showToast(typeof error === 'string' ? error : t('toasts.app_remove_failed'), false);
  }
}

async function handleBrowseExe() {
  try {
    const picked = await invoke('pick_and_add_exe');
    if (picked) {
      await refreshBothAppLists();
      showToast(t('toasts.app_added', { app: picked.display_name || picked.exe_name }));
    }
  } catch (error) {
    console.error('File picker failed:', error);
    showToast(typeof error === 'string' ? error : t('toasts.browse_failed'), false);
  }
}

async function refreshBothAppLists() {
  try {
    const [running, tracked] = await Promise.all([
      invoke('get_running_apps'),
      invoke('get_tracked_apps'),
    ]);

    if (running) runningApps = running;
    if (tracked) trackedApps = mergeAppsPreservingIcons(tracked, [trackedApps, runningApps]);

    renderAppsView();
  } catch (error) {
    console.warn('Failed to refresh app lists:', error);
  }
}

function updateSliderFill(slider) {
  if (!slider) return;
  const min = parseFloat(slider.min) || 0;
  const max = parseFloat(slider.max) || 100;
  const val = parseFloat(slider.value) || 0;
  const pct = Math.max(0, Math.min(100, ((val - min) / (max - min)) * 100));
  slider.style.background = `linear-gradient(to right, #6366f1 0%, #8b5cf6 ${pct}%, rgba(255, 255, 255, 0.08) ${pct}%, rgba(255, 255, 255, 0.08) 100%)`;

  const presetContainer = document.querySelector(`.slider-presets[data-target="${slider.id}"]`);
  if (presetContainer) {
    const chips = presetContainer.querySelectorAll('.preset-chip');
    chips.forEach(chip => {
      chip.classList.toggle('active', parseInt(chip.dataset.val, 10) === parseInt(val, 10));
    });
  }
}

function populateLanguageDropdown() {
  if (!selectLanguage) return;
  const languages = getAvailableLanguages();
  const currentLang = getUserLanguagePreference() || (currentConfig ? currentConfig.language : 'auto') || 'auto';

  selectLanguage.innerHTML = '';

  const autoOpt = document.createElement('option');
  autoOpt.value = 'auto';
  autoOpt.textContent = t('settings.lang_auto');
  selectLanguage.appendChild(autoOpt);

  languages.forEach(lang => {
    const opt = document.createElement('option');
    opt.value = lang.code;
    opt.textContent = lang.nativeName || lang.name;
    selectLanguage.appendChild(opt);
  });

  selectLanguage.value = currentLang;
}

async function loadConfig() {
  try {
    currentConfig = await invoke('get_config');
  } catch (error) {
    console.error('Failed to load initial config:', error);
  }

  if (currentConfig) {
    sliderInitial.value = currentConfig.initial_alert_minutes;
    sliderRepeat.value = currentConfig.repeat_alert_minutes;
    toggleSound.checked = currentConfig.sound_enabled;
    toggleMinimize.checked = currentConfig.minimize_to_tray_on_close;
  }
}

function applyConfigToUI() {
  if (!currentConfig) return;

  badgeInitialMin.textContent = t('settings.min_unit', { count: currentConfig.initial_alert_minutes });
  statThresholdVal.textContent = t('dashboard.minutes_unit', { count: currentConfig.initial_alert_minutes });
  updateSliderFill(sliderInitial);

  badgeRepeatMin.textContent = t('settings.min_unit', { count: currentConfig.repeat_alert_minutes });
  if (statRepeatVal) {
    statRepeatVal.textContent = t('dashboard.repeat_unit_hint', { count: currentConfig.repeat_alert_minutes });
  }
  updateSliderFill(sliderRepeat);

  document.querySelectorAll('.preset-chip').forEach((chip) => {
    const val = chip.dataset.val;
    chip.textContent = t('settings.min_unit', { count: val });
  });

  toggleSound.checked = currentConfig.sound_enabled;
  toggleMinimize.checked = currentConfig.minimize_to_tray_on_close;
}

async function persistConfig(showNotice = false) {
  if (!currentConfig) return;
  try {
    await invoke('save_config', { newConfig: currentConfig });
    if (showNotice) {
      showToast(t('toasts.settings_saved'));
    }
  } catch (error) {
    console.error('Failed to save config:', error);
    showToast(t('toasts.settings_save_failed'), false);
  }
}

function debounceSave() {
  clearTimeout(saveDebounceTimer);
  saveDebounceTimer = setTimeout(() => persistConfig(true), 400);
}

async function updateTrackerStatus() {
  try {
    const status = await invoke('get_tracker_status');
    if (!status) return;

    let appDisplayName = status.current_app_display_name;
    if (!appDisplayName && status.current_app) {
      const match = trackedApps.find(a => a.exe_name.toLowerCase() === status.current_app.toLowerCase())
        || runningApps.find(a => a.exe_name.toLowerCase() === status.current_app.toLowerCase());
      if (match && match.display_name) {
        appDisplayName = match.display_name;
      } else {
        appDisplayName = status.current_app;
      }
    }

    const currentAppName = appDisplayName || t('dashboard.desktop_idle');
    activeAppPill.textContent = currentAppName;
    activeAppPill.title = currentAppName;

    if (statActiveApp) {
      statActiveApp.textContent = currentAppName;
      statActiveApp.title = currentAppName;
    }

    if (statActiveTitle) {
      const winTitle = (status.current_window_title || '').trim();
      if (winTitle && winTitle.toLowerCase() !== currentAppName.toLowerCase()) {
        statActiveTitle.textContent = winTitle;
        statActiveTitle.title = winTitle;
      } else if (status.is_blacklisted) {
        statActiveTitle.textContent = t('dashboard.tracked_mode');
        statActiveTitle.title = '';
      } else {
        statActiveTitle.textContent = t('dashboard.desktop_idle');
        statActiveTitle.title = '';
      }
    }

    if (statRepeatVal && currentConfig) {
      statRepeatVal.textContent = t('dashboard.repeat_unit_hint', { count: currentConfig.repeat_alert_minutes });
    }

    if (status.is_blacklisted) {
      activeAppPill.classList.add('alert');
      dashboardStatusLabel.textContent = t('dashboard.tracked_mode');
      dashboardTimer.textContent = formatDuration(status.continuous_seconds);

      const targetSec = status.initial_threshold_seconds || 300;
      const progressRatio = Math.min(1, status.continuous_seconds / targetSec);
      const strokeOffset = CIRCLE_CIRCUMFERENCE * (1 - progressRatio);
      timerProgressCircle.style.strokeDashoffset = `${strokeOffset}`;

      statAlertCount.textContent = status.alert_count > 0 
        ? t('dashboard.stat_alerts_count', { count: status.alert_count }) 
        : t('dashboard.stat_alerts_zero');

      if (statAlertStatus) {
        statAlertStatus.textContent = status.alert_count > 0
          ? t('dashboard.stat_alerts_triggered')
          : t('dashboard.stat_alerts_none');
      }
    } else {
      activeAppPill.classList.remove('alert');
      dashboardStatusLabel.textContent = t('dashboard.focus_mode');
      dashboardTimer.textContent = '00:00';
      timerProgressCircle.style.strokeDashoffset = `${CIRCLE_CIRCUMFERENCE}`;
      statAlertCount.textContent = t('dashboard.stat_alerts_zero');

      if (statAlertStatus) {
        statAlertStatus.textContent = t('dashboard.stat_alerts_none');
      }
    }
  } catch (error) {
    console.warn('Failed to query tracker status:', error);
  }
}

sliderInitial.addEventListener('input', (e) => {
  const val = parseInt(e.target.value, 10);
  badgeInitialMin.textContent = t('settings.min_unit', { count: val });
  statThresholdVal.textContent = t('dashboard.minutes_unit', { count: val });
  updateSliderFill(sliderInitial);
  if (currentConfig) {
    currentConfig.initial_alert_minutes = val;
    debounceSave();
  }
});

sliderRepeat.addEventListener('input', (e) => {
  const val = parseInt(e.target.value, 10);
  badgeRepeatMin.textContent = t('settings.min_unit', { count: val });
  if (statRepeatVal) {
    statRepeatVal.textContent = t('dashboard.repeat_unit_hint', { count: val });
  }
  updateSliderFill(sliderRepeat);
  if (currentConfig) {
    currentConfig.repeat_alert_minutes = val;
    debounceSave();
  }
});

document.querySelectorAll('.slider-presets .preset-chip').forEach((chip) => {
  chip.addEventListener('click', () => {
    const parent = chip.closest('.slider-presets');
    const targetId = parent ? parent.dataset.target : null;
    const targetSlider = targetId ? document.getElementById(targetId) : null;
    if (!targetSlider) return;

    const val = parseInt(chip.dataset.val, 10);
    targetSlider.value = val;
    targetSlider.dispatchEvent(new Event('input', { bubbles: true }));
    showToast(t('toasts.preset_set', { min: val }));
  });
});

toggleSound.addEventListener('change', (e) => {
  if (currentConfig) {
    currentConfig.sound_enabled = e.target.checked;
    persistConfig(true);
  }
});

toggleMinimize.addEventListener('change', (e) => {
  if (currentConfig) {
    currentConfig.minimize_to_tray_on_close = e.target.checked;
    persistConfig(true);
  }
});

if (selectLanguage) {
  selectLanguage.addEventListener('change', async (e) => {
    const selected = e.target.value;
    if (currentConfig) {
      currentConfig.language = selected;
      try {
        await invoke('sync_locale', { language: selected });
      } catch (err) {
        console.warn('Failed to sync locale with backend:', err);
      }
      await persistConfig(false);
    }
    await setLanguage(selected);
    showToast(t('toasts.settings_saved'));
  });
}

blacklistSearch.addEventListener('input', (e) => {
  searchFilterQuery = e.target.value;
  renderAppsView();
});

btnBrowseExe.addEventListener('click', () => {
  handleBrowseExe();
});

btnRefreshRunning.addEventListener('click', async () => {
  btnRefreshRunning.style.transform = 'rotate(180deg)';
  setTimeout(() => (btnRefreshRunning.style.transform = 'rotate(0deg)'), 300);
  await refreshBothAppLists();
  showToast(t('toasts.windows_refreshed'));
});

async function handleWinMinimize() {
  try {
    await invoke('app_minimize');
  } catch (err) {
    console.warn('Failed to minimize window:', err);
  }
}

async function handleWinToggleMaximize() {
  try {
    const isMax = await invoke('app_toggle_maximize');
    if (iconWinMaximize && iconWinRestore) {
      iconWinMaximize.style.display = isMax ? 'none' : 'block';
      iconWinRestore.style.display = isMax ? 'block' : 'none';
    }
  } catch (err) {
    console.warn('Failed to toggle window maximize:', err);
  }
}

async function handleWinClose() {
  try {
    await invoke('app_close');
  } catch (err) {
    console.warn('Failed to close window:', err);
  }
}

if (btnWinMinimize) {
  btnWinMinimize.addEventListener('click', handleWinMinimize);
}
if (btnWinMaximize) {
  btnWinMaximize.addEventListener('click', handleWinToggleMaximize);
}
if (btnWinClose) {
  btnWinClose.addEventListener('click', handleWinClose);
}
if (customTitlebar) {
  customTitlebar.addEventListener('mousedown', (e) => {
    if (e.button === 0 && !e.target.closest('.win-btn') && !e.target.closest('button')) {
      invoke('app_start_dragging').catch(() => {});
    }
  });

  customTitlebar.addEventListener('dblclick', (e) => {
    if (!e.target.closest('.win-btn') && !e.target.closest('button')) {
      handleWinToggleMaximize();
    }
  });
}

window.addEventListener('resize', () => {
  const isMax = window.innerWidth >= (window.screen.availWidth - 10) && 
                window.innerHeight >= (window.screen.availHeight - 40);
  if (iconWinMaximize && iconWinRestore) {
    iconWinMaximize.style.display = isMax ? 'none' : 'block';
    iconWinRestore.style.display = isMax ? 'block' : 'none';
  }
});

if (btnOpenGithub) {
  btnOpenGithub.addEventListener('click', async () => {
    try {
      await invoke('open_external_url', { url: 'https://github.com/x0thra/mindsnap' });
    } catch (err) {
      console.warn('Failed to open repository URL:', err);
    }
  });
}

async function loadAndRenderTesters() {
  if (!testersListContainer) return;

  try {
    const response = await fetch('./testers.json');
    if (!response.ok) {
      throw new Error(`HTTP ${response.status}`);
    }
    const testers = await response.json();

    testersListContainer.innerHTML = '';
    if (Array.isArray(testers) && testers.length > 0) {
      testers.forEach((tester) => {
        const chip = document.createElement('div');
        chip.className = 'tester-chip';

        const name = typeof tester === 'string' ? tester : tester.name;
        const role = typeof tester === 'object' && tester.role ? tester.role : null;

        chip.innerHTML = `
          <span>★</span>
          <span>${name}</span>
          ${role ? `<span class="tester-chip-role">${role}</span>` : ''}
        `;
        testersListContainer.appendChild(chip);
      });
    } else {
      testersListContainer.innerHTML = `<span class="tester-empty-msg">${t('settings.about_testers_empty')}</span>`;
    }
  } catch (_err) {
    testersListContainer.innerHTML = `<span class="tester-empty-msg">${t('settings.about_testers_empty')}</span>`;
  }
}

onLanguageChange(() => {
  const titles = getTabTitles(currentTab);
  pageTitle.textContent = titles.title;
  pageSubtitle.textContent = titles.subtitle;

  populateLanguageDropdown();
  applyConfigToUI();
  renderAppsView();
  updateTrackerStatus();
  loadAndRenderTesters();
});

async function initApp() {
  await loadConfig();
  const initialLang = currentConfig ? currentConfig.language || 'auto' : 'auto';
  await initI18n(initialLang);

  applyConfigToUI();

  populateLanguageDropdown();
  await refreshBothAppLists();
  await loadAndRenderTesters();

  try {
    const version = await invoke('get_app_version');
    if (version) {
      const displayVer = version.replace(/^(\d+\.\d+)\.0-beta/, '$1-beta');
      if (titlebarVersion) titlebarVersion.textContent = `v${displayVer}`;
      if (aboutAppVersion) aboutAppVersion.textContent = `v${displayVer}`;
    }
  } catch (err) {
    console.warn('Failed to get app version:', err);
  }

  updateTrackerStatus();
  setInterval(updateTrackerStatus, 1000);
}

initApp();

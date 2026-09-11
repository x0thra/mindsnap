let availableLanguages = [];
let currentLanguageCode = 'tr_tr';
let activeTranslations = {};
let fallbackTranslations = {};
const languageChangeListeners = new Set();

function resolveKey(obj, path) {
  if (!obj || !path) return undefined;
  return path.split('.').reduce((prev, curr) => (prev && prev[curr] !== undefined ? prev[curr] : undefined), obj);
}

function normalizeLocaleString(str) {
  if (!str) return '';
  return str.toLowerCase().replace('-', '_');
}

function matchLocaleToCode(systemLocale, languages) {
  if (!systemLocale || !languages || languages.length === 0) {
    return 'en_us';
  }

  const normalized = normalizeLocaleString(systemLocale);
  const langPrefix = normalized.split('_')[0];

  // Exact code or alias match (e.g. "tr_tr")
  for (const lang of languages) {
    if (lang.code === normalized) return lang.code;
    if (Array.isArray(lang.aliases) && lang.aliases.includes(normalized)) {
      return lang.code;
    }
  }

  // Prefix match (e.g. "tr")
  for (const lang of languages) {
    if (lang.code.startsWith(langPrefix)) return lang.code;
    if (Array.isArray(lang.aliases) && lang.aliases.some(a => a.startsWith(langPrefix))) {
      return lang.code;
    }
  }

  return 'en_us';
}

async function loadTranslationFile(code) {
  try {
    const res = await fetch(`./locales/${code}.json`);
    if (!res.ok) {
      throw new Error(`HTTP ${res.status}`);
    }
    return await res.json();
  } catch (err) {
    console.warn(`[i18n] Failed to load locale '${code}.json':`, err);
    return null;
  }
}

let currentPreference = 'auto';

function getBackendInvoke() {
  if (typeof window !== 'undefined') {
    if (window.__TAURI__?.core?.invoke) return window.__TAURI__.core.invoke;
    if (window.__TAURI__?.invoke) return window.__TAURI__.invoke;
  }
  return null;
}

export async function resolveLanguageCode(preference) {
  const clean = (preference || 'auto').trim().toLowerCase();

  if (clean !== 'auto') {
    const exists = availableLanguages.some(l => l.code === clean);
    if (exists) return clean;
  }

  // Query Tauri backend for authoritative OS language detection
  const invoke = getBackendInvoke();
  if (invoke) {
    try {
      const backendResolved = await invoke('resolve_locale', { preference: clean });
      if (backendResolved && availableLanguages.some(l => l.code === backendResolved)) {
        return backendResolved;
      }
    } catch (err) {
      console.warn('[i18n] Backend locale resolution error:', err);
    }
  }

  // Fallback for standalone browser environments
  const candidateLocales = [
    navigator.language,
    ...(Array.isArray(navigator.languages) ? navigator.languages : [])
  ].filter(Boolean);

  for (const loc of candidateLocales) {
    const matched = matchLocaleToCode(loc, availableLanguages);
    if (matched && matched !== 'en_us') {
      return matched;
    }
  }

  return matchLocaleToCode(navigator.language, availableLanguages);
}

export async function initI18n(userPreference = 'auto') {
  currentPreference = userPreference || 'auto';

  try {
    const res = await fetch('./locales/languages.json');
    if (res.ok) {
      availableLanguages = await res.json();
    }
  } catch (err) {
    console.warn('[i18n] Could not load languages.json:', err);
    availableLanguages = [
      { code: 'tr_tr', name: 'Türkçe', nativeName: 'Türkçe' },
      { code: 'en_us', name: 'English (US)', nativeName: 'English (US)' }
    ];
  }

  fallbackTranslations = (await loadTranslationFile('en_us')) || {};

  const targetCode = await resolveLanguageCode(currentPreference);
  await setLanguage(currentPreference, true);
}

export async function setLanguage(preference, notifyListeners = true) {
  currentPreference = preference || 'auto';
  const targetCode = await resolveLanguageCode(currentPreference);

  let loaded = await loadTranslationFile(targetCode);
  if (!loaded) {
    console.warn(`[i18n] Falling back to default dictionary for '${targetCode}'.`);
    loaded = fallbackTranslations;
  }

  currentLanguageCode = targetCode;
  activeTranslations = loaded;

  document.documentElement.lang = targetCode.split('_')[0];
  applyTranslations();

  if (notifyListeners) {
    languageChangeListeners.forEach(listener => {
      try {
        listener(currentLanguageCode);
      } catch (e) {
        console.error('[i18n] Listener error:', e);
      }
    });
  }
}


export function t(key, params = {}) {
  let val = resolveKey(activeTranslations, key);
  if (val === undefined) {
    val = resolveKey(fallbackTranslations, key);
  }
  if (val === undefined) {
    val = key;
  }

  if (typeof val === 'string' && params && typeof params === 'object') {
    for (const [pKey, pVal] of Object.entries(params)) {
      val = val.replaceAll(`{${pKey}}`, pVal);
    }
  }

  return val;
}

export function applyTranslations() {
  document.querySelectorAll('[data-i18n]').forEach(el => {
    const key = el.dataset.i18n;
    if (!key) return;
    const translation = t(key);
    if (translation.includes('<') && translation.includes('>')) {
      el.innerHTML = translation;
    } else {
      el.textContent = translation;
    }
  });

  document.querySelectorAll('[data-i18n-placeholder]').forEach(el => {
    const key = el.dataset.i18nPlaceholder;
    if (key) el.placeholder = t(key);
  });

  document.querySelectorAll('[data-i18n-title]').forEach(el => {
    const key = el.dataset.i18nTitle;
    if (key) el.title = t(key);
  });
}

export function onLanguageChange(callback) {
  languageChangeListeners.add(callback);
  return () => languageChangeListeners.delete(callback);
}

export function getCurrentLanguage() {
  return currentLanguageCode;
}

export function getUserLanguagePreference() {
  return currentPreference;
}

export function getAvailableLanguages() {
  return [...availableLanguages];
}


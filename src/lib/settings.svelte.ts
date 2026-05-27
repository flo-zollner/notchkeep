import { I18N, type Lang } from './i18n/strings';

type Theme = 'light' | 'dark';

const STORAGE_KEY = 'saldo.settings';

interface PersistedSettings {
  theme: Theme;
  lang: Lang;
  hide: boolean;
  showCents: boolean;
}

const DEFAULTS: PersistedSettings = { theme: 'light', lang: 'de', hide: false, showCents: false };

function load(): PersistedSettings {
  if (typeof localStorage === 'undefined') return DEFAULTS;
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (!raw) return DEFAULTS;
    const parsed = JSON.parse(raw) as Partial<PersistedSettings>;
    return {
      theme: parsed.theme === 'dark' ? 'dark' : 'light',
      lang: parsed.lang === 'en' ? 'en' : 'de',
      hide: !!parsed.hide,
      showCents: !!parsed.showCents,
    };
  } catch {
    return DEFAULTS;
  }
}

function persist(s: PersistedSettings) {
  if (typeof localStorage === 'undefined') return;
  localStorage.setItem(STORAGE_KEY, JSON.stringify(s));
}

const initial = load();

export const settings = $state({
  theme: initial.theme,
  lang: initial.lang,
  hide: initial.hide,
  showCents: initial.showCents,
});

function persistAll() {
  persist({ theme: settings.theme, lang: settings.lang, hide: settings.hide, showCents: settings.showCents });
}

export function setTheme(v: Theme) {
  settings.theme = v;
  persistAll();
}

export function setLang(v: Lang) {
  settings.lang = v;
  persistAll();
}

export function setHide(v: boolean) {
  settings.hide = v;
  persistAll();
}

export function setShowCents(v: boolean) {
  settings.showCents = v;
  persistAll();
}

/** Decimal places for fmtEur, depending on the showCents setting. Reactive. */
export function eurDecimals(): 0 | 2 {
  return settings.showCents ? 2 : 0;
}

export function t() {
  return I18N[settings.lang];
}

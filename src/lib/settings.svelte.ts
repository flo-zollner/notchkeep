import { I18N, type Lang } from './i18n/strings';

export type Theme = 'auto' | 'light' | 'dark';
export type UpdateConsent = 'unset' | 'enabled' | 'declined';

function parseTheme(v: unknown): Theme {
  if (v === 'auto' || v === 'light' || v === 'dark') return v;
  return 'auto';
}

const STORAGE_KEY = 'saldo.settings';

interface PersistedSettings {
  theme: Theme;
  lang: Lang;
  hide: boolean;
  showCents: boolean;
  /** First-run setup wizard has been completed or skipped. */
  onboardingCompleted: boolean;
  /** Interactive feature tour (coach-marks) has been completed or skipped. */
  tourCompleted: boolean;
  /** Opt-in status for automatic update checks. */
  updateConsent: UpdateConsent;
  /** Version the user chose to skip, e.g. "0.2.3". Exact string match. */
  skippedVersion: string | null;
}

const DEFAULTS: PersistedSettings = {
  theme: 'auto', lang: 'de', hide: false, showCents: false,
  onboardingCompleted: false, tourCompleted: false,
  updateConsent: 'unset', skippedVersion: null,
};

function load(): PersistedSettings {
  if (typeof localStorage === 'undefined') return DEFAULTS;
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (!raw) return DEFAULTS;
    const parsed = JSON.parse(raw) as Partial<PersistedSettings>;
    return {
      theme: parseTheme(parsed.theme),
      lang: parsed.lang === 'en' ? 'en' : 'de',
      hide: !!parsed.hide,
      showCents: !!parsed.showCents,
      onboardingCompleted: !!parsed.onboardingCompleted,
      tourCompleted: !!parsed.tourCompleted,
      updateConsent:
        parsed.updateConsent === 'enabled' || parsed.updateConsent === 'declined'
          ? parsed.updateConsent : 'unset',
      skippedVersion:
        typeof parsed.skippedVersion === 'string' ? parsed.skippedVersion : null,
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
  onboardingCompleted: initial.onboardingCompleted,
  tourCompleted: initial.tourCompleted,
  updateConsent: initial.updateConsent,
  skippedVersion: initial.skippedVersion,
});

function persistAll() {
  persist({
    theme: settings.theme, lang: settings.lang, hide: settings.hide, showCents: settings.showCents,
    onboardingCompleted: settings.onboardingCompleted, tourCompleted: settings.tourCompleted,
    updateConsent: settings.updateConsent, skippedVersion: settings.skippedVersion,
  });
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

export function setOnboardingCompleted(v: boolean) {
  settings.onboardingCompleted = v;
  persistAll();
}

export function setTourCompleted(v: boolean) {
  settings.tourCompleted = v;
  persistAll();
}

export function setUpdateConsent(v: UpdateConsent) { settings.updateConsent = v; persistAll(); }
export function setSkippedVersion(v: string | null) { settings.skippedVersion = v; persistAll(); }

/** Decimal places for fmtEur, depending on the showCents setting. Reactive. */
export function eurDecimals(): 0 | 2 {
  return settings.showCents ? 2 : 0;
}

export function t() {
  return I18N[settings.lang];
}

/** Re-reads localStorage and re-initializes the singleton settings state.
 *  Tests-only — never call from app code. */
export function _reloadForTests(): void {
  const fresh = load();
  settings.theme = fresh.theme;
  settings.lang = fresh.lang;
  settings.hide = fresh.hide;
  settings.showCents = fresh.showCents;
  settings.onboardingCompleted = fresh.onboardingCompleted;
  settings.tourCompleted = fresh.tourCompleted;
  settings.updateConsent = fresh.updateConsent;
  settings.skippedVersion = fresh.skippedVersion;
}

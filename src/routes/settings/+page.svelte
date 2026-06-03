<script lang="ts">
  import CategoriesModal from '$lib/components/CategoriesModal.svelte';
  import RulesModal from '$lib/components/RulesModal.svelte';
  import LicensesModal from '$lib/components/LicensesModal.svelte';
  import Icon from '$lib/components/Icon.svelte';
  import { settings, setHide, setLang, setShowCents, setTheme, setUpdateConsent, setReleaseChannel, t } from '$lib/settings.svelte';
  import { getVersion } from '@tauri-apps/api/app';
  import { checkNow, updateState, downloadAndInstall, skipCurrent, restart } from '$lib/updater/updater.svelte';
  import UpdateAvailableDialog from '$lib/components/UpdateAvailableDialog.svelte';
  import ChannelWarningDialog from '$lib/components/ChannelWarningDialog.svelte';
  import { startOnboarding, startTour } from '$lib/onboarding/onboarding.svelte';
  import ExportButton from '$lib/components/ExportButton.svelte';
  import KursRefreshButton from '$lib/components/KursRefreshButton.svelte';
  import { api, type Account, type Category, type ExportFilter } from '$lib/api';
  import DateField from '$lib/components/DateField.svelte';
  import { openUrl } from '@tauri-apps/plugin-opener';
  import { onMount } from 'svelte';

  const SOURCE_URL = 'https://github.com/flo-zollner/notchkeep';
  // Display label without the scheme, derived from SOURCE_URL so the link target
  // and the visible text can never drift apart.
  const SOURCE_LABEL = SOURCE_URL.replace(/^https?:\/\//, '');
  // Canonical GPL-3.0 text (the app's own license). Opened externally rather than
  // bundled, so we don't ship the full license verbatim in the repo's static dir.
  const LICENSE_URL = 'https://www.gnu.org/licenses/gpl-3.0.html';
  async function openExternal(url: string) {
    try { await openUrl(url); } catch { window.open(url, '_blank'); }
  }

  const tp = $derived(t().portfolio);

  let showCats = $state(false);
  let showRules = $state(false);
  let showThirdParty = $state(false);
  let detectingTransfers = $state(false);
  let detectTransfersResult = $state<number | null>(null);

  async function runDetectTransfers() {
    detectingTransfers = true;
    detectTransfersResult = null;
    try {
      detectTransfersResult = await api.detectTransfers();
    } catch {
      // silently ignore
    } finally {
      detectingTransfers = false;
    }
  }

  let cleanupResult = $state<number | null>(null);
  let cleaningMirrors = $state(false);
  async function runCleanupPhantomMirrors() {
    cleaningMirrors = true;
    cleanupResult = null;
    try {
      cleanupResult = await api.cleanupPhantomMirrors();
    } finally {
      cleaningMirrors = false;
    }
  }

  let accounts = $state<Account[]>([]);
  let categories = $state<Category[]>([]);
  let from = $state('');
  let to = $state('');
  let accFilter = $state<'all' | number>('all');
  let catFilter = $state<'all' | number>('all');
  let searchFilter = $state('');

  onMount(async () => {
    accounts = await api.listAccounts();
    categories = await api.listCategories();
  });

  let appVersion = $state('');
  let checkMessage = $state('');
  let showUpdate = $state(false);
  $effect(() => { void getVersion().then((v) => (appVersion = v)).catch(() => {}); });

  function toggleAuto() {
    setUpdateConsent(settings.updateConsent === 'enabled' ? 'declined' : 'enabled');
  }
  async function onCheckNow() {
    checkMessage = t().updates.checking;
    const hasUpdate = await checkNow();
    if (hasUpdate) { showUpdate = true; checkMessage = ''; }
    else if (updateState.status === 'error') checkMessage = t().updates.checkFailed;
    else checkMessage = t().updates.upToDate;
  }

  let showChannelWarning = $state(false);

  // Deliberate deviation from the UX guide's "no confirm for reversible actions"
  // (§9/§14): switching channel is reversible, but enabling prereleases is a
  // consequential system decision (unstable builds, no auto-downgrade) — an
  // informed confirm is justified (§11/§15), analogous to UpdateActivationDialog.
  function selectChannel(c: 'stable' | 'beta') {
    if (c === settings.releaseChannel) return;
    if (c === 'beta') { showChannelWarning = true; return; } // confirm before enabling beta
    setReleaseChannel('stable');
  }
  function confirmBeta() {
    setReleaseChannel('beta');
    showChannelWarning = false;
  }
  function cancelBeta() {
    showChannelWarning = false; // selection stays 'stable'
  }

  function buildExportFilter(): ExportFilter {
    return {
      from: from || undefined,
      to: to || undefined,
      accountId: accFilter === 'all' ? undefined : (accFilter as number),
      categoryId: catFilter === 'all' ? undefined : (catFilter as number),
      search: searchFilter.trim() === '' ? undefined : searchFilter.trim(),
    };
  }
</script>

<div class="topbar">
  <div>
    <h1>{t().nav.settings}</h1>
    <div class="sub">{t().common.preferences}</div>
  </div>
</div>

<div class="grid-12">
  <div class="card col-6 card-pad-lg">
    <div class="card-h"><h3>{t().common.preferences}</h3></div>

    <div class="setting-row">
      <div>
        <div class="sr-label">{t().common.theme}</div>
        <div class="sr-sub">{t().common.themeAuto} · {t().common.themeLight} · {t().common.themeDark}</div>
      </div>
      <div class="seg">
        <button
          class:on={settings.theme === 'auto'}
          onclick={() => setTheme('auto')}
          title={t().common.themeAuto}
          aria-label={t().common.themeAuto}
        >
          <Icon name="monitor" size={13} />
        </button>
        <button
          class:on={settings.theme === 'light'}
          onclick={() => setTheme('light')}
          title={t().common.themeLight}
          aria-label={t().common.themeLight}
        >
          <Icon name="sun" size={13} />
        </button>
        <button
          class:on={settings.theme === 'dark'}
          onclick={() => setTheme('dark')}
          title={t().common.themeDark}
          aria-label={t().common.themeDark}
        >
          <Icon name="moon" size={13} />
        </button>
      </div>
    </div>

    <div class="setting-row">
      <div>
        <div class="sr-label">{t().common.language}</div>
        <div class="sr-sub">Deutsch · English</div>
      </div>
      <div class="seg">
        <button class:on={settings.lang === 'de'} onclick={() => setLang('de')}>DE</button>
        <button class:on={settings.lang === 'en'} onclick={() => setLang('en')}>EN</button>
      </div>
    </div>

    <div class="setting-row">
      <div>
        <div class="sr-label">{t().common.currency}</div>
        <div class="sr-sub">EUR (€)</div>
      </div>
      <span class="btn" aria-disabled="true">EUR</span>
    </div>

    <div class="setting-row">
      <div>
        <div class="sr-label">{t().common.hideAmounts}</div>
        <div class="sr-sub">{t().common.privacy}</div>
      </div>
      <button
        class="toggle"
        class:on={settings.hide}
        onclick={() => setHide(!settings.hide)}
        aria-pressed={settings.hide}
        aria-label={t().common.hideAmounts}
      >
        <span class="knob" class:on={settings.hide}></span>
      </button>
    </div>

    <div class="setting-row">
      <div>
        <div class="sr-label">{t().common.showCents}</div>
        <div class="sr-sub">{t().common.showCentsDesc}</div>
      </div>
      <button
        class="toggle"
        class:on={settings.showCents}
        onclick={() => setShowCents(!settings.showCents)}
        aria-pressed={settings.showCents}
        aria-label={t().common.showCents}
      >
        <span class="knob" class:on={settings.showCents}></span>
      </button>
    </div>
  </div>

  <div class="card col-6 card-pad-lg">
    <div class="card-h"><h3>{t().common.sync}</h3></div>
    <div class="setting-row">
      <div>
        <div class="sr-label">Syncthing</div>
        <div class="sr-sub">{t().common.autoSyncDesc}</div>
      </div>
      <span class="pill up">{t().common.active}</span>
    </div>
    <div class="setting-row">
      <div>
        <div class="sr-label">{t().common.categories}</div>
        <div class="sr-sub">{t().common.manageCats}</div>
      </div>
      <button class="btn" onclick={() => (showCats = true)}>
        {t().common.manageCats} <Icon name="chevron-right" size={12} />
      </button>
    </div>
    <div class="setting-row">
      <div>
        <div class="sr-label">{t().common.rulesC}</div>
        <div class="sr-sub">{t().common.applyToExistingDesc}</div>
      </div>
      <button class="btn" onclick={() => (showRules = true)}>
        {t().common.manageRules} <Icon name="chevron-right" size={12} />
      </button>
    </div>
    <div class="setting-row">
      <div>
        <div class="sr-label">Transfers erkennen</div>
        <div class="sr-sub">Setzt kind = 'transfer' wo Gegenkonto-IBAN zu einem eigenen Konto passt.</div>
      </div>
      <button class="btn" onclick={runDetectTransfers} disabled={detectingTransfers}>
        {detectingTransfers ? '…' : 'Transfers erkennen'}
        {#if detectTransfersResult !== null}
          <span class="detect-result">{detectTransfersResult} aktualisiert</span>
        {/if}
      </button>
    </div>
    <div class="setting-row">
      <div>
        <div class="sr-label">Phantom-Mirror reparieren</div>
        <div class="sr-sub">Findet auto-pair Mirror, für die im echten Datenbestand bereits eine matching Tx im ±3-Tage-Fenster existiert — löscht den Mirror und verlinkt die echte Tx stattdessen.</div>
      </div>
      <button class="btn" onclick={runCleanupPhantomMirrors} disabled={cleaningMirrors}>
        {cleaningMirrors ? '…' : 'Mirror bereinigen'}
        {#if cleanupResult !== null}
          <span class="detect-result">{cleanupResult} repariert</span>
        {/if}
      </button>
    </div>
  </div>

  <div class="card col-12 card-pad-lg">
    <div class="card-h"><h3>{t().onboarding.settingsTitle}</h3></div>
    <div class="setting-row">
      <div>
        <div class="sr-label">{t().onboarding.settingsRestart}</div>
        <div class="sr-sub">{t().onboarding.settingsRestartSub}</div>
      </div>
      <button class="btn" onclick={() => startOnboarding()}>
        {t().onboarding.settingsRestart} <Icon name="chevron-right" size={12} />
      </button>
    </div>
    <div class="setting-row">
      <div>
        <div class="sr-label">{t().onboarding.settingsTour}</div>
        <div class="sr-sub">{t().onboarding.settingsTourSub}</div>
      </div>
      <button class="btn" onclick={() => startTour()}>
        {t().onboarding.settingsTour} <Icon name="chevron-right" size={12} />
      </button>
    </div>
  </div>

  <div class="card col-12 card-pad-lg">
    <div class="card-h"><h3>{t().updates.settingsToggle}</h3></div>

    <div class="setting-row">
      <div>
        <div class="sr-label">{t().updates.channelLabel}</div>
        <div class="sr-sub">{t().updates.channelSub}</div>
      </div>
      <div class="seg">
        <button
          class:on={settings.releaseChannel === 'stable'}
          onclick={() => selectChannel('stable')}
        >{t().updates.channelStable}</button>
        <button
          class:on={settings.releaseChannel === 'beta'}
          onclick={() => selectChannel('beta')}
        >{t().updates.channelBeta}</button>
      </div>
    </div>

    <div class="setting-row">
      <div>
        <div class="sr-label">{t().updates.settingsToggle}</div>
      </div>
      <button
        class="toggle"
        class:on={settings.updateConsent === 'enabled'}
        onclick={toggleAuto}
        aria-pressed={settings.updateConsent === 'enabled'}
        aria-label={t().updates.settingsToggle}
      >
        <span class="knob" class:on={settings.updateConsent === 'enabled'}></span>
      </button>
    </div>

    <div class="setting-row">
      <div>
        <div class="sr-label">{appVersion ? t().updates.currentVersion(appVersion) : ''}</div>
      </div>
      <button class="btn" onclick={onCheckNow}>
        {t().updates.checkNow}
      </button>
    </div>
    <p class="muted" aria-live="polite" aria-atomic="true">{checkMessage}</p>
  </div>

  <div class="card col-12 card-pad-lg">
    <div class="card-h"><h3>{t().common.exportTitle}</h3></div>
    <div class="export-grid">
      <label>
        <span>{t().common.exportDateFrom}</span>
        <DateField bind:value={from} />
      </label>
      <label>
        <span>{t().common.exportDateTo}</span>
        <DateField bind:value={to} />
      </label>
      <label>
        <span>{t().common.account}</span>
        <select bind:value={accFilter}>
          <option value="all">{t().common.exportAccountAll}</option>
          {#each accounts as a (a.id)}
            <option value={a.id}>{a.name}</option>
          {/each}
        </select>
      </label>
      <label>
        <span>{t().common.categories}</span>
        <select bind:value={catFilter}>
          <option value="all">{t().common.exportCategoryAll}</option>
          {#each categories as c (c.id)}
            <option value={c.id}>{c.name}</option>
          {/each}
        </select>
      </label>
      <label class="full">
        <span>{t().common.exportSearch}</span>
        <input type="text" bind:value={searchFilter} placeholder="REWE, Edeka, …" />
      </label>
    </div>
    <div class="export-actions">
      <ExportButton getFilter={buildExportFilter} variant="btn-primary" />
    </div>
  </div>

  <div class="card col-12 card-pad-lg">
    <div class="card-h"><h3>{tp.refreshButton}</h3></div>
    <p class="muted">{tp.refreshHint}</p>
    <KursRefreshButton />
  </div>

  <div class="card col-12 card-pad-lg">
    <div class="card-h"><h3>{t().currencies.title}</h3></div>
    <p class="muted">{t().currencies.rateHint}</p>
    <a href="/settings/currencies" class="link-arrow">{t().currencies.title} →</a>
  </div>

  <div class="card col-12 card-pad-lg">
    <div class="card-h"><h3>{t().data.title}</h3></div>
    <p class="muted">{t().data.backupHint}</p>
    <a href="/settings/data" class="link-arrow">{t().data.navLink}</a>
  </div>

  <div class="card col-12 card-pad-lg">
    <div class="card-h"><h3>Über Notchkeep</h3></div>
    <dl class="about">
      <dt>Lizenz</dt>
      <dd>
        <button type="button" class="link-inline" onclick={() => openExternal(LICENSE_URL)}>
          GPL-3.0-or-later
        </button>
      </dd>
      <dt>Copyright</dt>
      <dd>© 2026 Florian Zollner</dd>
      <dt>Quellcode</dt>
      <dd>
        <button type="button" class="link-arrow" onclick={() => openExternal(SOURCE_URL)}>
          {SOURCE_LABEL}
        </button>
      </dd>
      <dt>Drittanbieter-Lizenzen</dt>
      <dd>
        <button type="button" class="link-arrow" onclick={() => (showThirdParty = true)}>
          Vollständige Liste anzeigen
        </button>
      </dd>
    </dl>
    <p class="warranty">
      Dieses Programm wird in der Hoffnung verteilt, dass es nützlich sein wird,
      jedoch <strong>OHNE JEDE GEWÄHRLEISTUNG</strong>; auch ohne die implizite
      Gewährleistung der MARKTGÄNGIGKEIT oder EIGNUNG FÜR EINEN BESTIMMTEN ZWECK.
      Siehe die GNU General Public License v3 für Details.
    </p>
  </div>

  <div class="col-12 version">{appVersion ? t().common.version(appVersion) : ''}</div>
</div>

{#if showCats}
  <CategoriesModal onClose={() => (showCats = false)} />
{/if}
{#if showRules}
  <RulesModal onClose={() => (showRules = false)} />
{/if}
{#if showThirdParty}
  <LicensesModal onClose={() => (showThirdParty = false)} />
{/if}
{#if showUpdate}
  <UpdateAvailableDialog
    onInstall={() => downloadAndInstall()}
    onSkip={() => { skipCurrent(); showUpdate = false; }}
    onClose={() => (showUpdate = false)}
    onRestart={() => restart()}
  />
{/if}

{#if showChannelWarning}
  <ChannelWarningDialog onConfirm={confirmBeta} onCancel={cancelBeta} />
{/if}

<style>
  .version {
    text-align: center;
    font-size: 11.5px;
    color: var(--text-faint);
    padding: 12px 0;
  }
  .toggle {
    width: 38px;
    height: 22px;
    border-radius: 999px;
    background: var(--border-strong);
    position: relative;
    padding: 0;
    transition: background 0.15s;
  }
  .toggle.on {
    background: var(--accent);
  }
  .knob {
    position: absolute;
    top: 2px;
    left: 2px;
    width: 18px;
    height: 18px;
    border-radius: 50%;
    background: var(--surface);
    box-shadow: var(--shadow-sm);
    transition: transform 0.15s;
  }
  .knob.on {
    transform: translateX(16px);
  }
  .export-grid {
    display: grid;
    grid-template-columns: repeat(2, 1fr);
    gap: 12px;
    margin: 12px 0;
  }
  .export-grid label {
    display: flex;
    flex-direction: column;
    gap: 4px;
    font-size: 12px;
    color: var(--text-faint);
  }
  .export-grid label.full {
    grid-column: 1 / -1;
  }
  .export-grid input,
  .export-grid select {
    padding: 8px;
    border: 1px solid var(--border-strong);
    border-radius: var(--r-sm);
    background: var(--surface);
    color: var(--text);
    font-size: 13px;
  }
  .export-actions {
    display: flex;
    justify-content: flex-end;
    margin-top: 8px;
  }
  .muted {
    font-size: 13px;
    color: var(--text-muted);
    margin-bottom: 12px;
  }
  .link-arrow { color: var(--accent); font-size: 13px; text-decoration: none; background: none; border: 0; padding: 0; cursor: pointer; font-family: inherit; }
  .link-arrow:hover { text-decoration: underline; }
  .detect-result { font-size: 11px; color: var(--positive); margin-left: 8px; }
  .about {
    display: grid;
    grid-template-columns: max-content 1fr;
    gap: 8px 18px;
    margin: 4px 0 14px;
    font-size: 13px;
  }
  .about dt { color: var(--text-muted); font-weight: 500; }
  .about dd { margin: 0; color: var(--text); }
  /* Inline text button styled as a link (license popup trigger). */
  .link-inline {
    background: none;
    border: 0;
    padding: 0;
    font: inherit;
    cursor: pointer;
    color: var(--accent);
    text-decoration: none;
  }
  .link-inline:hover { text-decoration: underline; }
  .warranty {
    font-size: 11.5px;
    color: var(--text-faint);
    line-height: 1.55;
    margin: 0;
    padding: 8px 12px;
    border-left: 3px solid var(--border-strong);
    background: var(--surface-2);
    border-radius: 4px;
  }

  @media (max-width: 599px) {
    /* export-grid: 2-col → 1-col; label.full already spans */
    .export-grid { grid-template-columns: 1fr; }
    .export-grid label.full { grid-column: 1; }

    /* export inputs/buttons touch target */
    .export-grid input,
    .export-grid select {
      min-height: var(--tap, 48px);
    }

    /* about dl: label-value pairs stay side-by-side but allow wrap */
    .about { grid-template-columns: max-content 1fr; gap: 4px 12px; }
  }

  @media (prefers-reduced-motion: reduce) {
    .toggle, .knob { transition: none; }
  }
</style>

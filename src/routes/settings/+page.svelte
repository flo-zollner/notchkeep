<script lang="ts">
  import CategoriesModal from '$lib/components/CategoriesModal.svelte';
  import RulesModal from '$lib/components/RulesModal.svelte';
  import Icon from '$lib/components/Icon.svelte';
  import { settings, setHide, setLang, setShowCents, setTheme, t } from '$lib/settings.svelte';
  import ExportButton from '$lib/components/ExportButton.svelte';
  import KursRefreshButton from '$lib/components/KursRefreshButton.svelte';
  import { api, type Account, type Category, type ExportFilter } from '$lib/api';
  import DateField from '$lib/components/DateField.svelte';
  import { openUrl } from '@tauri-apps/plugin-opener';
  import { onMount } from 'svelte';

  const SOURCE_URL = 'https://github.com/flo-zollner/notchkeep';
  async function openSource() {
    try { await openUrl(SOURCE_URL); } catch { window.open(SOURCE_URL, '_blank'); }
  }

  const tp = $derived(t().portfolio);

  let showCats = $state(false);
  let showRules = $state(false);
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
        <div class="sr-sub">Light · Dark</div>
      </div>
      <div class="seg">
        <button class:on={settings.theme === 'light'} onclick={() => setTheme('light')}>
          <Icon name="sun" size={13} />
        </button>
        <button class:on={settings.theme === 'dark'} onclick={() => setTheme('dark')}>
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
        <a href="/licenses.html" target="_blank" rel="noopener">GPL-3.0-or-later</a>
      </dd>
      <dt>Copyright</dt>
      <dd>© 2026 Florian Zollner</dd>
      <dt>Quellcode</dt>
      <dd>
        <button type="button" class="link-arrow" onclick={openSource}>
          github.com/&lt;TODO&gt;
        </button>
      </dd>
      <dt>Drittanbieter-Lizenzen</dt>
      <dd>
        <a href="/licenses.html" target="_blank" rel="noopener">
          Vollständige Liste anzeigen
        </a>
      </dd>
    </dl>
    <p class="warranty">
      Dieses Programm wird in der Hoffnung verteilt, dass es nützlich sein wird,
      jedoch <strong>OHNE JEDE GEWÄHRLEISTUNG</strong>; auch ohne die implizite
      Gewährleistung der MARKTGÄNGIGKEIT oder EIGNUNG FÜR EINEN BESTIMMTEN ZWECK.
      Siehe die GNU General Public License v3 für Details.
    </p>
  </div>

  <div class="col-12 version">{t().common.version}</div>
</div>

{#if showCats}
  <CategoriesModal onClose={() => (showCats = false)} />
{/if}
{#if showRules}
  <RulesModal onClose={() => (showRules = false)} />
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
    border-radius: 99px;
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
    transition: left 0.15s;
  }
  .knob.on {
    left: 18px;
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
    padding: 6px 8px;
    border: 1px solid var(--border-strong);
    border-radius: 6px;
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
  .detect-result { font-size: 11px; color: var(--positive); margin-left: 6px; }
  .about {
    display: grid;
    grid-template-columns: max-content 1fr;
    gap: 6px 18px;
    margin: 4px 0 14px;
    font-size: 13px;
  }
  .about dt { color: var(--text-muted); font-weight: 500; }
  .about dd { margin: 0; color: var(--text); }
  .about a {
    color: var(--accent);
    text-decoration: none;
  }
  .about a:hover { text-decoration: underline; }
  .warranty {
    font-size: 11.5px;
    color: var(--text-faint);
    line-height: 1.55;
    margin: 0;
    padding: 10px 12px;
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
      min-height: var(--tap, 44px);
    }

    /* about dl: label-value pairs stay side-by-side but allow wrap */
    .about { grid-template-columns: max-content 1fr; gap: 4px 12px; }
  }
</style>

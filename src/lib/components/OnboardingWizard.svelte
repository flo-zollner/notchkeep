<script lang="ts">
  import Sheet from '$lib/components/Sheet.svelte';
  import Icon from '$lib/components/Icon.svelte';
  import { t } from '$lib/settings.svelte';
  import { settings, setLang, setTheme, type Theme } from '$lib/settings.svelte';
  import { onboarding, finishOnboarding, startTour } from '$lib/onboarding/onboarding.svelte';
  import { EXPLAINER_CARDS } from '$lib/onboarding/tour-config';
  import { api, createInstitution } from '$lib/api';

  const o = $derived(t().onboarding);
  const c = $derived(t().common);

  const TOTAL = 5; // welcome, how, account, import, done

  // Step 2 — first account form
  let accountName = $state('');
  let institutionName = $state('');
  let accountKind = $state('bank');
  let creating = $state(false);
  let createdName = $state<string | null>(null);
  let createError = $state(false);

  const kinds = $derived([
    { value: 'bank', label: c.kindBank },
    { value: 'savings', label: c.kindSavings },
    { value: 'broker', label: c.kindBroker },
    { value: 'credit', label: c.kindCredit },
    { value: 'cash', label: c.kindCash },
    { value: 'loan', label: c.kindLoan },
  ]);

  const themes: { value: Theme; label: string; icon: string }[] = [
    { value: 'auto', label: 'auto', icon: 'monitor' },
    { value: 'light', label: 'light', icon: 'sun' },
    { value: 'dark', label: 'dark', icon: 'moon' },
  ];

  function next() {
    if (onboarding.wizardStep < TOTAL - 1) onboarding.wizardStep += 1;
  }
  function back() {
    if (onboarding.wizardStep > 0) onboarding.wizardStep -= 1;
  }
  function skip() {
    finishOnboarding();
  }
  function finishAndClose() {
    finishOnboarding();
  }
  function finishAndTour() {
    finishOnboarding();
    startTour();
  }

  async function createFirstAccount() {
    if (!accountName.trim() || creating) return;
    creating = true;
    createError = false;
    try {
      let institutionId: number | null = null;
      if (institutionName.trim()) {
        const inst = await createInstitution({ name: institutionName.trim() });
        institutionId = inst.id;
      }
      await api.createAccount(accountName.trim(), accountKind, 'EUR', null, null, institutionId);
      createdName = accountName.trim();
      next();
    } catch {
      createError = true;
    } finally {
      creating = false;
    }
  }
</script>

{#if onboarding.wizardActive}
  <Sheet open onClose={skip} dismissable={false}>
    {#snippet header()}
      <div class="ob-header">
        <span class="ob-progress">{o.stepOf(onboarding.wizardStep + 1, TOTAL)}</span>
        <button class="ob-skip" onclick={skip}>{o.skip}</button>
      </div>
      <div class="ob-bar" role="presentation">
        {#each Array(TOTAL) as _, i (i)}
          <span class="ob-seg" class:done={i <= onboarding.wizardStep}></span>
        {/each}
      </div>
    {/snippet}

    <div class="ob-body" data-testid="onboarding-wizard">
      {#if onboarding.wizardStep === 0}
        <!-- Welcome -->
        <div class="ob-hero">
          <div class="ob-logo">N</div>
          <h2>{o.welcomeTitle}</h2>
          <p>{o.welcomeBody}</p>
        </div>
        <div class="ob-quick">
          <div class="ob-quick-row">
            <span class="ob-quick-label">{o.welcomeLang}</span>
            <div class="seg">
              <button class:on={settings.lang === 'de'} onclick={() => setLang('de')}>DE</button>
              <button class:on={settings.lang === 'en'} onclick={() => setLang('en')}>EN</button>
            </div>
          </div>
          <div class="ob-quick-row">
            <span class="ob-quick-label">{o.welcomeTheme}</span>
            <div class="seg">
              {#each themes as th (th.value)}
                <button
                  class:on={settings.theme === th.value}
                  onclick={() => setTheme(th.value)}
                  aria-label={th.label}
                >
                  <Icon name={th.icon} size={13} />
                </button>
              {/each}
            </div>
          </div>
        </div>
      {:else if onboarding.wizardStep === 1}
        <!-- How it works -->
        <h2>{o.howTitle}</h2>
        <p class="ob-sub">{o.howBody}</p>
        <div class="ob-cards">
          {#each EXPLAINER_CARDS as card (card.id)}
            <div class="ob-card">
              <div class="ob-card-icon"><Icon name={card.icon} size={18} /></div>
              <div>
                <div class="ob-card-title">{o.cards[card.id].title}</div>
                <div class="ob-card-body">{o.cards[card.id].body}</div>
              </div>
            </div>
          {/each}
        </div>
      {:else if onboarding.wizardStep === 2}
        <!-- First account -->
        <h2>{o.accountTitle}</h2>
        <p class="ob-sub">{o.accountBody}</p>
        <div class="ob-form">
          <label>
            <span>{o.accountNameLabel}</span>
            <input type="text" bind:value={accountName} placeholder={o.accountNamePlaceholder} />
          </label>
          <label>
            <span>{o.institutionLabel}</span>
            <input type="text" bind:value={institutionName} placeholder={o.institutionPlaceholder} />
          </label>
          <label>
            <span>{o.accountKindLabel}</span>
            <select bind:value={accountKind}>
              {#each kinds as k (k.value)}
                <option value={k.value}>{k.label}</option>
              {/each}
            </select>
          </label>
          {#if createError}
            <div class="ob-error">{o.accountError}</div>
          {/if}
        </div>
      {:else if onboarding.wizardStep === 3}
        <!-- Import hint -->
        <div class="ob-hero">
          <div class="ob-card-icon big"><Icon name="download" size={26} /></div>
          <h2>{o.importTitle}</h2>
          <p>{o.importBody}</p>
        </div>
      {:else}
        <!-- Done -->
        <div class="ob-hero">
          <div class="ob-card-icon big done"><Icon name="check" size={26} /></div>
          <h2>{o.doneTitle}</h2>
          <p>{o.doneBody}</p>
          {#if createdName}
            <div class="ob-created">{o.accountCreated(createdName)}</div>
          {/if}
        </div>
      {/if}
    </div>

    {#snippet footer()}
      <div class="ob-footer">
        {#if onboarding.wizardStep > 0}
          <button class="btn" onclick={back}>{o.back}</button>
        {:else}
          <span></span>
        {/if}

        <div class="ob-footer-right">
          {#if onboarding.wizardStep === 2}
            <button class="btn" onclick={next}>{o.skip}</button>
            <button class="btn primary" onclick={createFirstAccount} disabled={!accountName.trim() || creating}>
              {creating ? o.creating : o.createAccount}
            </button>
          {:else if onboarding.wizardStep === TOTAL - 1}
            <button class="btn" onclick={finishAndClose}>{o.finish}</button>
            <button class="btn primary" onclick={finishAndTour}>{o.startTour}</button>
          {:else}
            <button class="btn primary" onclick={next}>{o.next}</button>
          {/if}
        </div>
      </div>
    {/snippet}
  </Sheet>
{/if}

<style>
  .ob-header {
    display: flex; align-items: center; justify-content: space-between;
    padding: 12px 18px 8px;
  }
  .ob-progress { font-size: 12px; color: var(--text-muted); font-weight: 500; }
  .ob-skip {
    font-size: 13px; color: var(--text-muted); background: none; border: 0;
    padding: 4px 8px; cursor: pointer; border-radius: var(--r-sm);
    min-height: var(--tap);
  }
  .ob-skip:hover { color: var(--text); }
  .ob-bar { display: flex; gap: 4px; padding: 0 18px 4px; }
  .ob-seg { flex: 1; height: 3px; border-radius: 2px; background: var(--border); transition: background 0.2s; }
  .ob-seg.done { background: var(--accent); }

  .ob-body { min-height: 320px; }
  .ob-body h2 { margin: 0 0 6px; font-size: 18px; font-weight: 650; }
  .ob-sub { margin: 0 0 16px; font-size: 13.5px; color: var(--text-muted); }

  .ob-hero { text-align: center; padding: 18px 8px; }
  .ob-hero h2 { font-size: 20px; }
  .ob-hero p { font-size: 14px; color: var(--text-muted); line-height: 1.6; max-width: 38ch; margin: 8px auto 0; }
  .ob-logo {
    width: 56px; height: 56px; border-radius: 16px; margin: 0 auto 14px;
    background: var(--accent); color: #fff; display: grid; place-items: center;
    font-size: 28px; font-weight: 700; box-shadow: var(--shadow-md);
  }

  .ob-quick { display: flex; flex-direction: column; gap: 12px; margin-top: 22px; }
  .ob-quick-row { display: flex; align-items: center; justify-content: space-between; }
  .ob-quick-label { font-size: 13.5px; color: var(--text); }
  .seg { display: inline-flex; border: 1px solid var(--border-strong); border-radius: var(--r-sm); overflow: hidden; }
  .seg button {
    padding: 6px 12px; font-size: 12.5px; background: var(--surface); color: var(--text-muted);
    border: 0; cursor: pointer; min-height: 32px; display: inline-flex; align-items: center; gap: 4px;
  }
  .seg button.on { background: var(--accent); color: #fff; }

  .ob-cards { display: grid; grid-template-columns: 1fr 1fr; gap: 10px; }
  .ob-card {
    display: flex; gap: 11px; padding: 12px;
    border: 1px solid var(--border); border-radius: var(--r-md); background: var(--surface-2);
  }
  .ob-card-icon {
    flex: none; width: 34px; height: 34px; border-radius: 9px;
    background: color-mix(in srgb, var(--accent) 14%, transparent); color: var(--accent);
    display: grid; place-items: center;
  }
  .ob-card-icon.big { width: 56px; height: 56px; border-radius: 16px; margin: 0 auto 6px; }
  .ob-card-icon.big.done { background: color-mix(in srgb, var(--positive) 16%, transparent); color: var(--positive); }
  .ob-card-title { font-size: 13.5px; font-weight: 600; margin-bottom: 3px; }
  .ob-card-body { font-size: 12px; color: var(--text-muted); line-height: 1.45; }

  .ob-form { display: flex; flex-direction: column; gap: 12px; max-width: 360px; }
  .ob-form label { display: flex; flex-direction: column; gap: 4px; font-size: 12px; color: var(--text-faint); }
  .ob-form input, .ob-form select {
    padding: 9px 10px; border: 1px solid var(--border-strong); border-radius: 7px;
    background: var(--surface); color: var(--text); font-size: 14px; min-height: var(--tap, 44px);
  }
  .ob-error { font-size: 12.5px; color: var(--negative); }
  .ob-created {
    margin-top: 14px; font-size: 13px; color: var(--positive);
    background: color-mix(in srgb, var(--positive) 10%, transparent);
    padding: 8px 12px; border-radius: var(--r-sm); display: inline-block;
  }

  .ob-footer { display: flex; align-items: center; justify-content: space-between; gap: 8px; width: 100%; }
  .ob-footer-right { display: flex; gap: 8px; }

  @media (max-width: 599px) {
    .ob-cards { grid-template-columns: 1fr; }
    .ob-body { min-height: auto; }
    .ob-footer-right { flex: 1; }
    .ob-footer-right .btn.primary { flex: 1; min-height: var(--tap); white-space: normal; }
  }
</style>

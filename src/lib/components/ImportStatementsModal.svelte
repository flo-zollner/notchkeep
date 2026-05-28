<script lang="ts">
  import Icon from './Icon.svelte';
  import { t } from '$lib/settings.svelte';
  import { api, errMsg, type Account, type ImportReport, type Institution } from '$lib/api';
  import Sheet from './Sheet.svelte';

  interface Props {
    accounts: Account[];
    institutions: Institution[];
    defaultAccountId?: number | null;
    onClose: () => void;
    onImported: () => void;
  }
  let { accounts, institutions, defaultAccountId = null, onClose, onImported }: Props = $props();

  const tc = $derived(t().common);

  type Parser = 'flatex' | 'tr' | 'sparkasse';
  /* svelte-ignore state_referenced_locally */
  let parser = $state<Parser>('flatex');
  /* svelte-ignore state_referenced_locally */
  let accountId = $state<number | null>(defaultAccountId);
  let pickedFiles = $state<File[]>([]);
  let busy = $state(false);
  let error = $state<string | null>(null);
  let report = $state<ImportReport | null>(null);
  let fileInputEl: HTMLInputElement;

  // Parser → expected institution (name as in db/institutions). When the account
  // belongs to this institution, the variant B routing places trades on the correct
  // depot. A mismatch (e.g. Flatex PDF assigned to a TR account) would route trades
  // incorrectly — therefore the dropdown is pre-filtered.
  const expectedInstitution = $derived(
    parser === 'flatex' ? 'flatexDEGIRO'
    : parser === 'tr' ? 'Trade Republic'
    : 'Erste Bank / Sparkasse'
  );

  const expectedInstitutionId = $derived.by(() => {
    const inst = institutions.find(
      (i) => i.name.toLowerCase() === expectedInstitution.toLowerCase()
    );
    return inst?.id ?? null;
  });

  // Filter: bank/broker/savings AND belonging to the expected institution.
  // Accounts without an institution are also offered (legacy / not yet assigned).
  const eligibleAccounts = $derived(
    accounts.filter((a) => {
      if (a.archived) return false;
      if (!['bank', 'broker', 'savings'].includes(a.kind)) return false;
      if (expectedInstitutionId === null) return true;
      // Match: account institution == expected institution, OR account has no institution yet
      return a.institution_id === expectedInstitutionId || a.institution_id === null;
    })
  );

  // On parser change: reset accountId if the current selection is no longer
  // in the filtered list.
  $effect(() => {
    parser; // re-evaluate
    if (accountId !== null && !eligibleAccounts.some((a) => a.id === accountId)) {
      accountId = null;
    }
  });

  const accept = $derived(parser === 'flatex' ? '.pdf,application/pdf' : '.csv,text/csv');
  const multiple = $derived(parser === 'flatex');   // TR + Sparkasse: 1 Datei
  const canSubmit = $derived(
    !busy && accountId !== null && pickedFiles.length > 0
  );

  function openPicker() {
    error = null;
    report = null;
    if (fileInputEl) fileInputEl.click();
  }

  function onFilesChosen(e: Event) {
    const input = e.currentTarget as HTMLInputElement;
    const list = input.files;
    if (!list) {
      pickedFiles = [];
      return;
    }
    const arr: File[] = [];
    for (let i = 0; i < list.length; i++) arr.push(list[i]);
    // Sort by filename (chronological thanks to Flatex naming "YYYYMMDD_…").
    // Makes the import deterministic and FIFO-stable for same-day transactions.
    arr.sort((a, b) => a.name.localeCompare(b.name));
    pickedFiles = arr;
  }

  function removeFile(idx: number) {
    pickedFiles = pickedFiles.filter((_, i) => i !== idx);
  }

  function onParserChange() {
    // On parser change: reset files (different format)
    pickedFiles = [];
    report = null;
    error = null;
    if (fileInputEl) fileInputEl.value = '';
  }

  async function submit() {
    if (!canSubmit || accountId === null) return;
    busy = true;
    error = null;
    report = null;
    try {
      let result: ImportReport;
      if (parser === 'flatex') {
        const bytesArr: Uint8Array[] = [];
        for (const f of pickedFiles) {
          bytesArr.push(new Uint8Array(await f.arrayBuffer()));
        }
        result = await api.importFlatexPdfs(accountId, bytesArr);
      } else if (parser === 'tr') {
        const f = pickedFiles[0];
        const bytes = new Uint8Array(await f.arrayBuffer());
        result = await api.importTradeRepublicCsv(accountId, bytes);
      } else {
        // sparkasse
        const f = pickedFiles[0];
        const bytes = new Uint8Array(await f.arrayBuffer());
        result = await api.importSparkasseCsv(accountId, bytes);
      }
      report = result;
      onImported();
    } catch (e) {
      error = errMsg(e);
    } finally {
      busy = false;
    }
  }


</script>

<Sheet open={true} {onClose} title={tc.importStatements} dismissable={!busy}>
    {#snippet footer()}
      <div class="footer-actions">
        <button type="button" onclick={onClose} disabled={busy}>
          {report ? tc.close : tc.cancel}
        </button>
        {#if !report}
          <button type="button" class="primary" onclick={submit} disabled={!canSubmit}>
            {busy ? '…' : tc.importStatements}
          </button>
        {/if}
      </div>
    {/snippet}
    <div class="body">
      <label class="field">
        <span class="field-label">{tc.parser}</span>
        <div class="seg" role="group">
          <button
            type="button"
            class:on={parser === 'flatex'}
            onclick={() => { parser = 'flatex'; onParserChange(); }}
            disabled={busy}
          >
            {tc.parserFlatex}
          </button>
          <button
            type="button"
            class:on={parser === 'tr'}
            onclick={() => { parser = 'tr'; onParserChange(); }}
            disabled={busy}
          >
            {tc.parserTr}
          </button>
          <button
            type="button"
            class:on={parser === 'sparkasse'}
            onclick={() => { parser = 'sparkasse'; onParserChange(); }}
            disabled={busy}
          >
            {tc.parserSparkasse}
          </button>
        </div>
      </label>

      <label class="field">
        <span class="field-label">{tc.account}</span>
        <select
          class="input"
          value={accountId === null ? '' : String(accountId)}
          onchange={(e) => {
            const v = (e.currentTarget as HTMLSelectElement).value;
            accountId = v === '' ? null : Number(v);
          }}
          disabled={busy}
        >
          <option value="">—</option>
          {#each eligibleAccounts as a (a.id)}
            <option value={String(a.id)}>{a.name}</option>
          {/each}
        </select>
        {#if eligibleAccounts.length === 0}
          <span class="hint warn">
            Keine Konten für Institut „{expectedInstitution}" gefunden. Lege erst ein Konto an.
          </span>
        {/if}
      </label>

      <div class="field">
        <span class="field-label">
          {tc.selectFiles}
          <span class="hint">· {tc.parserAccept(parser)}{multiple ? ` · ${tc.parserFlatex}: mehrere möglich` : ''}</span>
        </span>
        <input
          type="file"
          {accept}
          {multiple}
          bind:this={fileInputEl}
          onchange={onFilesChosen}
          style="display: none;"
        />
        <button
          type="button"
          class="file-btn"
          onclick={openPicker}
          disabled={busy}
        >
          <Icon name="plus" size={13} />
          {pickedFiles.length === 0 ? tc.selectFiles : tc.selectedFilesCount(pickedFiles.length)}
        </button>
        {#if pickedFiles.length > 0}
          <ul class="files">
            {#each pickedFiles as f, i (i)}
              <li>
                <span class="fname">{f.name}</span>
                <span class="fsize">{(f.size / 1024).toFixed(0)} KB</span>
                <button type="button" class="rm" onclick={() => removeFile(i)} disabled={busy} aria-label="remove">
                  <Icon name="x" size={11} />
                </button>
              </li>
            {/each}
          </ul>
        {/if}
      </div>

      {#if report}
        <div class="report ok">
          <strong>{tc.importDone}</strong>
          <span>· {tc.parsed}: {report.parsed}</span>
          <span>· {tc.inserted}: {report.inserted}</span>
          <span>· {tc.skipped}: {report.skipped}</span>
        </div>
        {#if report.warnings.length > 0}
          <details class="warnings">
            <summary>{tc.importWarnings(report.warnings.length)}</summary>
            <ul>
              {#each report.warnings as w}<li>{w}</li>{/each}
            </ul>
          </details>
        {/if}
      {/if}

      {#if error}
        <p class="err">{error}</p>
      {/if}
    </div>
</Sheet>

<style>
  .body { display: flex; flex-direction: column; gap: 14px; }
  .field { display: flex; flex-direction: column; gap: 6px; }
  .field-label {
    font-size: 12px; color: var(--text-muted);
    display: flex; align-items: baseline; gap: 4px;
  }
  .field-label .hint {
    font-size: 11px; color: var(--text-faint); font-weight: normal;
  }
  .hint.warn {
    font-size: 12px;
    color: var(--warning, #d97706);
    margin-top: 4px;
  }
  .seg {
    display: inline-flex;
    border: 1px solid var(--border);
    border-radius: 8px;
    overflow: hidden;
    align-self: flex-start;
  }
  .seg button {
    border: 0; background: transparent;
    padding: 6px 12px; cursor: pointer;
    font-size: 13px; color: var(--text-muted);
  }
  .seg button.on {
    background: var(--accent-soft, var(--surface-2));
    color: var(--text);
    font-weight: 600;
  }
  .seg button:disabled { opacity: .5; cursor: not-allowed; }
  .file-btn {
    display: inline-flex; align-items: center; gap: 6px;
    padding: 8px 12px;
    border: 1px dashed var(--border);
    background: var(--surface-2);
    border-radius: 8px;
    cursor: pointer; color: var(--text);
    align-self: flex-start;
    font-size: 13px;
  }
  .file-btn:disabled { opacity: .5; cursor: not-allowed; }
  .files {
    list-style: none; margin: 0; padding: 0;
    display: flex; flex-direction: column; gap: 4px;
  }
  .files li {
    display: flex; align-items: center; gap: 8px;
    padding: 6px 10px;
    background: var(--surface-2);
    border-radius: 6px;
    font-size: 12px;
  }
  .fname { flex: 1; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .fsize { color: var(--text-faint); }
  .rm {
    width: 22px; height: 22px;
    border: 0; background: transparent; cursor: pointer;
    color: var(--text-faint);
    display: grid; place-items: center; border-radius: 4px;
  }
  .rm:hover { color: var(--text); background: var(--surface); }
  .report {
    padding: 10px 12px;
    background: var(--positive-soft, var(--surface-2));
    border-radius: 8px;
    font-size: 13px;
    display: flex; flex-wrap: wrap; gap: 6px;
  }
  .err {
    margin: 0; padding: 10px 12px;
    background: var(--negative-soft);
    color: var(--negative);
    border-radius: 8px; font-size: 13px;
  }
  .footer-actions {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
    flex-wrap: nowrap;
  }
  @media (max-width: 600px) {
    .footer-actions button { flex: 1 1 0; min-width: 0; }
  }
  .footer-actions button {
    background: var(--surface-2);
    border: 1px solid var(--border);
    padding: 8px 14px;
    border-radius: 8px;
    cursor: pointer;
    color: var(--text);
    font-size: 13px;
  }
  .footer-actions button:disabled { opacity: .5; cursor: not-allowed; }
  .footer-actions .primary {
    background: var(--accent);
    color: var(--accent-fg, white);
    border-color: var(--accent);
    font-weight: 600;
  }
  .input {
    padding: 6px 10px;
    border: 1px solid var(--border);
    border-radius: 6px;
    background: var(--surface);
    color: var(--text);
    font: inherit;
  }
  .warnings {
    margin-top: 8px;
    padding: 8px 12px;
    background: color-mix(in srgb, var(--warning, #d97706) 12%, transparent);
    border-radius: 8px;
    font-size: 12px;
  }
  .warnings summary {
    cursor: pointer;
    color: var(--warning, #d97706);
    font-weight: 600;
  }
  .warnings ul {
    margin: 6px 0 0;
    padding-left: 18px;
    color: var(--text);
  }
  .warnings li {
    margin: 2px 0;
  }
</style>

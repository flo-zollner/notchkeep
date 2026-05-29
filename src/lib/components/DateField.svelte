<script lang="ts">
  import { t } from '$lib/settings.svelte';

  interface Props {
    value: string;
    min?: string;
    max?: string;
    disabled?: boolean;
    /** Optional extra CSS class for the outer wrapper. */
    class?: string;
    /** Callback on value change (analogous to native onchange). */
    onChange?: (value: string) => void;
  }
  let {
    value = $bindable(''),
    min,
    max,
    disabled = false,
    class: cls = '',
    onChange,
  }: Props = $props();

  let open = $state(false);
  let inputEl: HTMLInputElement | undefined;
  let popupEl: HTMLDivElement | undefined = $state();

  function parseIso(s: string): { y: number; m: number; d: number } | null {
    const mt = s?.match(/^(\d{4})-(\d{2})-(\d{2})$/);
    if (!mt) return null;
    return { y: +mt[1], m: +mt[2], d: +mt[3] };
  }
  function toIso(y: number, m: number, d: number): string {
    return `${String(y).padStart(4, '0')}-${String(m).padStart(2, '0')}-${String(d).padStart(2, '0')}`;
  }
  function daysInMonth(y: number, m: number): number {
    return new Date(y, m, 0).getDate();
  }
  function dowMondayFirst(y: number, m: number, d: number): number {
    return (new Date(y, m - 1, d).getDay() + 6) % 7;
  }

  const today = new Date();
  const todayY = today.getFullYear();
  const todayM = today.getMonth() + 1;
  const todayD = today.getDate();

  let viewY = $state(todayY);
  let viewM = $state(todayM);

  $effect(() => {
    if (!open) return;
    const v = parseIso(value);
    if (v) { viewY = v.y; viewM = v.m; }
    else { viewY = todayY; viewM = todayM; }
  });

  $effect(() => {
    if (!open) return;
    function handler(e: PointerEvent) {
      const target = e.target as Node | null;
      if (!target) return;
      if (popupEl?.contains(target)) return;
      if (inputEl?.contains(target)) return;
      open = false;
    }
    document.addEventListener('pointerdown', handler, true);
    return () => document.removeEventListener('pointerdown', handler, true);
  });

  function commitDate(y: number, m: number, d: number) {
    value = toIso(y, m, d);
    open = false;
    inputEl?.focus();
    onChange?.(value);
  }

  function onInputKeydown(e: KeyboardEvent) {
    if (e.key === 'Escape') { open = false; e.preventDefault(); return; }
    if (e.key === 'Enter') { open = false; e.preventDefault(); return; }
  }

  function stepMonth(delta: number) {
    let m = viewM + delta;
    let y = viewY;
    while (m < 1) { m += 12; y -= 1; }
    while (m > 12) { m -= 12; y += 1; }
    viewY = y;
    viewM = m;
  }

  function isDisabledCell(y: number, m: number, d: number): boolean {
    const iso = toIso(y, m, d);
    if (min && iso < min) return true;
    if (max && iso > max) return true;
    return false;
  }

  type Cell = { y: number; m: number; d: number; current: boolean };
  const grid = $derived.by<Cell[]>(() => {
    const dim = daysInMonth(viewY, viewM);
    const firstDow = dowMondayFirst(viewY, viewM, 1);
    const cells: Cell[] = [];
    const prevM = viewM === 1 ? 12 : viewM - 1;
    const prevY = viewM === 1 ? viewY - 1 : viewY;
    const prevDim = daysInMonth(prevY, prevM);
    for (let i = firstDow - 1; i >= 0; i--) {
      cells.push({ y: prevY, m: prevM, d: prevDim - i, current: false });
    }
    for (let d = 1; d <= dim; d++) cells.push({ y: viewY, m: viewM, d, current: true });
    const nextM = viewM === 12 ? 1 : viewM + 1;
    const nextY = viewM === 12 ? viewY + 1 : viewY;
    let n = 1;
    while (cells.length < 42) cells.push({ y: nextY, m: nextM, d: n++, current: false });
    return cells;
  });

  const sel = $derived(parseIso(value));
  const titleText = $derived(`${t().months[viewM - 1]} ${viewY}`);
  const DOW = ['Mo', 'Di', 'Mi', 'Do', 'Fr', 'Sa', 'So'];

  function clickOnInput() {
    if (disabled) return;
    open = true;
  }
</script>

<div class="datefield {cls}">
  <input
    bind:this={inputEl}
    type="text"
    class="df-input"
    {value}
    {disabled}
    inputmode="numeric"
    placeholder="JJJJ-MM-TT"
    pattern={'\\d{4}-\\d{2}-\\d{2}'}
    onfocus={clickOnInput}
    onclick={clickOnInput}
    onkeydown={onInputKeydown}
    oninput={(e) => { value = (e.currentTarget as HTMLInputElement).value; onChange?.(value); }}
  />
  {#if open}
    <div bind:this={popupEl} class="df-popup" role="dialog">
      <div class="df-head">
        <button type="button" class="df-nav" onclick={() => stepMonth(-1)} aria-label="◀">◀</button>
        <span class="df-title">{titleText}</span>
        <button type="button" class="df-nav" onclick={() => stepMonth(1)} aria-label="▶">▶</button>
      </div>
      <div class="df-dow">
        {#each DOW as d (d)}<span>{d}</span>{/each}
      </div>
      <div class="df-grid">
        {#each grid as cell, i (i)}
          {@const isSel = sel?.y === cell.y && sel?.m === cell.m && sel?.d === cell.d}
          {@const isTd = cell.y === todayY && cell.m === todayM && cell.d === todayD}
          {@const dis = isDisabledCell(cell.y, cell.m, cell.d)}
          <button
            type="button"
            class="df-cell"
            class:other={!cell.current}
            class:sel={isSel}
            class:today={isTd}
            disabled={dis}
            onclick={() => commitDate(cell.y, cell.m, cell.d)}
          >{cell.d}</button>
        {/each}
      </div>
    </div>
  {/if}
</div>

<style>
  .datefield {
    position: relative;
    display: inline-block;
  }
  .df-input {
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: 6px;
    padding: 4px 8px;
    font: inherit;
    font-size: 13px;
    color: var(--text);
    width: 7.5em;
    box-sizing: border-box;
  }
  .df-input:focus {
    outline: 1px solid var(--accent);
    outline-offset: -1px;
  }
  .df-popup {
    position: absolute;
    top: calc(100% + 4px);
    left: 0;
    z-index: 200;
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: 8px;
    box-shadow: 0 8px 24px rgba(0, 0, 0, 0.2);
    padding: 8px;
    width: 224px;
    font-size: 12px;
  }
  .df-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: 6px;
  }
  .df-title {
    font-weight: 500;
    color: var(--text);
  }
  .df-nav {
    background: transparent;
    border: 1px solid var(--border);
    border-radius: 4px;
    padding: 2px 6px;
    color: var(--text-muted);
    cursor: pointer;
    font: inherit;
    font-size: 11px;
  }
  .df-nav:hover { color: var(--text); border-color: var(--accent); }
  .df-dow {
    display: grid;
    grid-template-columns: repeat(7, 1fr);
    gap: 2px;
    color: var(--text-faint);
    font-size: 10px;
    text-align: center;
    margin-bottom: 2px;
  }
  .df-grid {
    display: grid;
    grid-template-columns: repeat(7, 1fr);
    gap: 2px;
  }
  .df-cell {
    background: transparent;
    border: 1px solid transparent;
    border-radius: 4px;
    padding: 4px 0;
    font: inherit;
    font-size: 12px;
    color: var(--text);
    cursor: pointer;
    text-align: center;
  }
  .df-cell.other { color: var(--text-faint); }
  .df-cell.today { border-color: var(--border); font-weight: 600; }
  .df-cell.sel { background: var(--accent); color: var(--surface); }
  .df-cell:hover:not(:disabled):not(.sel) {
    background: var(--surface-2);
  }
  .df-cell:disabled {
    color: var(--text-faint);
    opacity: 0.4;
    cursor: not-allowed;
  }
</style>

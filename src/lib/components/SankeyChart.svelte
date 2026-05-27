<script lang="ts">
  import { goto } from '$app/navigation';
  import { type CashflowSlice, type Category, fmtEur } from '$lib/api';
  import { settings, t, eurDecimals } from '$lib/settings.svelte';

  interface Props {
    breakdown: CashflowSlice[];
    categories: Category[];
    /** ISO YYYY-MM-DD — inklusiv Start des Zeitraums (für Navigation zu /transactions). */
    from?: string;
    /** ISO YYYY-MM-DD — exklusiv Ende des Zeitraums (Backend-Konvention); für UI-Filter ziehen wir 1 Tag ab. */
    to?: string;
  }
  let { breakdown, categories, from, to }: Props = $props();

  /** Backend nutzt halboffenes Intervall [from, to). UI-Filter ist inklusiv → 1 Tag abziehen. */
  function exclusiveToInclusive(iso: string | undefined): string | undefined {
    if (!iso) return undefined;
    const d = new Date(iso);
    if (Number.isNaN(d.getTime())) return iso;
    d.setDate(d.getDate() - 1);
    return d.toISOString().slice(0, 10);
  }

  let expanded = $state<Set<number>>(new Set());
  /** Set von Seiten ('income' | 'expense'), bei denen Sonstige aufgeklappt ist. */
  let otherExpanded = $state<Set<'income' | 'expense'>>(new Set());
  let hoverNodeId = $state<string | null>(null);

  // Fallback-Paletten falls Kategorie keine eigene Farbe hat.
  const PALETTE_INCOME = [
    '#10b981', '#22c55e', '#84cc16', '#14b8a6', '#06b6d4',
  ];
  const PALETTE_EXPENSE = [
    '#ef4444', '#f59e0b', '#a855f7', '#ec4899', '#6366f1',
    '#0ea5e9', '#fb7185', '#facc15', '#8b5cf6', '#94a3b8',
  ];
  /** Wenn ein Bucket weniger als dieser Anteil der Seitensumme hat, wird er in "Sonstige" gemergt. */
  const SMALL_THRESHOLD = 0.02;

  const catsById = $derived(new Map(categories.map((c) => [c.id, c])));
  const tc = $derived(t().cashflow);

  function categoryName(id: number | null): string {
    if (id === null) return tc.sankeyOther;
    return catsById.get(id)?.name ?? `#${id}`;
  }

  function categoryColor(id: number | null, fallback: string): string {
    if (id === null) return fallback;
    return catsById.get(id)?.color ?? fallback;
  }

  function effectiveCategoryId(catId: number | null): number | null {
    if (catId === null) return null;
    const cat = catsById.get(catId);
    if (!cat || cat.parent_id === null) return catId;
    if (expanded.has(cat.parent_id)) return catId;
    return cat.parent_id;
  }

  function hasChildren(id: number): boolean {
    return categories.some((c) => c.parent_id === id);
  }

  function toggleExpand(id: number) {
    if (expanded.has(id)) {
      expanded.delete(id);
    } else {
      expanded.add(id);
    }
    expanded = new Set(expanded);
  }

  function resetExpansion() {
    expanded = new Set();
    otherExpanded = new Set();
  }

  function toggleOtherExpand(side: 'income' | 'expense') {
    if (otherExpanded.has(side)) {
      otherExpanded.delete(side);
    } else {
      otherExpanded.add(side);
    }
    otherExpanded = new Set(otherExpanded);
  }

  /**
   * Berechnet die perceived Luminanz eines Hex-Hex-Color-Strings, returnt
   * eine passende Vordergrund-Farbe (schwarz für helle Backgrounds, weiß für dunkle).
   */
  function contrastText(bgHex: string): string {
    // Akzeptiert #rgb und #rrggbb
    let hex = bgHex.replace('#', '');
    if (hex.length === 3) hex = hex.split('').map((c) => c + c).join('');
    if (hex.length !== 6) return '#fff';
    const r = parseInt(hex.slice(0, 2), 16);
    const g = parseInt(hex.slice(2, 4), 16);
    const b = parseInt(hex.slice(4, 6), 16);
    // Standard relative-luminance formula (sRGB)
    const lum = (0.299 * r + 0.587 * g + 0.114 * b) / 255;
    return lum > 0.6 ? '#1a1a1a' : '#fff';
  }

  /** Navigation zu /transactions mit Filter auf Kategorie + Zeitraum. */
  function navigateToTx(categoryId: number | null) {
    if (categoryId === null || categoryId < 0) return;  // null + synthetisches Sonstige (-999999) nicht navigierbar
    const params = new URLSearchParams();
    params.set('categoryId', String(categoryId));
    if (from) params.set('from', from);
    const toIncl = exclusiveToInclusive(to);
    if (toIncl) params.set('to', toIncl);
    goto(`/transactions?${params.toString()}`);
  }

  function onNodeClick(n: { categoryId: number | null; canExpand: boolean; side: 'income' | 'cashflow' | 'expense' }) {
    // Sonstige-Sentinel: toggle Other-Expand
    if (n.categoryId === -999_999 && (n.side === 'income' || n.side === 'expense')) {
      toggleOtherExpand(n.side);
      return;
    }
    if (n.canExpand && n.categoryId !== null) {
      toggleExpand(n.categoryId);
    } else {
      navigateToTx(n.categoryId);
    }
  }

  type Bucket = { key: string; categoryId: number | null; sign: 1 | -1; sumAbsCents: number };

  /** Roll-up auf effective parent based on expansion-set. */
  const rolled = $derived.by<Bucket[]>(() => {
    const map = new Map<string, Bucket>();
    for (const s of breakdown) {
      const eff = effectiveCategoryId(s.categoryId);
      const key = `${eff ?? 'null'}-${s.sign}`;
      const existing = map.get(key);
      if (existing) {
        existing.sumAbsCents += s.sumAbsCents;
      } else {
        map.set(key, { key, categoryId: eff, sign: s.sign, sumAbsCents: s.sumAbsCents });
      }
    }
    return Array.from(map.values()).sort((a, b) => b.sumAbsCents - a.sumAbsCents);
  });

  /**
   * "Sonstige"-Auto-Bucket: mergt Slices < SMALL_THRESHOLD der Seitensumme.
   * Bei `expandOther = true` werden die kleinen Cats stattdessen einzeln gerendert
   * + ein Marker-Bucket "Sonstige (zuklappen)" zum Toggle.
   */
  function mergeSmallToOther(buckets: Bucket[], sign: 1 | -1, expandOther: boolean): Bucket[] {
    if (buckets.length <= 4) return buckets;
    const total = buckets.reduce((s, b) => s + b.sumAbsCents, 0);
    if (total === 0) return buckets;
    const threshold = total * SMALL_THRESHOLD;
    const big: Bucket[] = [];
    const smalls: Bucket[] = [];
    let otherSum = 0;
    for (const b of buckets) {
      if (b.sumAbsCents >= threshold || b.categoryId === null) {
        big.push(b);
      } else {
        smalls.push(b);
        otherSum += b.sumAbsCents;
      }
    }
    if (smalls.length === 0) return big.sort((a, b) => b.sumAbsCents - a.sumAbsCents);
    if (smalls.length === 1) {
      big.push(smalls[0]);
      return big.sort((a, b) => b.sumAbsCents - a.sumAbsCents);
    }
    if (expandOther) {
      // Aufgeklappt: alle kleinen Cats einzeln zeigen.
      big.push(...smalls);
    } else if (otherSum > 0) {
      // Zugeklappt: ein Sonstige-Bucket.
      big.push({
        key: `other-${sign}`,
        categoryId: -999_999,  // Sentinel
        sign,
        sumAbsCents: otherSum,
      });
    }
    return big.sort((a, b) => b.sumAbsCents - a.sumAbsCents);
  }

  const income = $derived(mergeSmallToOther(rolled.filter((b) => b.sign === 1), 1, otherExpanded.has('income')));
  const expense = $derived(mergeSmallToOther(rolled.filter((b) => b.sign === -1), -1, otherExpanded.has('expense')));
  const totalIncome = $derived(income.reduce((s, b) => s + b.sumAbsCents, 0));
  const totalExpense = $derived(expense.reduce((s, b) => s + b.sumAbsCents, 0));
  const balance = $derived(totalIncome - totalExpense);

  const W = 720;
  const H = 400;
  const COL_W = 140;
  const COL_GAP = (W - 3 * COL_W) / 2;
  const NODE_GAP = 6;
  const PAD_Y = 28;  // mehr Platz für Header-Labels

  const usableH = H - 2 * PAD_Y;
  const maxSide = $derived(Math.max(totalIncome, totalExpense, 1));
  const pxPerCent = $derived(usableH / maxSide);

  type Node = {
    id: string;
    label: string;
    amount: number;
    pct: number;          // 0..1 Anteil an der Seiten-Summe
    x: number;
    y: number;
    h: number;
    color: string;
    categoryId: number | null;
    canExpand: boolean;
    side: 'income' | 'cashflow' | 'expense';
  };

  const layout = $derived.by(() => {
    const nodes: Node[] = [];

    let cursorY = PAD_Y;
    income.forEach((b, i) => {
      const h = Math.max(2, b.sumAbsCents * pxPerCent);
      const pct = totalIncome > 0 ? b.sumAbsCents / totalIncome : 0;
      const color = categoryColor(b.categoryId, PALETTE_INCOME[i % PALETTE_INCOME.length]);
      const isOther = b.categoryId === -999_999;
      nodes.push({
        id: `inc-${b.key}`,
        label: isOther ? tc.sankeyOther : categoryName(b.categoryId),
        amount: b.sumAbsCents,
        pct,
        x: 0,
        y: cursorY,
        h,
        color,
        categoryId: b.categoryId,
        canExpand: !isOther && b.categoryId !== null && b.categoryId > 0 && hasChildren(b.categoryId),
        side: 'income',
      });
      cursorY += h + NODE_GAP;
    });

    const centralH = Math.max(totalIncome, totalExpense) * pxPerCent;
    const centralY = PAD_Y + (usableH - centralH) / 2;
    const centralX = COL_W + COL_GAP;
    nodes.push({
      id: 'cashflow',
      label: tc.cashflowNode,
      amount: Math.max(totalIncome, totalExpense),
      pct: 1,
      x: centralX,
      y: centralY,
      h: centralH,
      color: '#64748b',
      categoryId: null,
      canExpand: false,
      side: 'cashflow',
    });

    cursorY = PAD_Y;
    expense.forEach((b, i) => {
      const h = Math.max(2, b.sumAbsCents * pxPerCent);
      const pct = totalExpense > 0 ? b.sumAbsCents / totalExpense : 0;
      const color = categoryColor(b.categoryId, PALETTE_EXPENSE[i % PALETTE_EXPENSE.length]);
      const isOther = b.categoryId === -999_999;
      nodes.push({
        id: `exp-${b.key}`,
        label: isOther ? tc.sankeyOther : categoryName(b.categoryId),
        amount: b.sumAbsCents,
        pct,
        x: 2 * (COL_W + COL_GAP),
        y: cursorY,
        h,
        color,
        categoryId: b.categoryId,
        canExpand: !isOther && b.categoryId !== null && b.categoryId > 0 && hasChildren(b.categoryId),
        side: 'expense',
      });
      cursorY += h + NODE_GAP;
    });

    return nodes;
  });

  type Link = { d: string; color: string; nodeId: string };
  /** Y-Range auf dem Center-Node pro Side-Knoten — für Cross-Highlight beim Hover. */
  type CenterSegment = { y: number; h: number; color: string };

  const linkData = $derived.by<{ links: Link[]; segments: Map<string, CenterSegment> }>(() => {
    const links: Link[] = [];
    const segments = new Map<string, CenterSegment>();
    const central = layout.find((n) => n.side === 'cashflow');
    if (!central) return { links, segments };

    let centralCursor = central.y;
    for (const n of layout.filter((nn) => nn.side === 'income')) {
      const sourceX = n.x + COL_W;
      const sourceY1 = n.y;
      const sourceY2 = n.y + n.h;
      const targetX = central.x;
      const targetY1 = centralCursor;
      const targetY2 = centralCursor + n.h;
      links.push({
        d: bezierPath(sourceX, sourceY1, sourceY2, targetX, targetY1, targetY2),
        color: n.color,
        nodeId: n.id,
      });
      segments.set(n.id, { y: targetY1, h: targetY2 - targetY1, color: n.color });
      centralCursor += n.h;
    }

    centralCursor = central.y;
    for (const n of layout.filter((nn) => nn.side === 'expense')) {
      const sourceX = central.x + COL_W;
      const sourceY1 = centralCursor;
      const sourceY2 = centralCursor + n.h;
      const targetX = n.x;
      const targetY1 = n.y;
      const targetY2 = n.y + n.h;
      links.push({
        d: bezierPath(sourceX, sourceY1, sourceY2, targetX, targetY1, targetY2),
        color: n.color,
        nodeId: n.id,
      });
      segments.set(n.id, { y: sourceY1, h: sourceY2 - sourceY1, color: n.color });
      centralCursor += n.h;
    }

    return { links, segments };
  });

  const links = $derived(linkData.links);
  const centerSegments = $derived(linkData.segments);

  const hoverCenterSegment = $derived(
    hoverNodeId !== null && hoverNodeId !== 'cashflow'
      ? centerSegments.get(hoverNodeId) ?? null
      : null,
  );
  const centralNode = $derived(layout.find((n) => n.side === 'cashflow'));

  function bezierPath(
    sx: number, sy1: number, sy2: number,
    tx: number, ty1: number, ty2: number,
  ): string {
    const midX = (sx + tx) / 2;
    return `M ${sx} ${sy1}
            C ${midX} ${sy1}, ${midX} ${ty1}, ${tx} ${ty1}
            L ${tx} ${ty2}
            C ${midX} ${ty2}, ${midX} ${sy2}, ${sx} ${sy2}
            Z`;
  }

  /** Formatiert Cent-Betrag als Pill-Label "1.234 €". */
  function fmtAmountShort(cents: number): string {
    return fmtEur(cents, { hide: settings.hide, decimals: 0 });
  }

  function fmtPct(p: number): string {
    return `${Math.round(p * 100)}%`;
  }

  /** Tooltip-Text für Hover (SVG title-Element). */
  function tooltipFor(n: Node): string {
    if (n.side === 'cashflow') {
      return `${n.label}: ${fmtAmountShort(n.amount)}`;
    }
    return `${n.label}: ${fmtAmountShort(n.amount)} (${fmtPct(n.pct)})`;
  }
</script>

{#if breakdown.length === 0}
  <div class="empty">{tc.sankeyEmpty}</div>
{:else}
  {#if expanded.size > 0}
    <div class="toolbar">
      <button class="reset" type="button" onclick={resetExpansion}>
        ↺ {tc.sankeyResetExpansion ?? 'Drilldown zurücksetzen'}
      </button>
    </div>
  {/if}

  <svg viewBox="0 0 {W} {H}" class="chart" preserveAspectRatio="xMidYMid meet">
    <!-- Spalten-Header mit Summe -->
    <text
      x={COL_W / 2}
      y={14}
      text-anchor="middle"
      class="col-header"
    >
      {tc.income ?? 'Einnahmen'} · {fmtAmountShort(totalIncome)}
    </text>
    <text
      x={2 * (COL_W + COL_GAP) + COL_W / 2}
      y={14}
      text-anchor="middle"
      class="col-header"
    >
      {tc.expenses ?? 'Ausgaben'} · {fmtAmountShort(totalExpense)}
    </text>

    <!-- Bezier-Links -->
    {#each links as l (l.nodeId)}
      <path
        class="link"
        d={l.d}
        fill={l.color}
        opacity={hoverNodeId === null || hoverNodeId === l.nodeId ? 0.4 : 0.08}
      />
    {/each}

    <!-- Nodes -->
    {#each layout as n (n.id)}
      {@const isOtherBucket = n.categoryId === -999_999}
      {@const isClickable = n.canExpand || isOtherBucket || (n.categoryId !== null && n.categoryId > 0)}
      {@const fg = n.side === 'cashflow' ? '#fff' : contrastText(n.color)}
      {@const showCaret = n.canExpand || isOtherBucket}
      <!-- svelte-ignore a11y_click_events_have_key_events -->
      <!-- svelte-ignore a11y_no_static_element_interactions -->
      <!-- svelte-ignore a11y_no_noninteractive_tabindex -->
      <g
        class="node"
        class:expandable={n.canExpand || isOtherBucket}
        class:navigable={!n.canExpand && !isOtherBucket && n.categoryId !== null && n.categoryId > 0}
        class:hovered={hoverNodeId === n.id}
        onmouseenter={() => (hoverNodeId = n.id)}
        onmouseleave={() => (hoverNodeId = null)}
        onclick={() => onNodeClick(n)}
        role={isClickable ? 'button' : undefined}
        tabindex={isClickable ? 0 : undefined}
      >
        <title>{tooltipFor(n)}</title>
        <rect x={n.x} y={n.y} width={COL_W} height={n.h} fill={n.color} rx="2" />
        {#if n.h >= 26}
          <!-- Großer Knoten: Name + Betrag in zwei Zeilen -->
          <text
            x={n.side === 'income' ? n.x + COL_W - 6 : n.x + 6}
            y={n.y + n.h / 2 - 2}
            text-anchor={n.side === 'income' ? 'end' : 'start'}
            class="node-name"
            style:fill={fg}
          >
            {n.label}
            {#if showCaret}<tspan class="caret" style:fill={fg} opacity="0.7">▾</tspan>{/if}
          </text>
          <text
            x={n.side === 'income' ? n.x + COL_W - 6 : n.x + 6}
            y={n.y + n.h / 2 + 11}
            text-anchor={n.side === 'income' ? 'end' : 'start'}
            class="node-amount"
            style:fill={fg}
            opacity="0.85"
          >
            {fmtAmountShort(n.amount)}{#if n.side !== 'cashflow'} · {fmtPct(n.pct)}{/if}
          </text>
        {:else if n.h >= 12}
          <!-- Mittlerer Knoten: nur Name -->
          <text
            x={n.side === 'income' ? n.x + COL_W - 6 : n.x + 6}
            y={n.y + n.h / 2 + 4}
            text-anchor={n.side === 'income' ? 'end' : 'start'}
            class="node-name"
            style:fill={fg}
          >
            {n.label}
          </text>
        {/if}
      </g>
    {/each}

    <!-- Cross-Highlight: gehoverter Side-Knoten → korrespondierender Center-Bereich -->
    {#if hoverCenterSegment && centralNode}
      <rect
        class="center-highlight"
        x={centralNode.x}
        y={hoverCenterSegment.y}
        width={COL_W}
        height={hoverCenterSegment.h}
        fill={hoverCenterSegment.color}
        opacity="0.55"
        rx="2"
        pointer-events="none"
      />
    {/if}
  </svg>

  <div class="foot">
    <span class="lbl">{tc.balance}:</span>
    <span class="num" class:pos={balance > 0} class:neg={balance < 0}>
      {fmtEur(balance, { hide: settings.hide, signed: true, decimals: eurDecimals() })}
    </span>
  </div>
{/if}

<style>
  .empty {
    padding: 32px;
    text-align: center;
    color: var(--text-faint);
    font-size: 13px;
  }
  .toolbar {
    display: flex;
    justify-content: flex-end;
    margin-bottom: 4px;
  }
  .toolbar .reset {
    background: transparent;
    border: 1px solid var(--border);
    border-radius: 4px;
    padding: 3px 10px;
    font: inherit;
    font-size: 11px;
    color: var(--text-muted);
    cursor: pointer;
  }
  .toolbar .reset:hover {
    color: var(--text);
    border-color: var(--accent);
  }
  .chart {
    width: 100%;
    height: 400px;
  }
  .link {
    transition: opacity 0.15s ease;
  }
  .node.expandable,
  .node.navigable {
    cursor: pointer;
  }
  .node.expandable rect:hover,
  .node.navigable rect:hover {
    opacity: 0.88;
  }
  .node.hovered rect {
    stroke: var(--text);
    stroke-width: 1;
  }
  .node-name {
    font-size: 11px;
    font-weight: 500;
    fill: var(--text);
    pointer-events: none;
  }
  .node-name .caret {
    fill: var(--text-muted);
    font-size: 9px;
  }
  .node-amount {
    font-size: 10px;
    fill: var(--text-muted);
    font-variant-numeric: tabular-nums;
    pointer-events: none;
  }
  .col-header {
    font-size: 11px;
    font-weight: 500;
    fill: var(--text-muted);
    text-transform: uppercase;
    letter-spacing: 0.04em;
  }
  .foot {
    display: flex;
    gap: 8px;
    align-items: baseline;
    margin-top: 8px;
    font-size: 14px;
  }
  .foot .lbl { color: var(--text-muted); }
  .foot .num { font-variant-numeric: tabular-nums; font-weight: 500; }
  .foot .pos { color: var(--positive); }
  .foot .neg { color: var(--negative); }
</style>

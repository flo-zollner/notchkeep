<script lang="ts">
  import { tick } from 'svelte';
  import { page } from '$app/state';
  import { goto } from '$app/navigation';
  import { t } from '$lib/settings.svelte';
  import { onboarding, tourNext, tourPrev, finishTour } from '$lib/onboarding/onboarding.svelte';
  import { TOUR_STEPS } from '$lib/onboarding/tour-config';

  const o = $derived(t().onboarding);
  const total = TOUR_STEPS.length;

  type Box = { top: number; left: number; width: number; height: number };
  let target = $state<Box | null>(null);
  const PAD = 8;

  function isVisible(el: Element): boolean {
    const r = el.getBoundingClientRect();
    if (r.width <= 0 || r.height <= 0) return false;
    // Must intersect the viewport (rejects display:none → 0×0 and off-screen
    // elements). offsetParent is unreliable here: position:fixed elements like
    // the FAB and bottom tab bar always report a null offsetParent.
    if (r.bottom <= 0 || r.right <= 0 || r.top >= window.innerHeight || r.left >= window.innerWidth) {
      return false;
    }
    const cs = getComputedStyle(el);
    return cs.display !== 'none' && cs.visibility !== 'hidden' && cs.opacity !== '0';
  }

  function measure(step: (typeof TOUR_STEPS)[number]) {
    for (const sel of step.selectors) {
      // A selector can match several responsive variants of the same control
      // (e.g. the full sidebar, the collapsed rail and the mobile tab bar all
      // carry data-tour="nav"); only one is ever displayed. Pick the first
      // *visible* match rather than the first in DOM order, otherwise a hidden
      // earlier variant would shadow the one actually on screen.
      const el = [...document.querySelectorAll(sel)].find(isVisible);
      if (el) {
        const r = el.getBoundingClientRect();
        target = {
          top: Math.max(0, r.top - PAD),
          left: Math.max(0, r.left - PAD),
          width: r.width + PAD * 2,
          height: r.height + PAD * 2,
        };
        return;
      }
    }
    target = null; // graceful fallback → centred tooltip, no spotlight
  }

  const raf = () => new Promise((res) => requestAnimationFrame(() => res(null)));

  // Navigate + measure whenever the active tour step changes.
  $effect(() => {
    if (!onboarding.tourActive) {
      target = null;
      return;
    }
    const step = TOUR_STEPS[onboarding.tourStep];
    if (!step) return;
    let cancelled = false;
    (async () => {
      if (page.url.pathname !== step.route) {
        await goto(step.route);
      }
      await tick();
      await raf();
      await raf();
      if (!cancelled) measure(step);
    })();
    return () => { cancelled = true; };
  });

  // Keep the spotlight aligned on resize/scroll while the tour is open.
  $effect(() => {
    if (!onboarding.tourActive) return;
    const remeasure = () => {
      const step = TOUR_STEPS[onboarding.tourStep];
      if (step) measure(step);
    };
    window.addEventListener('resize', remeasure);
    window.addEventListener('scroll', remeasure, true);
    return () => {
      window.removeEventListener('resize', remeasure);
      window.removeEventListener('scroll', remeasure, true);
    };
  });

  // Tooltip placement: prefer below the target, then above, then to the side
  // (handles full-height targets like the sidebar), then centred as a fallback.
  // The anchor is clamped so the box never leaves the viewport.
  const TIP_W = 320;
  const TIP_H = 175;
  const GAP = 12;
  const tooltip = $derived.by(() => {
    if (typeof window === 'undefined' || !target) {
      return { centered: true, top: 0, left: 0, transform: '' };
    }
    const vw = window.innerWidth;
    const vh = window.innerHeight;
    const t = target;
    const clampX = (x: number) => Math.min(vw - TIP_W / 2 - 8, Math.max(TIP_W / 2 + 8, x));
    const clampY = (y: number) => Math.min(vh - TIP_H / 2 - 8, Math.max(TIP_H / 2 + 8, y));
    const cx = t.left + t.width / 2;
    const cy = t.top + t.height / 2;

    if (vh - (t.top + t.height) >= TIP_H + GAP) {
      return { centered: false, top: t.top + t.height + GAP, left: clampX(cx), transform: 'translateX(-50%)' };
    }
    if (t.top >= TIP_H + GAP) {
      return { centered: false, top: t.top - GAP, left: clampX(cx), transform: 'translate(-50%, -100%)' };
    }
    if (vw - (t.left + t.width) >= TIP_W + GAP) {
      return { centered: false, top: clampY(cy), left: t.left + t.width + GAP, transform: 'translateY(-50%)' };
    }
    if (t.left >= TIP_W + GAP) {
      return { centered: false, top: clampY(cy), left: t.left - GAP, transform: 'translate(-100%, -50%)' };
    }
    return { centered: true, top: 0, left: 0, transform: '' };
  });

  function onKey(e: KeyboardEvent) {
    if (!onboarding.tourActive) return;
    if (e.key === 'Escape') { e.preventDefault(); finishTour(); }
    else if (e.key === 'ArrowRight' || e.key === 'Enter') { e.preventDefault(); tourNext(total); }
    else if (e.key === 'ArrowLeft') { e.preventDefault(); tourPrev(); }
  }

  const current = $derived(TOUR_STEPS[onboarding.tourStep]);
  const isLast = $derived(onboarding.tourStep === total - 1);
</script>

<svelte:window onkeydown={onKey} />

{#if onboarding.tourActive && current}
  <div class="tour-root" data-testid="tour-overlay">
    {#if target}
      <div
        class="tour-spot"
        style:top="{target.top}px"
        style:left="{target.left}px"
        style:width="{target.width}px"
        style:height="{target.height}px"
      ></div>
    {:else}
      <div class="tour-dim" role="presentation"></div>
    {/if}

    <div
      class="tour-tip"
      class:centered={tooltip.centered}
      style:top={tooltip.centered ? '' : `${tooltip.top}px`}
      style:left={tooltip.centered ? '' : `${tooltip.left}px`}
      style:transform={tooltip.centered ? '' : tooltip.transform}
      role="dialog"
      aria-modal="true"
      aria-label={o.tourSteps[current.id].title}
    >
      <div class="tour-step">{o.stepOf(onboarding.tourStep + 1, total)}</div>
      <h3>{o.tourSteps[current.id].title}</h3>
      <p>{o.tourSteps[current.id].body}</p>
      <div class="tour-actions">
        <button class="tour-skip" onclick={finishTour}>{o.skip}</button>
        <div class="tour-nav">
          {#if onboarding.tourStep > 0}
            <button class="btn" onclick={tourPrev}>{o.back}</button>
          {/if}
          <button class="btn btn-primary" onclick={() => tourNext(total)}>
            {isLast ? o.finish : o.next}
          </button>
        </div>
      </div>
    </div>
  </div>
{/if}

<style>
  .tour-root { position: fixed; inset: 0; z-index: 200; }
  .tour-dim { position: absolute; inset: 0; background: var(--scrim); }
  .tour-spot {
    position: absolute;
    border-radius: var(--r-md);
    /* 9999px spread is an accepted spotlight overlay technique — not a magic spacing value */
    box-shadow: 0 0 0 9999px var(--scrim);
    outline: 2px solid var(--accent);
    outline-offset: 0;
    transition: top 0.25s, left 0.25s, width 0.25s, height 0.25s;
    pointer-events: none;
  }
  @media (prefers-reduced-motion: reduce) {
    .tour-spot { transition: none; }
  }
  .tour-tip {
    position: absolute;
    width: min(320px, calc(100vw - 32px));
    transform: translateX(-50%);
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--r-lg);
    box-shadow: var(--shadow-lg);
    padding: 14px 16px;
  }
  .tour-tip.centered {
    position: absolute;
    top: 50%; left: 50%;
    transform: translate(-50%, -50%);
  }
  .tour-tip h3 { margin: 2px 0 6px; font-size: 15px; font-weight: 650; }
  .tour-tip p { margin: 0 0 12px; font-size: 13px; color: var(--text-muted); line-height: 1.55; }
  .tour-step { font-size: 11px; color: var(--accent); font-weight: 600; letter-spacing: 0.02em; }
  .tour-actions { display: flex; align-items: center; justify-content: space-between; gap: 8px; }
  .tour-nav { display: flex; gap: 8px; }
  .tour-skip {
    font-size: 12.5px; color: var(--text-muted); background: none; border: 0;
    padding: 4px 6px; cursor: pointer; border-radius: var(--r-sm);
  }
  .tour-skip:hover { color: var(--text); }
</style>

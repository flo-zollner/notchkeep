import { readable } from 'svelte/store';

export type Bp = 'phone' | 'tablet' | 'notebook' | 'desktop';

const PHONE = '(max-width: 599px)';
const TABLET = '(min-width: 600px) and (max-width: 1023px)';
const NOTEBOOK = '(min-width: 1024px) and (max-width: 1599px)';
const DESKTOP = '(min-width: 1600px)';

function detect(): Bp {
  if (typeof window === 'undefined') return 'notebook';
  if (window.matchMedia(PHONE).matches) return 'phone';
  if (window.matchMedia(TABLET).matches) return 'tablet';
  if (window.matchMedia(DESKTOP).matches) return 'desktop';
  return 'notebook';
}

/** Reactive current breakpoint class. SSR default: notebook. */
export const bp = readable<Bp>(detect(), (set) => {
  if (typeof window === 'undefined') return;
  const queries = [PHONE, TABLET, NOTEBOOK, DESKTOP].map((q) => window.matchMedia(q));
  const update = () => set(detect());
  queries.forEach((mql) => mql.addEventListener('change', update));
  update();
  return () => queries.forEach((mql) => mql.removeEventListener('change', update));
});

/**
 * Material Design 3 ripple / state-layer touch feedback.
 *
 * Svelte action that spawns an expanding ink circle from the pointer position.
 * Active only on Android (`html[data-platform="android"]`) so desktop keeps its
 * existing flat feedback. The host element gets `position: relative` and
 * `overflow: hidden` via the `md-ripple-host` class (styled in app.css); the ink
 * span animates and removes itself on animation end.
 */
export function ripple(node: HTMLElement) {
  if (typeof document === 'undefined' || document.documentElement.dataset.platform !== 'android') {
    return {};
  }

  node.classList.add('md-ripple-host');

  function spawn(e: PointerEvent) {
    if (e.button !== 0 && e.pointerType === 'mouse') return;
    const rect = node.getBoundingClientRect();
    const size = Math.max(rect.width, rect.height);
    const ink = document.createElement('span');
    ink.className = 'md-ripple-ink';
    ink.style.width = `${size}px`;
    ink.style.height = `${size}px`;
    ink.style.left = `${e.clientX - rect.left - size / 2}px`;
    ink.style.top = `${e.clientY - rect.top - size / 2}px`;
    node.appendChild(ink);
    ink.addEventListener('animationend', () => ink.remove(), { once: true });
  }

  node.addEventListener('pointerdown', spawn);

  return {
    destroy() {
      node.removeEventListener('pointerdown', spawn);
      node.classList.remove('md-ripple-host');
    },
  };
}

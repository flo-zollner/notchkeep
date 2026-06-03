const FOCUSABLE =
  'a[href], button:not([disabled]), input:not([disabled]), select:not([disabled]), textarea:not([disabled]), [tabindex]:not([tabindex="-1"])';

export function focusTrap(node: HTMLElement) {
  const previousFocus = document.activeElement as HTMLElement | null;

  function getFocusable(): HTMLElement[] {
    return Array.from(node.querySelectorAll<HTMLElement>(FOCUSABLE));
  }

  // Focus first focusable element, or the node itself as fallback
  const focusable = getFocusable();
  if (focusable.length > 0) {
    focusable[0].focus();
  } else {
    node.setAttribute('tabindex', '-1');
    node.focus();
  }

  function onKeyDown(e: KeyboardEvent) {
    if (e.key !== 'Tab') return;

    const items = getFocusable();
    if (items.length === 0) {
      e.preventDefault();
      return;
    }

    const first = items[0];
    const last = items[items.length - 1];

    if (e.shiftKey) {
      if (document.activeElement === first) {
        e.preventDefault();
        last.focus();
      }
    } else {
      if (document.activeElement === last) {
        e.preventDefault();
        first.focus();
      }
    }
  }

  node.addEventListener('keydown', onKeyDown);

  return {
    destroy() {
      node.removeEventListener('keydown', onKeyDown);
      if (previousFocus && document.contains(previousFocus)) {
        previousFocus.focus();
      }
    }
  };
}

<script lang="ts">
  import { getCurrentWindow } from '@tauri-apps/api/window';

  const win = getCurrentWindow();

  // ResizeDirection as defined in @tauri-apps/api/window.d.ts — PascalCase.
  type Dir = 'North' | 'NorthEast' | 'East' | 'SouthEast' | 'South' | 'SouthWest' | 'West' | 'NorthWest';

  function start(dir: Dir, e: MouseEvent) {
    if (e.button !== 0) return; // only left click
    void win.startResizeDragging(dir);
    e.preventDefault();
  }
</script>

<!--
  Eight invisible resize handles around the window edges.
  4 corners are 8x8px, 4 sides are 4px thick.
  Sits on top of everything (z-index: 1001 > titlebar 1000) so corners win.
  Pure mouse-drag targets — keyboard users resize via WM shortcuts (Alt+F8 in KDE),
  so a11y-static / no-keyboard-listener warnings are intentional here.
-->
<!-- svelte-ignore a11y_no_static_element_interactions -->
<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
<div class="edge n" onmousedown={(e) => start('North', e)}></div>
<!-- svelte-ignore a11y_no_static_element_interactions -->
<div class="edge e" onmousedown={(e) => start('East', e)}></div>
<!-- svelte-ignore a11y_no_static_element_interactions -->
<div class="edge s" onmousedown={(e) => start('South', e)}></div>
<!-- svelte-ignore a11y_no_static_element_interactions -->
<div class="edge w" onmousedown={(e) => start('West', e)}></div>
<!-- svelte-ignore a11y_no_static_element_interactions -->
<div class="corner ne" onmousedown={(e) => start('NorthEast', e)}></div>
<!-- svelte-ignore a11y_no_static_element_interactions -->
<div class="corner se" onmousedown={(e) => start('SouthEast', e)}></div>
<!-- svelte-ignore a11y_no_static_element_interactions -->
<div class="corner sw" onmousedown={(e) => start('SouthWest', e)}></div>
<!-- svelte-ignore a11y_no_static_element_interactions -->
<div class="corner nw" onmousedown={(e) => start('NorthWest', e)}></div>

<style>
  .edge,
  .corner {
    position: fixed;
    z-index: 1001;
    background: transparent;
  }
  .edge.n {
    top: 0;
    left: 8px;
    right: 8px;
    height: 4px;
    cursor: ns-resize;
  }
  .edge.s {
    bottom: 0;
    left: 8px;
    right: 8px;
    height: 4px;
    cursor: ns-resize;
  }
  .edge.e {
    top: 8px;
    bottom: 8px;
    right: 0;
    width: 4px;
    cursor: ew-resize;
  }
  .edge.w {
    top: 8px;
    bottom: 8px;
    left: 0;
    width: 4px;
    cursor: ew-resize;
  }
  .corner {
    width: 8px;
    height: 8px;
  }
  .corner.nw {
    top: 0;
    left: 0;
    cursor: nwse-resize;
  }
  .corner.ne {
    top: 0;
    right: 0;
    cursor: nesw-resize;
  }
  .corner.sw {
    bottom: 0;
    left: 0;
    cursor: nesw-resize;
  }
  .corner.se {
    bottom: 0;
    right: 0;
    cursor: nwse-resize;
  }
  /* Hide on mobile — Tauri Android handles window sizing via OS */
  @media (max-width: 600px) {
    .edge,
    .corner {
      display: none;
    }
  }
</style>

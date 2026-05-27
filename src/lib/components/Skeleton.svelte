<script lang="ts">
  interface Props {
    /** Höhe in px oder z.B. "12px" / "2em". Default 12px. */
    height?: string | number;
    /** Breite. Default 100%. */
    width?: string | number;
    /** Border-Radius. Default 4px. */
    radius?: string | number;
    /** Margin oben (px). */
    marginTop?: string | number;
  }
  let { height = 12, width = '100%', radius = 4, marginTop = 0 }: Props = $props();

  function px(v: string | number): string {
    return typeof v === 'number' ? `${v}px` : v;
  }
</script>

<div
  class="skeleton"
  style:height={px(height)}
  style:width={px(width)}
  style:border-radius={px(radius)}
  style:margin-top={px(marginTop)}
  aria-busy="true"
  aria-hidden="true"
></div>

<style>
  .skeleton {
    background: linear-gradient(
      90deg,
      var(--surface-2) 0%,
      color-mix(in srgb, var(--surface-2) 60%, var(--border)) 50%,
      var(--surface-2) 100%
    );
    background-size: 200% 100%;
    animation: shimmer 1.4s linear infinite;
  }
  @keyframes shimmer {
    0% { background-position: -100% 0; }
    100% { background-position: 100% 0; }
  }
</style>

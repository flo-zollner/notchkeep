<script lang="ts">
  interface Props {
    values: number[];
    color?: string;
    height?: number;
    width?: number;
  }
  let { values, color = 'var(--accent)', height = 32, width = 100 }: Props = $props();

  const min = $derived(Math.min(...values));
  const max = $derived(Math.max(...values));
  const range = $derived(max - min || 1);
  const path = $derived(
    'M ' +
      values
        .map((v, i) => `${(i / (values.length - 1)) * width},${height - ((v - min) / range) * height}`)
        .join(' L ')
  );
</script>

<svg {width} {height} viewBox="0 0 {width} {height}">
  <path d={path} fill="none" stroke={color} stroke-width="1.5" stroke-linecap="round" />
</svg>

<style>
  svg {
    display: block;
  }
</style>

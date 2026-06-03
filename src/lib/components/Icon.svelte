<script lang="ts">
  type IconName =
    | 'overview' | 'networth' | 'tx' | 'budget' | 'cashflow' | 'reports'
    | 'accounts' | 'settings'
    | 'plus' | 'search' | 'filter'
    | 'arrow-down' | 'arrow-up' | 'arrow-right'
    | 'chevron-right' | 'chevron-down' | 'chevron-up'
    | 'check' | 'x' | 'sun' | 'moon' | 'monitor'
    | 'cart' | 'home' | 'car' | 'utensils' | 'film' | 'bag' | 'bolt'
    | 'heart' | 'repeat' | 'plane' | 'shield' | 'briefcase' | 'wallet'
    | 'lock' | 'globe' | 'download' | 'refresh' | 'dots'
    | 'eye' | 'eye-off' | 'pencil' | 'tag' | 'target' | 'trending' | 'goal'
    | 'copy' | 'warning' | 'alert-circle';

  interface Props {
    name: IconName | string;
    size?: number;
    stroke?: number;
    /** Forwarded to the <svg> (e.g. aria-hidden, aria-label, class). */
    [key: string]: unknown;
  }

  let { name, size = 16, stroke = 1.5, ...rest }: Props = $props();
</script>

<svg
  {...rest}
  width={size}
  height={size}
  viewBox="0 0 24 24"
  fill="none"
  stroke="currentColor"
  stroke-width={stroke}
  stroke-linecap="round"
  stroke-linejoin="round"
>
  {#if name === 'overview'}
    <rect x="3" y="3" width="7" height="9" rx="1.5" />
    <rect x="14" y="3" width="7" height="5" rx="1.5" />
    <rect x="14" y="12" width="7" height="9" rx="1.5" />
    <rect x="3" y="16" width="7" height="5" rx="1.5" />
  {:else if name === 'networth'}
    <path d="M3 17l5-5 4 4 8-9" />
    <path d="M14 7h6v6" />
  {:else if name === 'tx'}
    <path d="M7 17h10M7 17l-3-3M7 17l-3 3" />
    <path d="M17 7H7m10 0l3-3M17 7l3 3" />
  {:else if name === 'budget'}
    <circle cx="12" cy="12" r="9" />
    <path d="M12 7v5l3 2" />
  {:else if name === 'cashflow'}
    <path d="M3 12c3-4 6-4 9 0s6 4 9 0" />
  {:else if name === 'reports'}
    <path d="M4 19V5" />
    <path d="M4 19h16" />
    <rect x="7" y="12" width="3" height="7" />
    <rect x="12" y="8" width="3" height="11" />
    <rect x="17" y="14" width="3" height="5" />
  {:else if name === 'accounts'}
    <rect x="3" y="6" width="18" height="13" rx="2" />
    <path d="M3 10h18" />
    <path d="M7 15h3" />
  {:else if name === 'settings'}
    <circle cx="12" cy="12" r="3" />
    <path
      d="M19.4 15a1.7 1.7 0 0 0 .3 1.8l.1.1a2 2 0 1 1-2.8 2.8l-.1-.1a1.7 1.7 0 0 0-1.8-.3 1.7 1.7 0 0 0-1 1.5V21a2 2 0 1 1-4 0v-.1a1.7 1.7 0 0 0-1-1.5 1.7 1.7 0 0 0-1.8.3l-.1.1a2 2 0 1 1-2.8-2.8l.1-.1A1.7 1.7 0 0 0 4.6 15a1.7 1.7 0 0 0-1.5-1H3a2 2 0 1 1 0-4h.1a1.7 1.7 0 0 0 1.5-1 1.7 1.7 0 0 0-.3-1.8l-.1-.1a2 2 0 1 1 2.8-2.8l.1.1a1.7 1.7 0 0 0 1.8.3H9a1.7 1.7 0 0 0 1-1.5V3a2 2 0 1 1 4 0v.1a1.7 1.7 0 0 0 1 1.5 1.7 1.7 0 0 0 1.8-.3l.1-.1a2 2 0 1 1 2.8 2.8l-.1.1a1.7 1.7 0 0 0-.3 1.8V9a1.7 1.7 0 0 0 1.5 1H21a2 2 0 1 1 0 4h-.1a1.7 1.7 0 0 0-1.5 1z"
    />
  {:else if name === 'plus'}
    <path d="M12 5v14M5 12h14" />
  {:else if name === 'search'}
    <circle cx="11" cy="11" r="7" />
    <path d="M20 20l-3.5-3.5" />
  {:else if name === 'filter'}
    <path d="M3 4h18l-7 9v6l-4 1v-7L3 4z" />
  {:else if name === 'arrow-down'}
    <path d="M12 5v14M5 12l7 7 7-7" />
  {:else if name === 'arrow-up'}
    <path d="M12 19V5M5 12l7-7 7 7" />
  {:else if name === 'arrow-right'}
    <path d="M5 12h14M13 5l7 7-7 7" />
  {:else if name === 'chevron-right'}
    <path d="M9 6l6 6-6 6" />
  {:else if name === 'chevron-down'}
    <path d="M6 9l6 6 6-6" />
  {:else if name === 'chevron-up'}
    <path d="M6 15l6-6 6 6" />
  {:else if name === 'check'}
    <path d="M5 12l5 5 9-11" />
  {:else if name === 'x'}
    <path d="M6 6l12 12M18 6L6 18" />
  {:else if name === 'sun'}
    <circle cx="12" cy="12" r="4" />
    <path
      d="M12 2v2M12 20v2M4.93 4.93l1.41 1.41M17.66 17.66l1.41 1.41M2 12h2M20 12h2M4.93 19.07l1.41-1.41M17.66 6.34l1.41-1.41"
    />
  {:else if name === 'moon'}
    <path d="M21 12.8A9 9 0 1 1 11.2 3a7 7 0 0 0 9.8 9.8z" />
  {:else if name === 'monitor'}
    <rect x="2" y="3" width="20" height="14" rx="2" />
    <line x1="8" y1="21" x2="16" y2="21" />
    <line x1="12" y1="17" x2="12" y2="21" />
  {:else if name === 'cart'}
    <path d="M3 4h2l2.4 12.5a2 2 0 0 0 2 1.5h8.2a2 2 0 0 0 2-1.5L21 8H6" />
    <circle cx="9" cy="21" r="1" />
    <circle cx="18" cy="21" r="1" />
  {:else if name === 'home'}
    <path d="M3 12l9-9 9 9" />
    <path d="M5 10v10h14V10" />
  {:else if name === 'car'}
    <path d="M5 17h14M6 13l1.5-5h9L18 13M5 17v3M19 17v3" />
    <circle cx="8" cy="15" r="1" />
    <circle cx="16" cy="15" r="1" />
  {:else if name === 'utensils'}
    <path d="M5 3v8a2 2 0 0 0 2 2h0a2 2 0 0 0 2-2V3M7 13v8M17 3c-1.5 0-3 2-3 5s1.5 4 3 4v9" />
  {:else if name === 'film'}
    <rect x="3" y="4" width="18" height="16" rx="2" />
    <path d="M7 4v16M17 4v16M3 9h18M3 15h18" />
  {:else if name === 'bag'}
    <path d="M6 8h12l-1 13H7L6 8z" />
    <path d="M9 8V6a3 3 0 0 1 6 0v2" />
  {:else if name === 'bolt'}
    <path d="M13 2L4 14h7l-1 8 9-12h-7l1-8z" />
  {:else if name === 'heart'}
    <path d="M12 21s-7-4.5-9.5-9.5C.5 7.5 4 4 7 4c2 0 4 1 5 3 1-2 3-3 5-3 3 0 6.5 3.5 4.5 7.5C19 16.5 12 21 12 21z" />
  {:else if name === 'repeat'}
    <path d="M17 1l4 4-4 4" />
    <path d="M3 11V9a4 4 0 0 1 4-4h14M7 23l-4-4 4-4" />
    <path d="M21 13v2a4 4 0 0 1-4 4H3" />
  {:else if name === 'plane'}
    <path d="M10 4l9 6-9 6v-4l-6-2 6-2V4z" />
  {:else if name === 'shield'}
    <path d="M12 3l8 3v6c0 4-3 8-8 9-5-1-8-5-8-9V6l8-3z" />
  {:else if name === 'briefcase'}
    <rect x="3" y="7" width="18" height="13" rx="2" />
    <path d="M9 7V5a2 2 0 0 1 2-2h2a2 2 0 0 1 2 2v2" />
  {:else if name === 'wallet'}
    <rect x="3" y="6" width="18" height="14" rx="2" />
    <path d="M16 13h3" />
  {:else if name === 'lock'}
    <rect x="4" y="11" width="16" height="10" rx="2" />
    <path d="M8 11V7a4 4 0 0 1 8 0v4" />
  {:else if name === 'globe'}
    <circle cx="12" cy="12" r="9" />
    <path d="M3 12h18M12 3c3 3 3 15 0 18M12 3c-3 3-3 15 0 18" />
  {:else if name === 'download'}
    <path d="M12 4v12M6 12l6 6 6-6M4 20h16" />
  {:else if name === 'refresh'}
    <path d="M20 12a8 8 0 1 1-3-6.2L20 8M20 3v5h-5" />
  {:else if name === 'dots'}
    <circle cx="5" cy="12" r="1.2" />
    <circle cx="12" cy="12" r="1.2" />
    <circle cx="19" cy="12" r="1.2" />
  {:else if name === 'eye'}
    <path d="M2 12s4-7 10-7 10 7 10 7-4 7-10 7-10-7-10-7z" />
    <circle cx="12" cy="12" r="3" />
  {:else if name === 'eye-off'}
    <path
      d="M3 3l18 18M9 5.5C10 5.2 11 5 12 5c6 0 10 7 10 7-1 1.5-2 3-3.5 4M6.6 6.6C4 8.4 2 12 2 12s4 7 10 7c2 0 3.7-.6 5-1.5"
    />
    <path d="M9.9 9.9a3 3 0 0 0 4.2 4.2" />
  {:else if name === 'pencil'}
    <path d="M16 3l5 5-12 12H4v-5L16 3z" />
  {:else if name === 'tag'}
    <path d="M3 12V3h9l9 9-9 9-9-9z" />
    <circle cx="7.5" cy="7.5" r="1" />
  {:else if name === 'target'}
    <circle cx="12" cy="12" r="9" />
    <circle cx="12" cy="12" r="5" />
    <circle cx="12" cy="12" r="1" />
  {:else if name === 'trending'}
    <path d="M3 17l6-6 4 4 8-8" />
    <path d="M14 7h6v6" />
  {:else if name === 'goal'}
    <path d="M5 21V4" />
    <path d="M5 4h11l-2 3 2 3H5" />
  {:else if name === 'copy'}
    <rect x="9" y="9" width="11" height="11" rx="2" />
    <path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1" />
  {:else if name === 'warning'}
    <path d="M12 3L2 20h20L12 3z" />
    <path d="M12 9v5" />
    <path d="M12 18h.01" />
  {:else if name === 'alert-circle'}
    <circle cx="12" cy="12" r="9" />
    <path d="M12 8v4" />
    <path d="M12 16h.01" />
  {:else}
    <circle cx="12" cy="12" r="3" />
  {/if}
</svg>

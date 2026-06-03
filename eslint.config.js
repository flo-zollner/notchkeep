import js from '@eslint/js';
import ts from 'typescript-eslint';
import svelte from 'eslint-plugin-svelte';
import svelteParser from 'svelte-eslint-parser';

export default [
  js.configs.recommended,
  ...ts.configs.recommended,
  ...svelte.configs['flat/recommended'],
  {
    // Claude hooks/workflows and scripts are not app source — exclude entirely.
    // Vite config and playwright config are also excluded.
    ignores: [
      'build/',
      '.svelte-kit/',
      'static/',
      'src-tauri/',
      'node_modules/',
      'sbom*.json',
      'e2e/',
      'playwright.config.ts',
      'vite.config.js',
      '.claude/',
      'scripts/',
    ],
  },
  {
    // .svelte files: use svelte-eslint-parser, TypeScript as inner parser.
    // Also declare browser globals so window/document/setTimeout etc. are known.
    files: ['**/*.svelte', '*.svelte'],
    languageOptions: {
      parser: svelteParser,
      parserOptions: {
        parser: ts.parser,
      },
      globals: {
        // Browser globals — Svelte files run in a WebView
        window: 'readonly',
        document: 'readonly',
        navigator: 'readonly',
        location: 'readonly',
        history: 'readonly',
        localStorage: 'readonly',
        sessionStorage: 'readonly',
        setTimeout: 'readonly',
        clearTimeout: 'readonly',
        setInterval: 'readonly',
        clearInterval: 'readonly',
        requestAnimationFrame: 'readonly',
        cancelAnimationFrame: 'readonly',
        fetch: 'readonly',
        URL: 'readonly',
        URLSearchParams: 'readonly',
        File: 'readonly',
        FileReader: 'readonly',
        Blob: 'readonly',
        FormData: 'readonly',
        Event: 'readonly',
        CustomEvent: 'readonly',
        MouseEvent: 'readonly',
        KeyboardEvent: 'readonly',
        PointerEvent: 'readonly',
        TouchEvent: 'readonly',
        InputEvent: 'readonly',
        HTMLElement: 'readonly',
        HTMLDivElement: 'readonly',
        HTMLSpanElement: 'readonly',
        HTMLInputElement: 'readonly',
        HTMLSelectElement: 'readonly',
        HTMLTextAreaElement: 'readonly',
        HTMLButtonElement: 'readonly',
        HTMLAnchorElement: 'readonly',
        HTMLFormElement: 'readonly',
        SVGSVGElement: 'readonly',
        SVGElement: 'readonly',
        Element: 'readonly',
        getComputedStyle: 'readonly',
        Node: 'readonly',
        NodeList: 'readonly',
        MutationObserver: 'readonly',
        ResizeObserver: 'readonly',
        IntersectionObserver: 'readonly',
        MediaQueryList: 'readonly',
        matchMedia: 'readonly',
        confirm: 'readonly',
        alert: 'readonly',
        console: 'readonly',
        crypto: 'readonly',
        performance: 'readonly',
        queueMicrotask: 'readonly',
        structuredClone: 'readonly',
        Map: 'readonly',
        Set: 'readonly',
        Promise: 'readonly',
        WeakMap: 'readonly',
        WeakSet: 'readonly',
      },
    },
  },
  {
    // .svelte.ts runes files: use TypeScript parser directly, also browser context
    files: ['**/*.svelte.ts', '*.svelte.ts'],
    languageOptions: {
      parser: ts.parser,
      globals: {
        window: 'readonly',
        document: 'readonly',
        localStorage: 'readonly',
        setTimeout: 'readonly',
        clearTimeout: 'readonly',
        console: 'readonly',
      },
    },
  },
  {
    rules: {
      // --- Bugs/data-hygiene we WANT (not style) ---
      'no-console': ['warn', { allow: ['warn', 'error'] }],
      'no-debugger': 'error',
      '@typescript-eslint/no-unused-vars': ['warn', { argsIgnorePattern: '^_', varsIgnorePattern: '^_' }],

      // --- Style/churn OFF — no formatting ---
      '@typescript-eslint/no-explicit-any': 'off',
      'svelte/no-at-html-tags': 'off',
      'prefer-const': 'off',

      // --- Svelte-5 runes false-positives ---
      // In $effect(() => { dep; doWork(); }) the bare `dep;` is an intentional
      // reactive-dependency touch, not a real unused expression.
      '@typescript-eslint/no-unused-expressions': 'off',

      // --- Opinionated Svelte rules — warn, not error (require migration effort) ---
      // svelte/prefer-svelte-reactivity: replace native Map/Set/Date with Svelte
      // wrappers — valid advice, but a large migration; warn only for now.
      'svelte/prefer-svelte-reactivity': 'warn',

      // svelte/no-navigation-without-resolve: goto() without resolve() can cause
      // issues with page transitions, but many call sites are intentional fire-and-
      // forget navigations.  Downgrade to warn so it doesn't block CI until the
      // team decides on a migration strategy.
      'svelte/no-navigation-without-resolve': 'warn',

      // no-empty: empty catch blocks are sometimes intentional (best-effort paths).
      // Warn instead of error so we notice them without blocking CI.
      'no-empty': ['warn', { allowEmptyCatch: true }],

      // no-useless-assignment: false-positives in try/catch patterns where a
      // variable is initialised before try and re-assigned inside (common pattern).
      'no-useless-assignment': 'off',

      // svelte/require-each-key: missing keys on {#each} are a real correctness
      // concern, but the affected blocks are <option> lists inside <select> where
      // Svelte's DOM diffing is order-stable. Downgrade to warn for now.
      'svelte/require-each-key': 'warn',

      // svelte/no-unused-svelte-ignore: stale <!-- svelte-ignore --> comments left
      // over from Svelte 4 → 5 migration. Not a runtime issue; downgrade to warn.
      'svelte/no-unused-svelte-ignore': 'warn',

      // svelte/no-useless-children-snippet: style/cleanup concern, not a bug.
      'svelte/no-useless-children-snippet': 'warn',

      // svelte/prefer-writable-derived: refactoring suggestion ($state+$effect →
      // writable $derived). Valid advice but not a bug; downgrade to warn.
      'svelte/prefer-writable-derived': 'warn',
    },
  },
];

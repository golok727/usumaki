---
title: Theme with Variables
description: Define theme tokens, reference them from props, swap themes at runtime, and follow the system light/dark preference.
---

Define your colors and sizes once, reference them anywhere in your UI as `$name`, and swap the whole theme with a single call.

## Define Theme Tokens

Set up a map of tokens and pass it to `defineVars`. The returned `theme` object exposes each key as a `$name` reference for use in JSX.

```ts
// theme.ts
import { defineVars } from 'uzumaki';

const dark = {
  bg: '#0a0a0a',
  text: '#e4e4e7',
  accent: '#e2a52e',
};

const { vars: darkVars, theme } = defineVars(dark);

export { theme, darkVars };
```

Any string prop value that starts with `$` is resolved against the active vars at runtime. `defineVars` builds `theme` so that `theme.bg` is just the string `"$bg"` — you can write it by hand or let the helper do it for you.

## Apply on Window Create

Pass the initial map to `vars` on the window options.

```ts
import { Window } from 'uzumaki';
import { theme, darkVars } from './theme';

const window = new Window('main', {
  width: 800,
  height: 600,
  vars: darkVars,
  rootStyles: { bg: theme.bg, color: theme.text },
});
```

## Reference Tokens in Props

Drop `theme.token` into any prop — or write the `$name` string directly. Both work the same.

```tsx
<view bg={theme.bg} color="$text">
  <button bg={theme.accent} hover:bg={theme.accent}>
    <text color={theme.bg}>Click</text>
  </button>
</view>
```

If a token isn't defined yet, the prop just doesn't apply — define it later and everything bound to it picks up the value.

## Switch at Runtime

Call `setVars` with a new map. Every element that uses one of those tokens updates immediately. No re-render, no state loss.

```ts
window.setVars(lightVars);
```

To change a single token, use `setVar`. Pass `null` to remove a token entirely.

```ts
window.setVar('accent', '#22c55e');
window.setVar('accent', null);
```

## Light, Dark, or System

A window has two related theme values:

- `window.theme` is the preference you set: `'light'`, `'dark'`, or `'system'`. Defaults to `'system'`.
- `window.resolvedTheme` is read-only and always `'light'` or `'dark'` — the effective theme after resolving `'system'` against the OS.

```ts
window.theme = 'system'; // follow the OS
window.resolvedTheme; // 'dark' on a dark OS

window.theme = 'light'; // force light
window.resolvedTheme; // 'light'
```

A `'system'` window reads the OS preference at startup and tracks it as the user switches light/dark.

## React to Theme Changes

Listen for `themechange`. It fires whenever the resolved theme changes, whether from a manual `window.theme = ...` or from the OS switching while the preference is `'system'`. A forced theme that the OS change does not affect produces no event, so there is nothing to debounce.

```ts
window.on('themechange', (e) => {
  e.theme; // resolved: 'light' | 'dark'
  e.preference; // 'light' | 'dark' | 'system'
  window.setVars(e.theme === 'dark' ? darkVars : lightVars);
});
```

That is the whole contract: uzumaki emits the event and exposes the resolved value. Turning it into application state is up to you.

## Build a Theme Hook

The event pairs directly with React's `useSyncExternalStore`. uzumaki does not ship a theme hook, but it is a few lines on top of the API above:

```ts
import { useSyncExternalStore } from 'react';
import type { Window } from 'uzumaki';

export function useResolvedTheme(window: Window) {
  return useSyncExternalStore(
    (onChange) => {
      window.on('themechange', onChange);
      return () => window.off('themechange', onChange);
    },
    () => window.resolvedTheme,
  );
}
```

Now a component re-renders whenever the theme flips, from the OS or a settings toggle:

```tsx
function ThemeBadge({ window }: { window: Window }) {
  const theme = useResolvedTheme(window);
  return <text>Theme: {theme}</text>;
}
```

## Across the App

Each window owns its own vars and its own preference. Apply tokens once per window by reacting to `themechange` (as above), and components keep their state while the UI updates in place. To drive several windows from one setting, set `window.theme` on each and let each window's listener apply its own vars.

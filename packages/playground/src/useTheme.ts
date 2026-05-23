import { useCallback, useSyncExternalStore } from 'react';
import type { Window, WindowTheme } from 'uzumaki';
import { themes, type ThemeName } from './theme';

/**
 * Wire a window's theme tokens to its resolved theme. Applies the initial
 * vars and re-applies them whenever the resolved theme changes (manual set or
 * OS switch). Call once per window after creation.
 */
export function installThemeVars(window: Window): void {
  window.setVars(themes[window.resolvedTheme]);
  window.on('themechange', (e) => {
    window.setVars(themes[e.theme]);
  });
}

export function useTheme(window: Window): ThemeName {
  const subscribe = useCallback(
    (onChange: () => void) => {
      window.on('themechange', onChange);
      return () => window.off('themechange', onChange);
    },
    [window],
  );
  return useSyncExternalStore(subscribe, () => window.resolvedTheme);
}

export function setTheme(window: Window, preference: WindowTheme): void {
  window.theme = preference;
}

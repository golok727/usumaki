import { createHighlighter } from 'shiki';
import type { ThemeName } from '../theme';

const SHIKI_THEME = {
  dark: 'ayu-dark',
  light: 'github-light',
} as const satisfies Record<ThemeName, string>;

const highlighter = await createHighlighter({
  langs: ['tsx'],
  themes: Object.values(SHIKI_THEME),
});

export function highlightTsx(code: string, theme: ThemeName) {
  return highlighter.codeToTokensBase(code, {
    lang: 'tsx',
    theme: SHIKI_THEME[theme],
  });
}

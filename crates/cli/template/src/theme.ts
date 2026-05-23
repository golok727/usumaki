import { defineVars } from 'uzumaki';

const dark = {
  bg: '#0a0a0a',
  text: '#e4e4e7',
  textMuted: '#a1a1aa',
  accent: '#e2a52e',
  accentHi: '#f0c04a',
  accentDim: '#7a5518',
  accentDark: '#3d2a0c',
};

const light: typeof dark = {
  bg: '#fafafa',
  text: '#18181b',
  textMuted: '#52525b',
  accent: '#f59e0b',
  accentHi: '#92400e',
  accentDim: '#fed7aa',
  accentDark: '#fef3c7',
};

const { vars: darkVars, theme } = defineVars(dark);
const { vars: lightVars } = defineVars(light);

export const C = theme;
export const themes = { dark: darkVars, light: lightVars };
export type ThemeName = keyof typeof themes;

export type NodeId = number;

export interface WindowPosition {
  x: number;
  y: number;
}

export interface WindowSize {
  width: number;
  height: number;
}

export type WindowTheme = 'light' | 'dark' | 'system';
export type ResolvedTheme = 'light' | 'dark';
export type WindowLevel = 'normal' | 'alwaysOnTop' | 'alwaysOnBottom';

export interface WindowOptions {
  width?: number;
  height?: number;
  title?: string;
  visible?: boolean;
  resizable?: boolean;
  decorations?: boolean;
  transparent?: boolean;
  maximized?: boolean;
  minimized?: boolean;
  fullscreen?: boolean;
  windowLevel?: WindowLevel;
  minWidth?: number;
  minHeight?: number;
  maxWidth?: number;
  maxHeight?: number;
  position?: WindowPosition;
  theme?: WindowTheme;
  active?: boolean;
  contentProtected?: boolean;
  closable?: boolean;
  minimizable?: boolean;
  maximizable?: boolean;
  rootStyles?: Record<string, string | number | boolean>;
  vars?: Record<string, string>;
}

export interface ListenerEntry {
  name: string;
  handler: Function;
  capture: boolean;
}

export interface AppPath {
  readonly resourceDir: string;
  readonly identifier: string;
  resource(rel: string): string;
  cacheDir(): string | null;
  dataDir(): string | null;
  configDir(): string | null;
  tempDir(): string;
  exeDir(): string | null;
  homeDir(): string | null;
}

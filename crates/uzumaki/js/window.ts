import core, { type CoreWindow } from 'ext:uzumaki/core.ts';
import type {
  WindowOptions,
  WindowLevel,
  WindowPosition,
  WindowSize,
  WindowTheme,
  ResolvedTheme,
} from 'ext:uzumaki/types.ts';
import { UzTextNode } from 'ext:uzumaki/node.ts';
import { Element } from 'ext:uzumaki/elements/element.ts';
import { UzElement } from 'ext:uzumaki/elements/base.ts';
import { UzRootElement } from 'ext:uzumaki/elements/root.ts';
import { UzViewElement } from 'ext:uzumaki/elements/view.ts';
import { UzTextElement } from 'ext:uzumaki/elements/text.ts';
import { UzButtonElement } from 'ext:uzumaki/elements/button.ts';
import { UzImageElement } from 'ext:uzumaki/elements/image.ts';
import { UzInputElement } from 'ext:uzumaki/elements/input.ts';
import { UzCheckboxElement } from 'ext:uzumaki/elements/checkbox.ts';
import {
  UzEventTarget,
  type ListenerOptions,
} from 'ext:uzumaki/event-target.ts';
import {
  buildLifecycleEvent,
  type WindowEventMap,
  type WindowEventName,
  type WindowEventHandler,
} from 'ext:uzumaki/events.ts';
import { clearWindowNodes, nodeCount } from 'ext:uzumaki/registry.ts';

const windowsByLabel = new Map<string, Window>();
const windowsById = new Map<number, Window>();
const DEFAULT_WINDOW_WIDTH = 800;
const DEFAULT_WINDOW_HEIGHT = 600;
const DEFAULT_WINDOW_TITLE = 'uzumaki';
const DEFAULT_WINDOW_LEVEL: WindowLevel = 'normal';
const DEFAULT_THEME_PREFERENCE: WindowTheme = 'system';

type AnimationFrameCallback = (timestamp: number) => void;

type ElementConstructor<T extends Element<any> = Element<any>> = new (
  window: Window,
) => T;

const ELEMENT_CONSTRUCTORS = {
  view: UzViewElement,
  text: UzTextElement,
  button: UzButtonElement,
  input: UzInputElement,
  checkbox: UzCheckboxElement,
  image: UzImageElement,
} satisfies Record<string, ElementConstructor>;

export type ElementTagName = keyof typeof ELEMENT_CONSTRUCTORS;
export type ElementForTag<T extends ElementTagName> = InstanceType<
  (typeof ELEMENT_CONSTRUCTORS)[T]
>;

export interface WindowAttributes {
  width: number;
  height: number;
  title: string;
  rootStyles: Record<string, unknown>;
}

export class Window {
  private _id: number;
  private _native: CoreWindow;
  private _label: string;
  private _remBase: number = 16;
  private _disposed: boolean = false;
  private _disposables: (() => void)[] = [];
  private _root: UzRootElement | null = null;
  private _nextAnimationFrameHandle: number = 1;
  private _animationFrameCallbacks = new Map<number, AnimationFrameCallback>();
  private _animationFramePendingNotified: boolean = false;
  private _themePreference: WindowTheme = DEFAULT_THEME_PREFERENCE;
  private _systemTheme: ResolvedTheme = 'light';
  /** @internal Used by the dispatcher and runtime glue. */
  readonly _emitter: UzEventTarget<WindowEventMap> = new UzEventTarget();

  constructor(label: string, attributes: WindowOptions = {}) {
    const existing = windowsByLabel.get(label);
    if (existing) {
      throw new Error(`Window with label ${label} already exists`);
    }

    const { rootStyles, vars, ...createOptions } = attributes;

    this._label = label;
    this._native = core.createWindow(createOptions);
    this._id = this._native.id;
    this._themePreference = createOptions.theme ?? DEFAULT_THEME_PREFERENCE;
    const nativeTheme = this._native.theme;
    if (nativeTheme === 'dark' || nativeTheme === 'light') {
      this._systemTheme = nativeTheme;
    }

    if (vars) {
      for (const [key, value] of Object.entries(vars)) {
        if (value != null) this._native.setVar(key, value);
      }
    }

    if (rootStyles) {
      const root = this.root;
      for (const [key, value] of Object.entries(rootStyles)) {
        if (value != null) root.setAttribute(key, value as never);
      }
    }

    windowsByLabel.set(label, this);
    windowsById.set(this._id, this);
  }

  close() {
    this._clearAnimationFrameCallbacks();
    this._emitter._clear();
    windowsByLabel.delete(this._label);
    windowsById.delete(this._id);
    this._native.close();
  }

  addDisposable(cb: () => void): void {
    this._disposables.push(cb);
  }

  /** @internal */
  static _getById(id: number): Window | undefined {
    return windowsById.get(id);
  }

  set title(title: string) {
    this._native.title = title;
  }

  set decorations(decorations: boolean) {
    this._native.decorations = decorations;
  }

  set visible(visible: boolean) {
    this._native.visible = visible;
  }

  set transparent(transparent: boolean) {
    this._native.transparent = transparent;
  }

  set resizable(resizable: boolean) {
    this._native.resizable = resizable;
  }

  set maximized(maximized: boolean) {
    this._native.maximized = maximized;
  }

  set minimized(minimized: boolean) {
    this._native.minimized = minimized;
  }

  set fullscreen(fullscreen: boolean) {
    this._native.fullscreen = fullscreen;
  }

  set windowLevel(windowLevel: WindowLevel) {
    this._native.windowLevel = windowLevel;
  }

  setMinSize(width: number, height: number): void {
    this._native.setMinSize(width, height);
  }

  setMaxSize(width: number, height: number): void {
    this._native.setMaxSize(width, height);
  }

  setPosition(x: number, y: number): void {
    this._native.setPosition(x, y);
  }

  set theme(preference: WindowTheme) {
    const prev = this.resolvedTheme;
    this._themePreference = preference;
    this._native.theme = preference;
    this._maybeEmitThemeChange(prev);
  }

  focus(): void {
    this._native.focus();
  }

  requestRedraw(): void {
    core.requestRedraw(this._id);
  }

  setVar(key: string, value: string | null): void {
    this._native.setVar(key, value);
  }

  setVars(patch: Record<string, string | null | undefined>): void {
    for (const [key, value] of Object.entries(patch)) {
      this._native.setVar(key, value ?? null);
    }
  }

  requestAnimationFrame(callback: AnimationFrameCallback): number {
    if (typeof callback !== 'function') {
      throw new TypeError('requestAnimationFrame expects a function');
    }

    const handle = this._nextAnimationFrameHandle++;
    this._animationFrameCallbacks.set(handle, callback);

    this._setAnimationFramePending(true);

    return handle;
  }

  cancelAnimationFrame(handle: number): void {
    if (this._animationFrameCallbacks.delete(handle)) {
      this._syncAnimationFramePending();
    }
  }

  set contentProtected(contentProtected: boolean) {
    this._native.contentProtected = contentProtected;
  }

  set closable(closable: boolean) {
    this._native.closable = closable;
  }

  set minimizable(minimizable: boolean) {
    this._native.minimizable = minimizable;
  }

  set maximizable(value: boolean) {
    this._native.maximizable = value;
  }

  get scaleFactor(): number {
    return this._native.scaleFactor ?? 1;
  }

  get innerWidth(): number {
    return this._native.innerWidth ?? DEFAULT_WINDOW_WIDTH;
  }

  get innerHeight(): number {
    return this._native.innerHeight ?? DEFAULT_WINDOW_HEIGHT;
  }

  get title(): string {
    return this._native.title ?? DEFAULT_WINDOW_TITLE;
  }

  get visible(): boolean {
    return this._native.visible ?? true;
  }

  get transparent(): boolean {
    return this._native.transparent ?? false;
  }

  get resizable(): boolean {
    return this._native.resizable ?? true;
  }

  get decorations(): boolean {
    return this._native.decorations ?? true;
  }

  get maximized(): boolean {
    return this._native.maximized ?? false;
  }

  get minimized(): boolean {
    return this._native.minimized ?? false;
  }

  get fullscreen(): boolean {
    return this._native.fullscreen ?? false;
  }

  get alwaysOnTop(): boolean {
    return this.windowLevel === 'alwaysOnTop';
  }

  get alwaysOnBottom(): boolean {
    return this.windowLevel === 'alwaysOnBottom';
  }

  get windowLevel(): WindowLevel {
    return this._native.windowLevel ?? DEFAULT_WINDOW_LEVEL;
  }

  get innerSize(): WindowSize | null {
    return this._native.innerSize;
  }

  get outerSize(): WindowSize | null {
    return this._native.outerSize;
  }

  get position(): WindowPosition | null {
    return this._native.position;
  }

  get theme(): WindowTheme {
    return this._themePreference;
  }

  /**
   * The effective theme after resolving a `system` preference against the OS.
   * Always `light` or `dark`. Track changes with the `themechange` event.
   */
  get resolvedTheme(): ResolvedTheme {
    return this._themePreference === 'system'
      ? this._systemTheme
      : this._themePreference;
  }

  get active(): boolean | null {
    return this._native.active;
  }

  get contentProtected(): boolean {
    return this._native.contentProtected ?? false;
  }

  get closable(): boolean {
    return this._native.closable ?? true;
  }

  get minimizable(): boolean {
    return this._native.minimizable ?? true;
  }

  get maximizable(): boolean {
    return this._native.maximizable ?? true;
  }

  get label(): string {
    return this._label;
  }

  get id(): number {
    return this._id;
  }

  get root(): UzRootElement {
    if (!this._root) {
      this._root = new UzRootElement(this);
    }
    return this._root;
  }

  createElement<T extends ElementTagName>(type: T): ElementForTag<T>;
  createElement(type: string): Element<any>;
  createElement(type: string): Element<any> {
    const Constructor = ELEMENT_CONSTRUCTORS[type as ElementTagName] as
      | ElementConstructor
      | undefined;
    if (Constructor) return new Constructor(this);
    return new UzElement(type, this);
  }

  createTextNode(text: string): UzTextNode {
    return new UzTextNode(this, text);
  }

  get isDisposed(): boolean {
    return this._disposed;
  }

  get remBase(): number {
    return this._native.remBase ?? this._remBase;
  }

  set remBase(value: number) {
    this._remBase = value;
    this._native.remBase = value;
  }

  on<K extends WindowEventName>(
    eventName: K,
    handler: WindowEventHandler<K>,
    options?: ListenerOptions,
  ): void {
    this._emitter.on(eventName, handler, options);
  }

  off<K extends WindowEventName>(
    eventName: K,
    handler: WindowEventHandler<K>,
    options?: ListenerOptions,
  ): void {
    this._emitter.off(eventName, handler, options);
  }

  /** @internal Fire a lifecycle event (load/close/resize). */
  _dispatchLifecycle(
    name: 'load' | 'close' | 'resize',
    payload?: any,
  ): boolean {
    const event = buildLifecycleEvent(name, payload);
    this._emitter.emit(name, event as any);
    return event.defaultPrevented;
  }

  /** @internal Apply an OS theme change forwarded from the native layer. */
  _onSystemThemeChange(osTheme: ResolvedTheme): void {
    const prev = this.resolvedTheme;
    this._systemTheme = osTheme;
    this._maybeEmitThemeChange(prev);
  }

  private _maybeEmitThemeChange(prevResolved: ResolvedTheme): void {
    const resolved = this.resolvedTheme;
    if (resolved === prevResolved) return;
    const event = buildLifecycleEvent('themechange', {
      theme: resolved,
      preference: this._themePreference,
    });
    this._emitter.emit('themechange', event as any);
  }

  /** @internal */
  _flushAnimationFrameCallbacks(): void {
    if (this._animationFrameCallbacks.size === 0) {
      return;
    }

    const callbacks = this._animationFrameCallbacks;
    this._animationFrameCallbacks = new Map();
    const timestamp = globalThis.performance.now();

    for (const callback of callbacks.values()) {
      try {
        callback(timestamp);
      } catch (error) {
        console.error(error);
      }
    }

    this._syncAnimationFramePending();
  }

  private _clearAnimationFrameCallbacks(): void {
    if (this._animationFrameCallbacks.size === 0) {
      return;
    }
    this._animationFrameCallbacks.clear();
    this._setAnimationFramePending(false);
  }

  private _syncAnimationFramePending(): void {
    this._setAnimationFramePending(this._animationFrameCallbacks.size > 0);
  }

  private _setAnimationFramePending(pending: boolean): void {
    if (this._animationFramePendingNotified === pending) {
      return;
    }
    this._animationFramePendingNotified = pending;
    this._native.setAnimationFramePending(pending);
  }
}

export function getWindow(label: string): Window | undefined {
  return windowsByLabel.get(label);
}

/** @internal Called when the native window is destroyed. */
export function disposeWindow(_window: Window) {
  const window = _window as never as {
    id: number;
    label: string;
    _disposed: boolean;
    _disposables: (() => void)[];
    _emitter: { _clear(): void };
    _clearAnimationFrameCallbacks(): void;
  };

  window._clearAnimationFrameCallbacks();
  window._disposed = true;
  for (const cb of window._disposables) {
    cb();
  }
  window._disposables = [];
  window._emitter._clear();
  clearWindowNodes(_window);
  windowsByLabel.delete(window.label);
  windowsById.delete(window.id);
}

/** @internal */
export function __internalDebugNodeCount(windowId: number): number {
  const w = windowsById.get(windowId);
  return w ? nodeCount(w) : 0;
}

/** @internal */
export function flushAnimationFrameCallbacks(windowId: number): void {
  windowsById.get(windowId)?._flushAnimationFrameCallbacks();
}

import type { UzNode } from 'ext:uzumaki/node.ts';
import type { ResolvedTheme, WindowTheme } from 'ext:uzumaki/types.ts';

export const enum EventType {
  MouseMove = 0,
  MouseDown = 1,
  MouseUp = 2,
  Click = 3,
  MouseEnter = 4,
  MouseLeave = 5,
  MouseOver = 6,
  MouseOut = 7,
  KeyDown = 10,
  KeyUp = 11,
  Input = 20,
  Focus = 21,
  Blur = 22,
  BeforeInput = 23,
  Commit = 24,
  Copy = 25,
  Cut = 26,
  Paste = 27,
  TextUpdate = 28,
}

export const enum EventPhase {
  None = 0,
  Capture = 1,
  Target = 2,
  Bubble = 3,
}

export interface UzumakiEvent<T extends UzNode = UzNode> {
  readonly type: EventType | string;
  readonly target: UzNode | null;
  currentTarget: T | null;
  readonly eventPhase: EventPhase;
  readonly bubbles: boolean;
  readonly defaultPrevented: boolean;
  stopPropagation(): void;
  stopImmediatePropagation(): void;
  preventDefault(): void;
}

export interface UzMouseEvent<
  T extends UzNode = UzNode,
> extends UzumakiEvent<T> {
  readonly x: number;
  readonly y: number;
  /** Cursor position relative to the target element's top-left corner. */
  readonly localX: number;
  readonly localY: number;
  readonly screenX: number;
  readonly screenY: number;
  readonly button: number;
  readonly buttons: number;
  /**
   * The element the pointer entered from or exited to. Set for
   * `mouseenter`/`mouseleave`/`mouseover`/`mouseout`; `null` otherwise.
   */
  readonly relatedTarget: UzNode | null;
}

export interface UzKeyboardEvent<
  T extends UzNode = UzNode,
> extends UzumakiEvent<T> {
  readonly key: string;
  readonly code: string;
  readonly keyCode: number;
  readonly repeat: boolean;
  readonly ctrlKey: boolean;
  readonly altKey: boolean;
  readonly shiftKey: boolean;
  readonly metaKey: boolean;
}

export interface UzInputEvent<
  T extends UzNode = UzNode,
> extends UzumakiEvent<T> {
  /**
   * The kind of edit, e.g. `insertText`, `deleteContentBackward`,
   * `historyUndo`. Matches the `input`/`beforeinput` `inputType` semantics.
   */
  readonly inputType: string;
  /** The inserted text, or `null` for deletions and history edits. */
  readonly data: string | null;
}

export interface UzFocusEvent<
  T extends UzNode = UzNode,
> extends UzumakiEvent<T> {}

export interface UzClipboardEvent<
  T extends UzNode = UzNode,
> extends UzumakiEvent<T> {
  readonly selectionText: string | null;
  readonly clipboardText: string | null;
}

export interface UzumakiResizeEvent<
  T extends UzNode = UzNode,
> extends UzumakiEvent<T> {
  readonly width: number;
  readonly height: number;
}

export interface UzThemeChangeEvent<
  T extends UzNode = UzNode,
> extends UzumakiEvent<T> {
  /** The effective theme after resolving `system` against the OS. */
  readonly theme: ResolvedTheme;
  /** The window's theme preference that produced `theme`. */
  readonly preference: WindowTheme;
}

export interface UzSelectionEndpoint {
  /** The text-bearing node the endpoint resolves to. */
  readonly node: UzNode;
  /** Grapheme offset within `node`. */
  readonly offset: number;
}

export interface UzSelectionChangeEvent<
  T extends UzNode = UzNode,
> extends UzumakiEvent<T> {
  /** Where the user started the selection. `null` when there is no selection. */
  readonly anchor: UzSelectionEndpoint | null;
  /** Where the selection currently ends (the moving end). `null` when empty. */
  readonly focus: UzSelectionEndpoint | null;
  /** True when anchor and focus resolve to the same point. */
  readonly isCollapsed: boolean;
}

/** DOM-style events that can be attached to any element. */
export interface UzEventMap {
  /** Pointer moved over the element. Bubbles. */
  mousemove: UzMouseEvent;
  /** A mouse button was pressed over the element. Bubbles. */
  mousedown: UzMouseEvent;
  /** A mouse button was released over the element. Bubbles. */
  mouseup: UzMouseEvent;
  /** A press and release landed on the same element. Bubbles. */
  click: UzMouseEvent;
  /** Pointer entered the element. Fires per element; does not bubble. */
  mouseenter: UzMouseEvent;
  /** Pointer left the element. Fires per element; does not bubble. */
  mouseleave: UzMouseEvent;
  /** Pointer entered the element or a descendant. Bubbles. */
  mouseover: UzMouseEvent;
  /** Pointer left the element or a descendant. Bubbles. */
  mouseout: UzMouseEvent;
  /** A key was pressed while the element was focused. Bubbles. */
  keydown: UzKeyboardEvent;
  /** A key was released while the element was focused. Bubbles. */
  keyup: UzKeyboardEvent;
  /** The element's value changed. Bubbles. */
  input: UzInputEvent;
  /**
   * Fires before an edit is applied to an input. Call `preventDefault()` to
   * stop the edit from committing. Bubbles.
   */
  beforeinput: UzInputEvent;
  /**
   * Edit a keypress would have applied to an `editContext` view. The framework
   * does not mutate the view; the handler must update its own text buffer and
   * re-render. Mirrors the web EditContext API's `textupdate` event. Bubbles.
   */
  textupdate: UzInputEvent;
  /**
   * The value was committed: fires on blur for a text input whose value differs
   * from when it was focused, and immediately when a checkbox toggles. Pairs
   * with the live `input`/`valuechange` events. Bubbles.
   */
  commit: UzInputEvent;
  /** The element gained focus. Does not bubble. */
  focus: UzFocusEvent;
  /** The element lost focus. Does not bubble. */
  blur: UzFocusEvent;
  copy: UzClipboardEvent;
  cut: UzClipboardEvent;
  paste: UzClipboardEvent;
}

/** Window receives all DOM events (for bubble/capture) plus its lifecycle events. */
export interface WindowEventMap extends UzEventMap {
  load: UzumakiEvent;
  close: UzumakiEvent;
  resize: UzumakiResizeEvent;
  themechange: UzThemeChangeEvent;
  /**
   * The active text selection inside this window changed. Fires for view-level
   * selection on `selectable` containers. Does not bubble.
   */
  selectionchange: UzSelectionChangeEvent;
}

export type EventName = keyof UzEventMap;
export type WindowEventName = keyof WindowEventMap;

export type EventHandler<K extends EventName = EventName> = (
  event: UzEventMap[K],
) => void;

export type WindowEventHandler<K extends WindowEventName = WindowEventName> = (
  event: WindowEventMap[K],
) => void;

export const EVENT_NAME_TO_TYPE: Record<string, EventType> = {
  mousemove: EventType.MouseMove,
  mousedown: EventType.MouseDown,
  mouseup: EventType.MouseUp,
  click: EventType.Click,
  mouseenter: EventType.MouseEnter,
  mouseleave: EventType.MouseLeave,
  mouseover: EventType.MouseOver,
  mouseout: EventType.MouseOut,
  keydown: EventType.KeyDown,
  keyup: EventType.KeyUp,
  input: EventType.Input,
  beforeinput: EventType.BeforeInput,
  textupdate: EventType.TextUpdate,
  commit: EventType.Commit,
  focus: EventType.Focus,
  blur: EventType.Blur,
  copy: EventType.Copy,
  cut: EventType.Cut,
  paste: EventType.Paste,
};

export const EVENT_TYPE_TO_NAME: Record<number, EventName> = {
  [EventType.MouseMove]: 'mousemove',
  [EventType.MouseDown]: 'mousedown',
  [EventType.MouseUp]: 'mouseup',
  [EventType.Click]: 'click',
  [EventType.MouseEnter]: 'mouseenter',
  [EventType.MouseLeave]: 'mouseleave',
  [EventType.MouseOver]: 'mouseover',
  [EventType.MouseOut]: 'mouseout',
  [EventType.KeyDown]: 'keydown',
  [EventType.KeyUp]: 'keyup',
  [EventType.Input]: 'input',
  [EventType.BeforeInput]: 'beforeinput',
  [EventType.TextUpdate]: 'textupdate',
  [EventType.Commit]: 'commit',
  [EventType.Focus]: 'focus',
  [EventType.Blur]: 'blur',
  [EventType.Copy]: 'copy',
  [EventType.Cut]: 'cut',
  [EventType.Paste]: 'paste',
};

export const NON_BUBBLING_TYPES: ReadonlySet<EventType> = new Set([
  EventType.Focus,
  EventType.Blur,
  EventType.MouseEnter,
  EventType.MouseLeave,
]);

function isMouseType(t: EventType): boolean {
  return t >= 0 && t <= 7;
}

function isKeyboardType(t: EventType): boolean {
  return t >= 10 && t <= 11;
}

function isInputType(t: EventType): boolean {
  return (
    t === EventType.Input ||
    t === EventType.BeforeInput ||
    t === EventType.TextUpdate ||
    t === EventType.Commit
  );
}

function isFocusType(t: EventType): boolean {
  return t === EventType.Focus || t === EventType.Blur;
}

function isClipboardType(t: EventType): boolean {
  return t === EventType.Copy || t === EventType.Cut || t === EventType.Paste;
}

interface InternalFlags {
  _stopped: boolean;
  _stoppedImmediate: boolean;
  _prevented: boolean;
  _phase: EventPhase;
}

export interface UzEventInit<T extends UzNode = UzNode> {
  bubbles?: boolean;
  currentTarget?: T | null;
  eventPhase?: EventPhase;
}

export class UzEvent<T extends UzNode = UzNode> implements UzumakiEvent<T> {
  readonly type: EventType | string;
  currentTarget: T | null;
  readonly bubbles: boolean;
  private _target: UzNode | null;
  private readonly _flags: InternalFlags;

  constructor(
    type: EventType | string,
    targetOrInit: UzNode | UzEventInit<T> | null = null,
    init: UzEventInit<T> = {},
  ) {
    const target =
      targetOrInit && isEventInit(targetOrInit) ? null : targetOrInit;
    const {
      bubbles = false,
      currentTarget = target as T | null,
      eventPhase = EventPhase.None,
    } = targetOrInit && isEventInit(targetOrInit) ? targetOrInit : init;

    this.type = type;
    this._target = target as UzNode | null;
    this.currentTarget = currentTarget;
    this.bubbles = bubbles;
    this._flags = {
      _stopped: false,
      _stoppedImmediate: false,
      _prevented: false,
      _phase: eventPhase,
    };
  }

  get target(): UzNode | null {
    return this._target;
  }

  get eventPhase(): EventPhase {
    return this._flags._phase;
  }

  get defaultPrevented(): boolean {
    return this._flags._prevented;
  }

  stopPropagation(): void {
    this._flags._stopped = true;
  }

  stopImmediatePropagation(): void {
    this._flags._stopped = true;
    this._flags._stoppedImmediate = true;
  }

  preventDefault(): void {
    this._flags._prevented = true;
  }

  /** @internal */
  _getFlags(): InternalFlags {
    return this._flags;
  }

  /** @internal */
  _setPhase(phase: EventPhase): void {
    this._flags._phase = phase;
  }

  /** @internal */
  _setTarget(target: UzNode | null): void {
    this._target = target;
  }
}

function isEventInit(value: object): value is UzEventInit {
  return (
    'bubbles' in value || 'currentTarget' in value || 'eventPhase' in value
  );
}

export function buildDomEvent(
  type: EventType,
  target: UzNode | null,
  payload: any,
): UzumakiEvent {
  const bubbles = !NON_BUBBLING_TYPES.has(type);

  const base = new UzEvent(type, target, { bubbles });

  if (isMouseType(type)) {
    return Object.assign(base, {
      x: payload?.x ?? 0,
      y: payload?.y ?? 0,
      localX: payload?.localX ?? 0,
      localY: payload?.localY ?? 0,
      screenX: payload?.screenX ?? 0,
      screenY: payload?.screenY ?? 0,
      button: payload?.button ?? 0,
      buttons: payload?.buttons ?? 0,
      relatedTarget: payload?.relatedTarget ?? null,
    }) as UzMouseEvent;
  }

  if (isKeyboardType(type)) {
    const mods: number = payload?.modifiers ?? 0;
    return Object.assign(base, {
      key: payload?.key ?? '',
      code: payload?.code ?? '',
      keyCode: payload?.keyCode ?? 0,
      repeat: payload?.repeat ?? false,
      ctrlKey: !!(mods & 1),
      altKey: !!(mods & 2),
      shiftKey: !!(mods & 4),
      metaKey: !!(mods & 8),
    }) as UzKeyboardEvent;
  }

  if (isInputType(type)) {
    return Object.assign(base, {
      value: payload?.value ?? '',
      inputType: payload?.inputType ?? '',
      data: payload?.data ?? null,
    }) as UzInputEvent;
  }

  if (isClipboardType(type)) {
    return Object.assign(base, {
      selectionText: payload?.selectionText ?? null,
      clipboardText: payload?.clipboardText ?? null,
    }) as UzClipboardEvent;
  }

  if (isFocusType(type)) {
    return base as UzFocusEvent;
  }

  return base;
}

export function buildLifecycleEvent(
  type: string,
  payload: any,
):
  | UzumakiEvent
  | UzumakiResizeEvent
  | UzThemeChangeEvent
  | UzSelectionChangeEvent {
  const base = new UzEvent(type, null, {
    currentTarget: null,
    eventPhase: EventPhase.Target,
  });

  if (type === 'resize') {
    return Object.assign(base, {
      width: payload?.width ?? 0,
      height: payload?.height ?? 0,
    }) as UzumakiResizeEvent;
  }

  if (type === 'themechange') {
    return Object.assign(base, {
      theme: payload?.theme ?? 'light',
      preference: payload?.preference ?? 'system',
    }) as UzThemeChangeEvent;
  }

  if (type === 'selectionchange') {
    return Object.assign(base, {
      anchor: payload?.anchor ?? null,
      focus: payload?.focus ?? null,
      isCollapsed: payload?.isCollapsed ?? true,
    }) as UzSelectionChangeEvent;
  }

  return base;
}

/** @internal Reads private flags set by buildDomEvent. */
export function _eventFlags(event: UzumakiEvent): InternalFlags {
  if (event instanceof UzEvent) return event._getFlags();
  const flags = (event as any)._flags as InternalFlags | undefined;
  if (flags) return flags;
  throw new Error('Cannot dispatch an event without internal state');
}

/** @internal Set the current phase on an event built by buildDomEvent. */
export function _setEventPhase(event: UzumakiEvent, phase: EventPhase): void {
  if (event instanceof UzEvent) {
    event._setPhase(phase);
    return;
  }
  _eventFlags(event)._phase = phase;
}

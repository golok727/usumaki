// @ts-expect-error hope its there :3
import { primordials } from 'ext:core/mod.js';

import {
  op_get_uz_runtime_version,
  AppPath as CoreAppPath,
  // @ts-expect-error it is what it is
} from 'ext:core/ops';
import 'ext:uzumaki/types.ts';
import 'ext:uzumaki/window.ts';
import 'ext:uzumaki/events.ts';
import 'ext:uzumaki/dispatcher.ts';

import {
  Window,
  disposeWindow,
  flushAnimationFrameCallbacks,
} from 'ext:uzumaki/window.ts';
import { EventType as UzEventType } from 'ext:uzumaki/events.ts';
import { dispatchDomEvent } from 'ext:uzumaki/dispatcher.ts';
import { AppPath } from 'ext:uzumaki/types.ts';

const { ObjectDefineProperty } = primordials;

// todo find a better way to do this
let appPath: AppPath;
ObjectDefineProperty(globalThis, 'Uz', {
  value: {
    get path() {
      if (appPath === undefined) appPath = new CoreAppPath();
      return appPath;
    },
  },
  writable: false,
  configurable: false,
});

export type { AppPath };

declare global {
  const Uz: {
    path: AppPath;
  };
}

export { getWindow, Window } from 'ext:uzumaki/window.ts';
export type {
  WindowOptions,
  WindowLevel,
  WindowPosition,
  WindowSize,
  WindowTheme,
} from 'ext:uzumaki/types.ts';
export { UzNode, UzTextNode } from 'ext:uzumaki/node.ts';
export { Element } from 'ext:uzumaki/elements/element.ts';
export { UzElement } from 'ext:uzumaki/elements/base.ts';
export { UzRootElement } from 'ext:uzumaki/elements/root.ts';
export { UzViewElement } from 'ext:uzumaki/elements/view.ts';
export { UzTextElement } from 'ext:uzumaki/elements/text.ts';
export { UzButtonElement } from 'ext:uzumaki/elements/button.ts';
export { UzImageElement } from 'ext:uzumaki/elements/image.ts';
export { UzInputElement } from 'ext:uzumaki/elements/input.ts';
export { UzCheckboxElement } from 'ext:uzumaki/elements/checkbox.ts';

export { Clipboard } from 'ext:uzumaki/clipboard.ts';
export { UzEventTarget as EventEmitter } from 'ext:uzumaki/event-target.ts';
export { EventType, UzEvent, EventPhase } from 'ext:uzumaki/events.ts';
export type {
  EventName,
  EventHandler,
  UzEventMap,
  WindowEventName,
  WindowEventHandler,
  WindowEventMap,
  UzumakiEvent,
  UzMouseEvent,
  UzKeyboardEvent,
  UzInputEvent,
  UzFocusEvent,
  UzClipboardEvent,
  UzumakiResizeEvent,
} from 'ext:uzumaki/events.ts';

ObjectDefineProperty(globalThis, '__uzumaki_flush_animation_frame__', {
  value: flushAnimationFrameCallbacks,
  writable: false,
  configurable: false,
});

function defineDispatch(name: string, fn: (...args: any[]) => unknown): void {
  ObjectDefineProperty(globalThis, name, {
    value: fn,
    writable: false,
    configurable: false,
  });
}

function dispatchToNode(
  windowId: number,
  type: UzEventType,
  nodeId: number | null,
  payload: any,
): boolean {
  const w = Window._getById(windowId);
  if (!w) return false;
  return dispatchDomEvent(w, type, nodeId, payload);
}

defineDispatch(
  '__uzumaki_dispatch_mouse__',
  (
    type: UzEventType,
    windowId: number,
    nodeId: number,
    x: number,
    y: number,
    screenX: number,
    screenY: number,
    button: number,
    buttons: number,
  ) =>
    dispatchToNode(windowId, type, nodeId, {
      x,
      y,
      screenX,
      screenY,
      button,
      buttons,
    }),
);

defineDispatch(
  '__uzumaki_dispatch_keyboard__',
  (
    type: UzEventType,
    windowId: number,
    nodeId: number | null,
    key: string,
    code: string,
    keyCode: number,
    modifiers: number,
    repeat: boolean,
  ) =>
    dispatchToNode(windowId, type, nodeId, {
      key,
      code,
      keyCode,
      modifiers,
      repeat,
    }),
);

defineDispatch(
  '__uzumaki_dispatch_input__',
  (windowId: number, nodeId: number, inputType: string, data: string | null) =>
    dispatchToNode(windowId, UzEventType.Input, nodeId, {
      inputType,
      data,
    }),
);

defineDispatch(
  '__uzumaki_dispatch_focus__',
  (type: UzEventType, windowId: number, nodeId: number) =>
    dispatchToNode(windowId, type, nodeId, {}),
);

defineDispatch(
  '__uzumaki_dispatch_clipboard__',
  (
    type: UzEventType,
    windowId: number,
    nodeId: number | null,
    selectionText: string | null,
    clipboardText: string | null,
  ) =>
    dispatchToNode(windowId, type, nodeId, {
      selectionText,
      clipboardText,
    }),
);

defineDispatch(
  '__uzumaki_dispatch_resize__',
  (windowId: number, width: number, height: number) => {
    const w = Window._getById(windowId);
    if (w) w._dispatchLifecycle('resize', { width, height });
  },
);

defineDispatch('__uzumaki_window_load__', (windowId: number) => {
  const w = Window._getById(windowId);
  if (w) w._dispatchLifecycle('load');
});

defineDispatch('__uzumaki_window_close__', (windowId: number) => {
  const w = Window._getById(windowId);
  if (w) {
    w._dispatchLifecycle('close');
    disposeWindow(w);
  }
});

defineDispatch(
  '__uzumaki_theme_changed__',
  (windowId: number, isDark: boolean) => {
    const w = Window._getById(windowId);
    if (w) w._onSystemThemeChange(isDark ? 'dark' : 'light');
  },
);

defineDispatch('__uzumaki_hot_reload__', () => {
  console.log('[uzumaki] Hot reload');
});

/**
 * Build a theme ref object from a map of tokens.
 *
 * Returns `{ vars, theme }`. Pass `vars` to a window's `vars` option (or
 * `window.setVars(...)`); use `theme.token` anywhere a style prop value is
 * expected. Values that start with `$` are looked up from the window's
 * theme at paint time.
 *
 * @example
 * const { vars, theme } = defineVars({ bg: '#0a0a0a', text: '#e4e4e7' });
 * new Window('main', { vars, rootStyles: { bg: theme.bg, color: theme.text } });
 */
export function defineVars<T extends Record<string, string>>(
  tokens: T,
): { vars: T; theme: { [K in keyof T]: string } } {
  const theme = Object.fromEntries(
    Object.keys(tokens).map((k) => [k, `$${k}`]),
  ) as { [K in keyof T]: string };
  return { vars: tokens, theme };
}

export const RUNTIME_VERSION: number = op_get_uz_runtime_version();

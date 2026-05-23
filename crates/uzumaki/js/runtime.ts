// @ts-expect-error hope its there :3
import { primordials } from 'ext:core/mod.js';

import {
  op_get_uz_runtime_version,
  AppPath as CoreAppPath,
  // @ts-expect-error it is what it is
} from 'ext:core/ops';
import { dispatchAppEvent, onAppEvent } from 'ext:uzumaki/core.ts';

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

const EVENT_TYPE_MAP: Record<string, UzEventType> = {
  mouseDown: UzEventType.MouseDown,
  mouseUp: UzEventType.MouseUp,
  click: UzEventType.Click,
  keyDown: UzEventType.KeyDown,
  keyUp: UzEventType.KeyUp,
  input: UzEventType.Input,
  focus: UzEventType.Focus,
  blur: UzEventType.Blur,
  copy: UzEventType.Copy,
  cut: UzEventType.Cut,
  paste: UzEventType.Paste,
};

ObjectDefineProperty(globalThis, '__uzumaki_on_app_event__', {
  value: function (event: any /** Todo type */) {
    return dispatchAppEvent(event);
  },
  writable: false,
  configurable: false,
});

ObjectDefineProperty(globalThis, '__uzumaki_flush_animation_frame__', {
  value: flushAnimationFrameCallbacks,
  writable: false,
  configurable: false,
});

/**
 * Subscribe
 */
onAppEvent((event: AppEvent, ctx) => {
  if (event.type === 'windowLoad') {
    const w = Window._getById(event.windowId);
    if (w) w._dispatchLifecycle('load');
    return;
  }

  if (event.type === 'windowClose') {
    const w = Window._getById(event.windowId);
    if (w) {
      w._dispatchLifecycle('close');
      disposeWindow(w);
    }
    return;
  }

  if (event.type === 'resize') {
    const w = Window._getById(event.windowId);
    if (w) {
      w._dispatchLifecycle('resize', {
        width: event.width ?? 0,
        height: event.height ?? 0,
      });
    }
    return;
  }

  if (event.type === 'themeChanged') {
    const w = Window._getById(event.windowId);
    if (w) w._onSystemThemeChange(event.theme === 'dark' ? 'dark' : 'light');
    return;
  }

  if (event.type === 'hotReload') {
    console.log('[uzumaki] Hot reload');
    return;
  }

  const eventType = EVENT_TYPE_MAP[event.type];
  if (eventType === undefined) return;

  const w = Window._getById(event.windowId);
  if (!w) return;

  const prevented = dispatchDomEvent(w, eventType, event.nodeId ?? null, event);
  if (prevented) ctx.preventDefault();
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

interface AppEvent {
  type: string;
  windowId: number;
  nodeId?: any;
  key?: string;
  code?: string;
  keyCode?: number;
  modifiers?: number;
  repeat?: boolean;
  width?: number;
  height?: number;
  x?: number;
  y?: number;
  screenX?: number;
  screenY?: number;
  button?: number;
  buttons?: number;
  value?: string;
  inputType?: string;
  data?: string | null;
  theme?: string;
}

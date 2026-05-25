import core from 'ext:uzumaki/core.ts';
import { UzEventTarget } from 'ext:uzumaki/event-target.ts';
import {
  EventPhase,
  EventType,
  EVENT_TYPE_TO_NAME,
  _eventFlags,
  _setEventPhase,
  buildDomEvent,
  type EventName,
  type UzumakiEvent,
} from 'ext:uzumaki/events.ts';
import { getNode } from 'ext:uzumaki/registry.ts';
import type { NodeId } from 'ext:uzumaki/types.ts';
import type { Window } from 'ext:uzumaki/window.ts';

function nodeAt(window: Window, id: NodeId | null) {
  if (id == null) return null;
  return getNode(window, id) ?? null;
}

function eventNodeEmitter(
  node: any,
): UzEventTarget<Record<any, any>> | undefined {
  if (
    node &&
    typeof node === 'object' &&
    node._emitter instanceof UzEventTarget
  ) {
    return (node as any)._emitter;
  }
  return undefined;
}

function fireEmitter(
  emitter: {
    _listeners(
      name: EventName,
    ): readonly { handler: Function; capture: boolean }[] | undefined;
  },
  name: EventName,
  event: UzumakiEvent,
  capturePhase: boolean,
): void {
  const list = emitter._listeners(name);
  if (!list) return;
  const flags = _eventFlags(event);
  // snapshot: a handler may call on/off during dispatch
  // eslint-disable-next-line unicorn/no-useless-spread
  for (const entry of [...list]) {
    if (
      event.eventPhase === EventPhase.Target ||
      entry.capture === capturePhase
    ) {
      try {
        entry.handler(event);
      } catch (error) {
        if (error instanceof Error) {
          console.error(error);
        } else {
          console.error('Error', error);
        }
      }
      if (flags._stoppedImmediate) return;
    }
  }
}

/**
 * Walk capture -> target -> bubble for a DOM event originating from a node in
 * `window`. Returns true if `preventDefault()` was called.
 */
export function dispatchDomEvent(
  window: Window,
  type: EventType,
  targetNodeId: NodeId | null,
  payload: any,
): boolean {
  const name = EVENT_TYPE_TO_NAME[type];
  if (!name) return false;

  const target = nodeAt(window, targetNodeId);
  const event = buildDomEvent(type, target, payload);

  // enter/leave fire on a single element with no capture/bubble; Rust already
  // emits one event per crossed element, so route them straight to the node.
  if (type === EventType.MouseEnter || type === EventType.MouseLeave) {
    if (targetNodeId != null) fireDirect(window, name, targetNodeId, event);
    return event.defaultPrevented;
  }

  return dispatchEvent(window, name, targetNodeId, event);
}

/** Fire a non-bubbling event on a single node, bypassing capture/bubble. */
function fireDirect(
  window: Window,
  name: EventName,
  nodeId: NodeId,
  event: UzumakiEvent,
): void {
  const node = nodeAt(window, nodeId);
  const emitter = eventNodeEmitter(node);
  if (!emitter) return;
  if ('_setTarget' in event) (event as any)._setTarget(node);
  _setEventPhase(event, EventPhase.Target);
  event.currentTarget = node;
  fireEmitter(emitter, name, event, false);
}

export function dispatchEvent(
  window: Window,
  name: EventName,
  targetNodeId: NodeId | null,
  event: UzumakiEvent,
): boolean {
  const windowId = window.id;
  // todo we dont need thos we can do node.parent
  const path: NodeId[] =
    targetNodeId == null ? [] : core.getAncestorPath(windowId, targetNodeId);

  const target = nodeAt(window, targetNodeId);
  if (!event.target && '_setTarget' in event) {
    (event as any)._setTarget(target);
  }
  const flags = _eventFlags(event);
  const bubbles = event.bubbles;

  // No DOM target: fire window-level bubble handlers only.
  if (path.length === 0) {
    _setEventPhase(event, EventPhase.Bubble);
    event.currentTarget = null;
    fireEmitter(window._emitter as any, name, event, false);
    return flags._prevented;
  }

  // Capture: window -> root -> ... -> parent of target
  _setEventPhase(event, EventPhase.Capture);
  event.currentTarget = null;
  fireEmitter(window._emitter as any, name, event, true);

  for (let i = path.length - 1; i > 0 && !flags._stopped; i--) {
    const node = nodeAt(window, path[i]!);
    const emitter = eventNodeEmitter(node);
    if (emitter) {
      event.currentTarget = node;
      fireEmitter(emitter, name, event, true);
    }
  }

  // Target
  if (!flags._stopped) {
    _setEventPhase(event, EventPhase.Target);
    const node = nodeAt(window, path[0]!);
    const emitter = eventNodeEmitter(node);
    if (emitter) {
      event.currentTarget = node;
      fireEmitter(emitter, name, event, false);
    }
  }

  // Bubble: target -> ... -> root -> window
  if (bubbles && !flags._stopped) {
    _setEventPhase(event, EventPhase.Bubble);
    for (let i = 1; i < path.length && !flags._stopped; i++) {
      const node = nodeAt(window, path[i]!);
      const emitter = eventNodeEmitter(node);
      if (emitter) {
        event.currentTarget = node;
        fireEmitter(emitter, name, event, false);
      }
    }
    if (!flags._stopped) {
      event.currentTarget = null;
      fireEmitter(window._emitter as any, name, event, false);
    }
  }

  return flags._prevented;
}

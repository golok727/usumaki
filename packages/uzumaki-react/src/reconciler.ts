import { createElement, isValidElement as isReactElement } from 'react';
import ReactReconciler, { type EventPriority } from 'react-reconciler';
import { DefaultEventPriority } from 'react-reconciler/constants.js';

import { INTRINSIC_ELEMENTS, __DEV__ } from './constants';

import type { JSX } from './jsx/runtime';

import { UzElement } from 'uzumaki';
import { UzNode, UzTextNode } from 'uzumaki';
import { Window } from 'uzumaki';
import {
  applyProps,
  commitText,
  createElement as createHostElement,
  createText as createHostText,
  hide,
  resetText,
  unhide,
} from './host';
import { WindowProvider } from './window-context';

/**
 * Get text content of a <text> node. will throw an error if you nest a react element inside this
 */
function getTextContent(children: any): string {
  if (children == null) return '';
  if (Array.isArray(children)) {
    return children
      .map((child) => {
        if (__DEV__ && isReactElement(child)) {
          throw new Error(
            `[uzumaki] <text> received a React element as a child (<${child.type}>). ` +
              `Only strings and numbers are allowed inside <text>.`,
          );
        }
        return child == null ? '' : String(child);
      })
      .join('');
  }

  if (__DEV__ && isReactElement(children)) {
    throw new Error(
      `[uzumaki] <text> received a React element as a child (<${children.type}>). ` +
        `Only strings and numbers are allowed inside <text>.`,
    );
  }

  return String(children);
}

function isTextElementType(type: string): boolean {
  return type === 'text';
}

function createElementInstance(
  type: string,
  props: Record<string, any>,
  window: Window,
): UzElement {
  if (!INTRINSIC_ELEMENTS.has(type)) {
    throw new Error(
      `[uzumaki] Unknown intrinsic element: <${type}>. Did you mean <view>?`,
    );
  }
  const normalizedProps = isTextElementType(type)
    ? { ...props, children: getTextContent(props.children) }
    : props;
  return createHostElement(window, type, normalizedProps);
}

type Type = string;
type Props = Record<string, any>;
type Instance = UzElement;
type TextInstance = UzTextNode;
type SuspenseInstance = any;
type HydratableInstance = any;
type FormInstance = any;
type PublicInstance = UzNode;
type HostContext = {};
type ChildSet = any;
type TimeoutHandle = ReturnType<typeof setTimeout>;
type NoTimeout = undefined;
type TransitionStatus = any;

let currentPriority: EventPriority = DefaultEventPriority;

function createReconciler() {
  return ReactReconciler<
    Type,
    Props,
    Window,
    Instance,
    TextInstance,
    SuspenseInstance,
    HydratableInstance,
    FormInstance,
    PublicInstance,
    HostContext,
    ChildSet,
    TimeoutHandle,
    NoTimeout,
    TransitionStatus
  >({
    supportsMutation: true,
    supportsPersistence: false,

    createInstance(type, props, window) {
      return createElementInstance(type, props, window);
    },

    createTextInstance(text, window) {
      return createHostText(window, text);
    },

    shouldSetTextContent(type) {
      return isTextElementType(type);
    },

    appendInitialChild(parent, child) {
      parent.appendChild(child);
    },

    finalizeInitialChildren() {
      return false;
    },

    appendChildToContainer(window, child) {
      if (window.isDisposed) return;
      window.root.appendChild(child);
    },

    appendChild(parent, child) {
      parent.appendChild(child);
    },

    insertBefore(parent, child, before) {
      parent.insertBefore(child, before);
    },

    insertInContainerBefore(window, child, before) {
      if (window.isDisposed) return;
      window.root.insertBefore(child, before);
    },

    removeChild(parent, child) {
      if (parent.window.isDisposed) return;
      parent.removeChild(child);
    },

    removeChildFromContainer(window, child) {
      if (window.isDisposed) return;
      window.root.removeChild(child);
    },

    commitUpdate(instance, _type, oldProps, newProps, _internalHandle) {
      if (instance.window.isDisposed) return;
      const normalizedNewProps = isTextElementType(instance.type)
        ? { ...newProps, children: getTextContent(newProps.children) }
        : newProps;
      const normalizedOldProps = isTextElementType(instance.type)
        ? { ...oldProps, children: getTextContent(oldProps.children) }
        : oldProps;
      applyProps(instance, normalizedNewProps, normalizedOldProps);
    },

    commitTextUpdate(instance, _oldText, newText) {
      if (instance.window.isDisposed) return;
      commitText(instance, newText);
    },

    detachDeletedInstance() {},

    hideInstance(instance) {
      hide(instance);
    },

    unhideInstance(instance) {
      unhide(instance);
    },

    hideTextInstance() {},

    unhideTextInstance() {},

    resetTextContent(instance) {
      resetText(instance);
    },

    clearContainer(window) {
      window.root.removeChildren();
    },

    getRootHostContext: () => ({}),
    getChildHostContext: (parentHostContext) => parentHostContext,
    getPublicInstance: (instance) => instance,

    prepareForCommit(_window) {
      return null;
    },

    resetAfterCommit(window) {
      window.requestRedraw();
    },

    preparePortalMount: () => {},
    scheduleTimeout: (fn, delay) => setTimeout(fn, delay),
    cancelTimeout: (id) => clearTimeout(id),
    noTimeout: undefined,
    isPrimaryRenderer: true,
    getInstanceFromNode: () => null,
    beforeActiveInstanceBlur: () => {},
    afterActiveInstanceBlur: () => {},
    prepareScopeUpdate: () => {},
    getInstanceFromScope: () => null,
    supportsHydration: false,
    NotPendingTransition: undefined,
    HostTransitionContext: {
      $$typeof: Symbol.for('react.context'),
      _currentValue: null,
      _currentValue2: null,
    } as any,
    setCurrentUpdatePriority: (newPriority) => {
      currentPriority = newPriority;
    },
    getCurrentUpdatePriority: () => currentPriority,
    resolveUpdatePriority: () => DefaultEventPriority,
    resetFormInstance: () => {},
    requestPostPaintCallback: () => {},
    shouldAttemptEagerTransition: () => false,
    trackSchedulerEvent: () => {},
    resolveEventType: () => null,
    resolveEventTimeStamp: () => Date.now(),
    maySuspendCommit: () => false,
    preloadInstance: () => false,
    startSuspendingCommit: () => false,
    suspendInstance: () => {},
    waitForCommitToBeReady: () => null,
  });
}

export interface Root {
  render(element: JSX.Element | null): void;
  unmount(): void;
}

export function createRoot(window: Window): Root {
  const reconciler = createReconciler();

  const container = reconciler.createContainer(
    window,
    1,
    null,
    false,
    null,
    '',
    console.error,
    console.error,
    console.error,
    () => {},
  );

  let unmounted = false;

  function unmount() {
    if (unmounted) return;
    unmounted = true;
    reconciler.updateContainer(null, container, null, null);
  }

  window.addDisposable(unmount);

  return {
    render(element) {
      if (unmounted) {
        throw new Error('[uzumaki] cannot render into an unmounted root');
      }
      reconciler.updateContainer(
        createElement(WindowProvider, { window } as never, element),
        container,
        null,
        null,
      );
    },
    unmount,
  };
}

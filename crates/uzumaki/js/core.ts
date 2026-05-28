import type {
  NodeId,
  WindowOptions,
  WindowLevel,
  WindowPosition,
  WindowSize,
  WindowTheme,
} from './types';

import {
  op_create_window,
  op_request_quit,
  op_request_redraw,
  op_get_root_node,
  op_create_element_node,
  op_create_text_node,
  op_set_encoded_image_data,
  op_apply_cached_image,
  op_clear_image_data,
  op_focus_element,
  op_get_ancestor_path,
  op_read_clipboard_text,
  op_write_clipboard_text,
  // @ts-expect-error it is what it is
} from 'ext:core/ops';

// @ts-expect-error registered via the `objects = [...]` list on the extension;
// exposed as a JS-constructible cppgc class.
import { CoreNode as CoreNodeImpl } from 'ext:core/ops';
export const CoreNode = CoreNodeImpl as CoreNodeConstructor;

export interface CoreWindow {
  readonly id: number;

  close(): void;

  readonly innerWidth: number | null;
  readonly innerHeight: number | null;

  getSelection(): CoreSelection;

  title: string | null;
  visible: boolean | null;
  transparent: boolean | null;
  resizable: boolean | null;
  decorations: boolean | null;
  maximized: boolean | null;
  minimized: boolean | null;
  fullscreen: boolean | null;
  windowLevel: WindowLevel | null;

  setMinSize(width: number, height: number): boolean;
  setMaxSize(width: number, height: number): boolean;

  readonly innerSize: WindowSize | null;
  readonly outerSize: WindowSize | null;
  readonly position: WindowPosition | null;

  setPosition(x: number, y: number): boolean;

  readonly scaleFactor: number | null;

  theme: WindowTheme | null;

  readonly active: boolean | null;
  focus(): boolean;
  setAnimationFramePending(pending: boolean): boolean;

  contentProtected: boolean | null;
  closable: boolean | null;
  minimizable: boolean | null;
  maximizable: boolean | null;

  remBase: number;

  setVar(key: string, value: string | null): boolean;
}

export interface CoreNodeConstructor {
  new (windowId: number, nodeId: NodeId): CoreNode;
}

export interface CoreSelection {
  readonly windowId: number;
  readonly isActive: boolean;
  readonly isCollapsed: boolean;
  readonly anchorNodeId: NodeId | null;
  readonly anchorOffset: number;
  readonly focusNodeId: NodeId | null;
  readonly focusOffset: number;
  readonly text: string;
  collapse(nodeId: NodeId, offset: number): void;
  extend(nodeId: NodeId, offset: number): void;
  set(
    anchorNodeId: NodeId,
    anchorOffset: number,
    focusNodeId: NodeId,
    focusOffset: number,
  ): void;
  empty(): void;
}

export interface CoreNode {
  readonly id: NodeId;
  readonly windowId: number;
  readonly nodeType: number;
  readonly nodeName: string;
  readonly parentNodeId: NodeId | null;
  readonly firstChildId: NodeId | null;
  readonly lastChildId: NodeId | null;
  readonly nextSiblingId: NodeId | null;
  readonly previousSiblingId: NodeId | null;
  textContent: string | null;
  appendChild(child: CoreNode): void;
  insertBefore(child: CoreNode, before: CoreNode | null): void;
  removeChild(child: CoreNode): void;
  remove(): void;
  clearChildren(): void;
  setAttribute(name: string, value: string): void;
  removeAttribute(name: string): void;
  getAttribute(name: string): unknown;
  scrollIntoView(block: number, inline: number): void;
}

interface Core {
  createWindow(options: WindowOptions): CoreWindow;
  requestQuit(): void;
  requestRedraw(windowId: number): void;
  getRootNode(windowId: number): CoreNode;
  createElementNode(windowId: number, elementType: string): CoreNode;
  createTextNode(windowId: number, text: string): CoreNode;
  setEncodedImageData(
    windowId: number,
    nodeId: NodeId,
    cacheKey: string,
    data: Uint8Array,
  ): void;
  applyCachedImage(windowId: number, nodeId: NodeId, cacheKey: string): boolean;
  clearImageData(windowId: number, nodeId: NodeId): void;
  focusElement(windowId: number, nodeId: NodeId): void;
  getAncestorPath(windowId: number, nodeId: NodeId): NodeId[];
  readClipboardText(): Promise<string | null>;
  writeClipboardText(text: string): Promise<boolean>;
}

const core: Core = {
  createWindow: op_create_window,
  requestQuit: op_request_quit,
  requestRedraw: op_request_redraw,
  getRootNode: op_get_root_node,
  createElementNode: op_create_element_node,
  createTextNode: op_create_text_node,
  setEncodedImageData: op_set_encoded_image_data,
  applyCachedImage: op_apply_cached_image,
  clearImageData: op_clear_image_data,
  focusElement: op_focus_element,
  getAncestorPath: op_get_ancestor_path,
  readClipboardText: op_read_clipboard_text,
  writeClipboardText: op_write_clipboard_text,
};

export default core;

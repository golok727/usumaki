import core, { CoreNode } from 'ext:uzumaki/core.ts';
import { getNode, registerNode } from 'ext:uzumaki/registry.ts';
import type { NodeId } from 'ext:uzumaki/types.ts';
import type { Window } from 'ext:uzumaki/window.ts';

export type ScrollAlign = 'start' | 'center' | 'end' | 'nearest';

const SCROLL_ALIGN: Record<ScrollAlign, number> = {
  start: 0,
  center: 1,
  end: 2,
  nearest: 3,
};

export const NodeType = {
  Root: 1,
  Element: 2,
  Text: 3,
} as const;

export class UzNode {
  protected readonly _native: CoreNode;
  readonly window: Window;
  /**
   * Strong refs to child wrappers to avoid gc. ( todo find a better approach  use TracedReference?)
   */
  private readonly _childWrappers: Set<UzNode> = new Set();
  private _parentWrapper: UzNode | null = null;

  constructor(window: Window, native: CoreNode) {
    this.window = window;
    this._native = native;
    registerNode(this);
  }

  private _setParentWrapper(parent: UzNode | null): void {
    if (this._parentWrapper === parent) return;
    if (this._parentWrapper) {
      this._parentWrapper._childWrappers.delete(this);
    }
    this._parentWrapper = parent;
    if (parent) {
      parent._childWrappers.add(this);
    }
  }

  get nodeId(): NodeId {
    return this._native.id;
  }

  get windowId(): number {
    return this._native.windowId;
  }

  get nodeType(): number {
    return this._native.nodeType;
  }

  get parentNode(): UzNode | null {
    return resolveNode(this.window, this._native.parentNodeId);
  }

  get firstChild(): UzNode | null {
    return resolveNode(this.window, this._native.firstChildId);
  }

  get lastChild(): UzNode | null {
    return resolveNode(this.window, this._native.lastChildId);
  }

  get nextSibling(): UzNode | null {
    return resolveNode(this.window, this._native.nextSiblingId);
  }

  get previousSibling(): UzNode | null {
    return resolveNode(this.window, this._native.previousSiblingId);
  }

  get textContent(): string | null {
    return this._native.textContent;
  }

  set textContent(text: string | null) {
    this._native.textContent = text ?? '';
  }

  appendChild<T extends UzNode>(child: T): T {
    if (!this.window.isDisposed) {
      this._native.appendChild(child._native);
      child._setParentWrapper(this);
    }
    return child;
  }

  insertBefore<T extends UzNode>(child: T, before: UzNode | null): T {
    if (!this.window.isDisposed) {
      this._native.insertBefore(child._native, before?._native ?? null);
      child._setParentWrapper(this);
    }
    return child;
  }

  removeChild<T extends UzNode>(child: T): T {
    if (!this.window.isDisposed) {
      this._native.removeChild(child._native);
      if (child._parentWrapper === this) child._setParentWrapper(null);
    }
    return child;
  }

  /**
   * Detach this node from its parent.
   */
  remove(): void {
    if (!this.window.isDisposed) {
      this._native.remove();
      this._setParentWrapper(null);
    }
  }

  clearChildren(): void {
    if (!this.window.isDisposed) {
      this._native.clearChildren();
      for (const child of this._childWrappers) {
        child._parentWrapper = null;
      }
      this._childWrappers.clear();
    }
  }

  scrollIntoView({
    block = 'nearest',
    inline = 'nearest',
  }: { block?: ScrollAlign; inline?: ScrollAlign } = {}): void {
    if (
      !this.window.isDisposed &&
      SCROLL_ALIGN[block] != null &&
      SCROLL_ALIGN[inline] != null
    ) {
      this._native.scrollIntoView(SCROLL_ALIGN[block], SCROLL_ALIGN[inline]);
    }
  }
}

export class UzTextNode extends UzNode {
  constructor(window: Window, text: string) {
    super(window, core.createTextNode(window.id, text));
  }
}

/**
 * Resolve a node id to its JS wrapper. If the wrapper was collected but
 * Rust still owns the slab entry (because the node is connected to the
 * tree), rebuild a fresh base `UzNode` around it. Callers outside this
 * module should be event-dispatch glue that already holds a native node
 * id the `CoreNode` constructor stays an implementation detail.
 *
 * @internal
 */
export function resolveNode(
  window: Window,
  nodeId: NodeId | null,
): UzNode | null {
  if (nodeId == null) return null;
  const existing = getNode(window, nodeId);
  if (existing) return existing;
  try {
    return new UzNode(window, new CoreNode(window.id, nodeId));
  } catch {
    return null;
  }
}

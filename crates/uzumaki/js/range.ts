import { resolveNode, UzNode } from 'ext:uzumaki/node.ts';
import type { Window } from 'ext:uzumaki/window.ts';
import type { CoreRange } from 'ext:uzumaki/core.ts';

/**
 * Detached span of text. Holds a start/end endpoint pair without being tied
 * to the active selection. Pass one to `Selection.setRange` to apply it,
 * or read the current selection out as a `Range` via `Selection.getRange`.
 *
 * Endpoints are `(node, offset)` where `offset` is a byte offset within a
 * text leaf, matching how the runtime stores selection internally.
 */
export class Range {
  private readonly _window: Window;
  /** @internal Native handle. */
  readonly _core: CoreRange;

  /** @internal */
  constructor(window: Window, core: CoreRange) {
    this._window = window;
    this._core = core;
  }

  get window(): Window {
    return this._window;
  }

  get startContainer(): UzNode | null {
    return resolveNode(this._window, this._core.startContainerId);
  }

  get startOffset(): number {
    return this._core.startOffset;
  }

  get endContainer(): UzNode | null {
    return resolveNode(this._window, this._core.endContainerId);
  }

  get endOffset(): number {
    return this._core.endOffset;
  }

  get collapsed(): boolean {
    return this._core.collapsed;
  }

  get isValid(): boolean {
    return this._core.isValid;
  }

  setStart(node: UzNode, offset: number): void {
    this._assertOwnsNode(node);
    this._core.setStart(node.nodeId, offset);
  }

  setEnd(node: UzNode, offset: number): void {
    this._assertOwnsNode(node);
    this._core.setEnd(node.nodeId, offset);
  }

  /**
   * Cover all text inside `container`, from the first text leaf (offset 0)
   * to the end of the last. No-op when the container has no text
   * descendants.
   */
  selectNodeContents(container: UzNode): void {
    this._assertOwnsNode(container);
    this._core.selectNodeContents(container.nodeId);
  }

  collapse(toStart: boolean = true): void {
    this._core.collapse(toStart);
  }

  private _assertOwnsNode(node: UzNode): void {
    if (node.window !== this._window) {
      throw new Error('Range endpoint must belong to the same window');
    }
  }
}

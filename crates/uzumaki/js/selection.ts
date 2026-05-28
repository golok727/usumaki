import type { CoreSelection } from 'ext:uzumaki/core.ts';
import { resolveNode, UzNode } from 'ext:uzumaki/node.ts';
import type { Window } from 'ext:uzumaki/window.ts';
import type { UzSelectionEndpoint } from 'ext:uzumaki/events.ts';
import { Range } from 'ext:uzumaki/range.ts';

/** Shape accepted by `Selection.set`: a node plus grapheme offset. */
export interface UzSelectionEndpointInit {
  node: UzNode;
  offset: number;
}

/**
 * Live view of a window's text selection. Read endpoints through `anchor` /
 * `focus`; mutate via `collapse`, `extend`, `setBaseAndExtent`, or `empty`.
 * Programmatic mutations emit `selectionchange` on the window.
 */
export class Selection {
  private readonly _window: Window;
  private readonly _core: CoreSelection;

  /** @internal */
  constructor(window: Window, core: CoreSelection) {
    this._window = window;
    this._core = core;
  }

  get isActive(): boolean {
    return this._core.isActive;
  }

  get isCollapsed(): boolean {
    return this._core.isCollapsed;
  }

  get anchor(): UzSelectionEndpoint | null {
    const node = resolveNode(this._window, this._core.anchorNodeId);
    return node == null ? null : { node, offset: this._core.anchorOffset };
  }

  get focus(): UzSelectionEndpoint | null {
    const node = resolveNode(this._window, this._core.focusNodeId);
    return node == null ? null : { node, offset: this._core.focusOffset };
  }

  get text(): string {
    return this._core.text;
  }

  collapse(node: UzNode, offset: number): void {
    this._core.collapse(node.nodeId, offset);
    this._emit();
  }

  extend(node: UzNode, offset: number): void {
    this._core.extend(node.nodeId, offset);
    this._emit();
  }

  set(anchor: UzSelectionEndpointInit, focus?: UzSelectionEndpointInit): void {
    const f = focus ?? anchor;
    this._core.set(anchor.node.nodeId, anchor.offset, f.node.nodeId, f.offset);
    this._emit();
  }

  empty(): void {
    this._core.empty();
    this._emit();
  }

  /**
   * Replace the selection with all text inside `container`. Equivalent to
   * `setRange(window.createRange().selectNodeContents(container))`.
   */
  selectAll(container: UzNode): void {
    const range = this._window.createRange();
    range.selectNodeContents(container);
    if (range.isValid) this.setRange(range);
  }

  /**
   * Snapshot the current selection as a detached `Range`. Returns `null`
   * when there is no active selection.
   */
  getRange(): Range | null {
    const anchor = this.anchor;
    const focus = this.focus;
    if (anchor == null || focus == null) return null;
    const range = this._window.createRange();
    range.setStart(anchor.node, anchor.offset);
    range.setEnd(focus.node, focus.offset);
    return range;
  }

  /**
   * Apply a `Range` as the active selection. The range's start becomes the
   * anchor and the end becomes the focus.
   */
  setRange(range: Range): void {
    if (!range.isValid) return;
    this._core.setRange(range._core);
    this._emit();
  }

  private _emit(): void {
    this._window._dispatchLifecycle('selectionchange', {
      anchor: this.anchor,
      focus: this.focus,
      isCollapsed: this._core.isCollapsed,
    });
  }
}

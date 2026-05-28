import type { UzNode } from 'ext:uzumaki/node.ts';
import type { Window } from 'ext:uzumaki/window.ts';

/**
 * Detached span of text. Holds a start/end endpoint pair without being tied
 * to the active selection. Pass one to `Selection.setRange` to apply it,
 * or read the current selection out as a `Range` via `Selection.getRange`.
 *
 * Endpoints are `(node, offset)` where `offset` is a byte offset within a
 * text leaf, matching how the runtime stores selection internally. A range
 * is `valid` only when both endpoints are set.
 */
export class Range {
  private readonly _window: Window;
  private _startContainer: UzNode | null = null;
  private _startOffset: number = 0;
  private _endContainer: UzNode | null = null;
  private _endOffset: number = 0;

  constructor(window: Window) {
    this._window = window;
  }

  /** Window this range belongs to. Ranges cannot cross windows. */
  get window(): Window {
    return this._window;
  }

  get startContainer(): UzNode | null {
    return this._startContainer;
  }

  get startOffset(): number {
    return this._startOffset;
  }

  get endContainer(): UzNode | null {
    return this._endContainer;
  }

  get endOffset(): number {
    return this._endOffset;
  }

  /** True when start and end resolve to the same point. */
  get collapsed(): boolean {
    return (
      this._startContainer === this._endContainer &&
      this._startOffset === this._endOffset
    );
  }

  /** True once both endpoints have been set. */
  get isValid(): boolean {
    return this._startContainer != null && this._endContainer != null;
  }

  setStart(node: UzNode, offset: number): void {
    this._assertOwnsNode(node);
    this._startContainer = node;
    this._startOffset = offset;
  }

  setEnd(node: UzNode, offset: number): void {
    this._assertOwnsNode(node);
    this._endContainer = node;
    this._endOffset = offset;
  }

  /**
   * Collapse the range to a single point. Defaults to the start endpoint;
   * pass `false` to collapse to the end instead.
   */
  collapse(toStart: boolean = true): void {
    if (toStart) {
      this._endContainer = this._startContainer;
      this._endOffset = this._startOffset;
    } else {
      this._startContainer = this._endContainer;
      this._startOffset = this._endOffset;
    }
  }

  cloneRange(): Range {
    const copy = new Range(this._window);
    copy._startContainer = this._startContainer;
    copy._startOffset = this._startOffset;
    copy._endContainer = this._endContainer;
    copy._endOffset = this._endOffset;
    return copy;
  }

  private _assertOwnsNode(node: UzNode): void {
    if (node.window !== this._window) {
      throw new Error('Range endpoint must belong to the same window');
    }
  }
}

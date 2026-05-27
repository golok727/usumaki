import type { ReactNode, Ref } from 'react';

import type {
  UzMouseEvent,
  UzKeyboardEvent,
  UzInputEvent,
  UzFocusEvent,
} from 'uzumaki';
import { UzNode } from 'uzumaki';
import {
  UzButtonElement,
  UzCheckboxElement,
  UzImageElement,
  UzInputElement,
  UzTextElement,
  UzViewElement,
} from 'uzumaki';

type Overflow = 'visible' | 'hidden' | 'scroll' | 'auto';

interface ElementStyles {
  h?: number | string;
  w?: number | string;
  minH?: number | string;
  minW?: number | string;
  p?: number | string;
  px?: number | string;
  py?: number | string;
  pt?: number | string;
  pb?: number | string;
  pl?: number | string;
  pr?: number | string;
  m?: number | string;
  mx?: number | string;
  my?: number | string;
  mt?: number | string;
  mb?: number | string;
  ml?: number | string;
  mr?: number | string;
  flex?: string | number | true;
  flexDir?: 'row' | 'col' | 'column';
  flexWrap?: 'nowrap' | 'wrap' | 'wrap-reverse';
  flexGrow?: number | string;
  flexShrink?: number | string;
  items?: 'start' | 'end' | 'center' | 'stretch' | 'baseline';
  justify?: 'start' | 'end' | 'center' | 'between' | 'around' | 'evenly';
  gap?: number | string;
  bg?: string;
  color?: string;
  fontSize?: number | string;
  fontWeight?: string | number;
  fontFamily?: string;
  textAlign?: 'left' | 'center' | 'right' | 'start' | 'end' | 'justify';
  textWrap?: 'wrap' | 'nowrap' | 'anywhere' | 'break-word';
  wordBreak?: 'normal' | 'break-all' | 'keep-all';
  rounded?: number | string;
  roundedTL?: number | string;
  roundedTR?: number | string;
  roundedBR?: number | string;
  roundedBL?: number | string;
  border?: number | string;
  borderTop?: number | string;
  borderRight?: number | string;
  borderBottom?: number | string;
  borderLeft?: number | string;
  borderColor?: string;
  outline?: number | string;
  outlineColor?: string;
  outlineOffset?: number | string;
  opacity?: number | string;
  cursor?:
    | 'default'
    | 'auto'
    | 'pointer'
    | 'text'
    | 'wait'
    | 'crosshair'
    | 'move'
    | 'not-allowed'
    | 'grab'
    | 'grabbing'
    | 'help'
    | 'progress'
    | 'ew-resize'
    | 'ns-resize'
    | 'nesw-resize'
    | 'nwse-resize'
    | 'col-resize'
    | 'row-resize'
    | 'all-scroll'
    | 'zoom-in'
    | 'zoom-out';
  display?: 'flex' | 'none' | 'block';
  position?: 'relative' | 'absolute';
  top?: number | string;
  right?: number | string;
  bottom?: number | string;
  left?: number | string;
  translate?: number | [number, number] | { x?: number; y?: number };
  translateX?: number | string;
  translateY?: number | string;
  rotate?: number | string;
  scale?: number | [number, number] | { x?: number; y?: number };
  scaleX?: number | string;
  scaleY?: number | string;
  overflow?: Overflow;
  overflowX?: Overflow;
  overflowY?: Overflow;
  scrollbarWidth?: number | string;
  scrollbarColor?: string;
  scrollbarHoverColor?: string;
  scrollbarActiveColor?: string;
  scrollbarRadius?: number | string;
  // if true text inside this view can be selected
  selectable?: boolean;
  visibility?: 'visible' | 'hidden';
}

type PrefixedStyles<Prefix extends string> = {
  [K in keyof ElementStyles as `${Prefix}:${string & K}`]?: ElementStyles[K];
};

type HoverStyles = PrefixedStyles<'hover'>;
type ActiveStyles = PrefixedStyles<'active'>;
type FocusStyles = PrefixedStyles<'focus'>;

interface ElementAttributes
  extends ElementStyles, HoverStyles, ActiveStyles, FocusStyles {
  focusable?: boolean;
}

interface EventProps<T extends UzNode> {
  /** A press and release landed on this element. */
  onClick?: (ev: UzMouseEvent<T>) => void;
  /** Capture-phase {@link onClick}. */
  onClickCapture?: (ev: UzMouseEvent<T>) => void;
  /** A mouse button was pressed over this element. */
  onMouseDown?: (ev: UzMouseEvent<T>) => void;
  /** Capture-phase {@link onMouseDown}. */
  onMouseDownCapture?: (ev: UzMouseEvent<T>) => void;
  /** A mouse button was released over this element. */
  onMouseUp?: (ev: UzMouseEvent<T>) => void;
  /** Capture-phase {@link onMouseUp}. */
  onMouseUpCapture?: (ev: UzMouseEvent<T>) => void;
  /** The pointer moved over this element. */
  onMouseMove?: (ev: UzMouseEvent<T>) => void;
  /** Capture-phase {@link onMouseMove}. */
  onMouseMoveCapture?: (ev: UzMouseEvent<T>) => void;
  /** The pointer entered this element. Does not fire for descendants. */
  onMouseEnter?: (ev: UzMouseEvent<T>) => void;
  /** The pointer left this element. Does not fire for descendants. */
  onMouseLeave?: (ev: UzMouseEvent<T>) => void;
  /** The pointer entered this element or one of its descendants. */
  onMouseOver?: (ev: UzMouseEvent<T>) => void;
  /** Capture-phase {@link onMouseOver}. */
  onMouseOverCapture?: (ev: UzMouseEvent<T>) => void;
  /** The pointer left this element or one of its descendants. */
  onMouseOut?: (ev: UzMouseEvent<T>) => void;
  /** Capture-phase {@link onMouseOut}. */
  onMouseOutCapture?: (ev: UzMouseEvent<T>) => void;
  /** A key was pressed while this element was focused. */
  onKeyDown?: (ev: UzKeyboardEvent<T>) => void;
  /** Capture-phase {@link onKeyDown}. */
  onKeyDownCapture?: (ev: UzKeyboardEvent<T>) => void;
  /** A key was released while this element was focused. */
  onKeyUp?: (ev: UzKeyboardEvent<T>) => void;
  /** Capture-phase {@link onKeyUp}. */
  onKeyUpCapture?: (ev: UzKeyboardEvent<T>) => void;
}

// @oxlint-ignore
export namespace JSX {
  export type Element = ReactNode;

  export interface ElementClass {}

  export interface IntrinsicAttributes {
    key?: string | number;
  }

  export interface IntrinsicElements {
    view: ElementAttributes &
      EventProps<UzViewElement> & {
        children?: any;
        key?: string | number;
        id?: string;
        ref?: Ref<UzViewElement>;
        onFocus?: (ev: UzFocusEvent<UzViewElement>) => void;
        onBlur?: (ev: UzFocusEvent<UzViewElement>) => void;
      };
    text: ElementAttributes &
      EventProps<UzTextElement> & {
        children?: any;
        key?: string | number;
        id?: string;
        ref?: Ref<UzTextElement>;
      };
    button: ElementAttributes &
      EventProps<UzButtonElement> & {
        children?: any;
        key?: string | number;
        id?: string;
        ref?: Ref<UzButtonElement>;
        onFocus?: (ev: UzFocusEvent<UzButtonElement>) => void;
        onBlur?: (ev: UzFocusEvent<UzButtonElement>) => void;
      };
    input: ElementAttributes &
      EventProps<UzInputElement> & {
        value?: string;
        placeholder?: string;
        disabled?: boolean;
        maxLength?: number;
        multiline?: boolean;
        secure?: boolean;
        // input = "while typing"; commit = "finalized" (on blur if changed);
        // beforeinput = "before typing", preventDefault() stops the edit.
        /** Fires after the value changes while typing. */
        onInput?: (ev: UzInputEvent<UzInputElement>) => void;
        /** Fires on blur when the value changed since it was focused. */
        onCommit?: (ev: UzInputEvent<UzInputElement>) => void;
        /**
         * Fires before an edit commits. Call `ev.preventDefault()` to reject
         * the edit; `ev.data` holds the text about to be inserted.
         */
        onBeforeInput?: (ev: UzInputEvent<UzInputElement>) => void;
        onFocus?: (ev: UzFocusEvent<UzInputElement>) => void;
        onBlur?: (ev: UzFocusEvent<UzInputElement>) => void;
        onValueChange?: (value: string) => void;
        children?: any;
        key?: string | number;
        id?: string;
        ref?: Ref<UzInputElement>;
      };
    checkbox: ElementAttributes &
      EventProps<UzCheckboxElement> & {
        checked?: boolean;
        /** Fires immediately when the checkbox toggles. */
        onCommit?: (ev: UzInputEvent<UzCheckboxElement>) => void;
        onValueChange?: (value: boolean) => void;
        onInput?: (ev: UzInputEvent<UzCheckboxElement>) => void;
        onFocus?: (ev: UzFocusEvent<UzCheckboxElement>) => void;
        onBlur?: (ev: UzFocusEvent<UzCheckboxElement>) => void;
        children?: any;
        key?: string | number;
        id?: string;
        ref?: Ref<UzCheckboxElement>;
      };
    image: ElementAttributes &
      EventProps<UzImageElement> & {
        src: string;
        // todo type this better
        onLoad?: (ev: { src: string }) => void;
        onLoadStart?: (ev: { src: string }) => void;
        onError?: (ev: { src: string; message: string }) => void;
        children?: any;
        key?: string | number;
        id?: string;
        ref?: Ref<UzImageElement>;
      };
  }
}

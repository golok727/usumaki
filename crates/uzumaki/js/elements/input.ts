import {
  EventType,
  buildDomEvent,
  type UzEventMap,
  type UzInputEvent,
} from 'ext:uzumaki/events.ts';
import type { Window } from 'ext:uzumaki/window.ts';
import { UzElement } from 'ext:uzumaki/elements/base.ts';

export interface InputChangeEvent {
  readonly value: string;
}

export interface InputEventMap extends UzEventMap {
  /**
   * Fires before a pending text change is applied. Call `preventDefault()` to
   * cancel the change (e.g. for input filtering). Native may not emit this for
   * every modification source yet.
   */
  beforeinput: UzInputEvent;
  valuechange: string;
}

export type UzInputType = 'text' | 'password';

export class UzInputElement extends UzElement<InputEventMap> {
  // Value as of the last commit (focus/blur boundary); `change` fires when the
  // value diverges from this on blur.
  private _committedValue = '';

  constructor(window: Window) {
    super('input', window);

    this.on('input', () => {
      if (this._emitter._listenerCount('valuechange') > 0) {
        this._emitter.emit('valuechange', this.value);
      }
    });

    this.on('focus', () => {
      this._committedValue = this.value;
    });

    this.on('blur', () => {
      const value = this.value;
      if (value === this._committedValue) return;
      this._committedValue = value;
      this.emit(
        'commit',
        buildDomEvent(EventType.Commit, this, {
          inputType: '',
          data: value,
        }) as UzInputEvent,
      );
    });
  }

  get value(): string {
    return String(this.getAttribute('value') ?? '');
  }
  set value(value: string) {
    this.setAttribute('value', value);
  }

  get placeholder(): string {
    return String(this.getAttribute('placeholder') ?? '');
  }
  set placeholder(value: string) {
    this.setAttribute('placeholder', value);
  }

  get disabled(): boolean {
    return Boolean(this.getAttribute('disabled'));
  }
  set disabled(value: boolean) {
    this.setAttribute('disabled', value);
  }

  get multiline(): boolean {
    return Boolean(this.getAttribute('multiline'));
  }
  set multiline(value: boolean) {
    this.setAttribute('multiline', value);
  }

  get secure(): boolean {
    return Boolean(this.getAttribute('secure'));
  }

  set secure(value: boolean) {
    this.setAttribute('secure', value);
  }

  get maxLength(): number | null {
    const v = this.getAttribute('maxLength');
    return typeof v === 'number' ? v : (v == null ? null : Number(v));
  }
  set maxLength(value: number | null | undefined) {
    if (value == null) {
      this.setAttribute('maxLength', -1);
    } else {
      this.setAttribute('maxLength', value);
    }
  }

  get inputType(): UzInputType {
    return this.secure ? 'password' : 'text';
  }

  set inputType(value: UzInputType) {
    this.secure = value === 'password';
  }
}

import {
  EventType,
  buildDomEvent,
  UzEventMap,
  type UzInputEvent,
} from 'ext:uzumaki/events.ts';
import type { Window } from 'ext:uzumaki/window.ts';
import { UzElement } from 'ext:uzumaki/elements/base.ts';

export interface CheckboxEventHandlerMap extends UzEventMap {
  valuechange: boolean;
}

export class UzCheckboxElement extends UzElement<CheckboxEventHandlerMap> {
  constructor(window: Window) {
    super('checkbox', window);

    // Toggling commits immediately, so `commit` fires right after `input`.
    this.on('input', () => {
      if (this._emitter._listenerCount('valuechange') > 0) {
        this._emitter.emit('valuechange', this.checked);
      }
      this.emit(
        'commit',
        buildDomEvent(EventType.Commit, this, {
          inputType: '',
          data: null,
        }) as UzInputEvent,
      );
    });
  }

  get checked(): boolean {
    const checked = this.getAttribute('checked');
    return checked === true || checked === 'true';
  }
}

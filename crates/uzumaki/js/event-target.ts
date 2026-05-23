export interface ListenerOptions {
  capture?: boolean;
}

export interface EmitterEntry<F extends Function = Function> {
  handler: F;
  capture: boolean;
}

export interface EventTargetOptions<M extends Record<string, any>> {
  dispatch?: <K extends keyof M>(name: K, event: M[K]) => boolean | undefined;
}

export class UzEventTarget<M extends Record<string, any>> {
  private _entries: Map<keyof M, EmitterEntry[]> = new Map();
  private _dispatch?: <K extends keyof M>(
    name: K,
    event: M[K],
  ) => boolean | undefined;

  constructor(options: EventTargetOptions<M> = {}) {
    this._dispatch = options.dispatch;
  }

  on<K extends keyof M>(
    name: K,
    handler: (event: M[K]) => void,
    options?: ListenerOptions,
  ): void {
    const capture = options?.capture ?? false;
    let list = this._entries.get(name);
    if (!list) {
      list = [];
      this._entries.set(name, list);
    }
    list.push({ handler, capture });
  }

  off<K extends keyof M>(
    name: K,
    handler: (event: M[K]) => void,
    options?: ListenerOptions,
  ): void {
    const capture = options?.capture ?? false;
    const list = this._entries.get(name);
    if (!list) return;
    const idx = list.findIndex(
      (e) => e.handler === handler && e.capture === capture,
    );
    if (idx !== -1) list.splice(idx, 1);
    if (list.length === 0) this._entries.delete(name);
  }

  emit<K extends keyof M>(name: K, event: M[K]): boolean {
    const prevented = this._dispatch?.(name, event);
    if (prevented !== undefined) return prevented;
    return this._emitLocal(name, event);
  }

  /** @internal Fire local listeners without DOM-style propagation. */
  _emitLocal<K extends keyof M>(name: K, event: M[K]): boolean {
    const list = this._entries.get(name);
    if (!list || list.length === 0) return false;
    // snapshot: a handler may call off() during dispatch
    // eslint-disable-next-line unicorn/no-useless-spread
    for (const entry of [...list]) {
      try {
        entry.handler(event);
      } catch (error) {
        if (error instanceof Error) {
          console.error(error);
        } else {
          console.error('Error', error);
        }
      }
    }
    return !!(event && typeof event === 'object' && event.defaultPrevented);
  }

  /** @internal Used by the dispatcher to filter handlers by phase. */
  _listeners<K extends keyof M>(name: K): readonly EmitterEntry[] | undefined {
    return this._entries.get(name);
  }

  /** @internal */
  _hasAny(): boolean {
    return this._entries.size > 0;
  }

  /** @internal */
  _listenerCount<K extends keyof M>(name: K): number {
    return this._entries.get(name)?.length ?? 0;
  }

  /** @internal */
  _clear(): void {
    this._entries.clear();
  }
}

import core from 'ext:uzumaki/core.ts';
import type { UzEventMap } from 'ext:uzumaki/events.ts';
import type { Window } from 'ext:uzumaki/window.ts';
import { UzElement } from 'ext:uzumaki/elements/base.ts';

const WINDOWS_DRIVE_PATH = /^[A-Za-z]:[\\/]/;
const URL_SCHEME = /^[A-Za-z][A-Za-z\d+\-.]*:/;

const inflightBytes = new Map<string, Promise<Uint8Array>>();

function isFilePath(source: string): boolean {
  return (
    WINDOWS_DRIVE_PATH.test(source) ||
    source.startsWith('/') ||
    source.startsWith('./') ||
    source.startsWith('../') ||
    source.startsWith(String.raw`\\`)
  );
}

async function fetchImageBytes(source: string): Promise<Uint8Array> {
  if (isFilePath(source)) {
    return Deno.readFile(source);
  }
  if (URL_SCHEME.test(source)) {
    const url = new URL(source);
    if (url.protocol === 'file:') {
      return Deno.readFile(url);
    }
    const response = (await fetch(url)) as any; // fixme types
    if (!response.ok) {
      throw new Error(`HTTP ${response.status} while loading ${source}`);
    }
    return new Uint8Array(await response.arrayBuffer());
  }
  return Deno.readFile(source);
}

function loadImageBytes(source: string): Promise<Uint8Array> {
  let promise = inflightBytes.get(source);
  if (promise) return promise;
  promise = fetchImageBytes(source).catch((error) => {
    inflightBytes.delete(source);
    throw error;
  });
  inflightBytes.set(source, promise);
  return promise;
}

export interface ImageLoadEvent {
  readonly src: string;
}

export interface ImageErrorEvent {
  readonly src: string;
  readonly message: string;
}

export interface ImageEventMap extends UzEventMap {
  load: ImageLoadEvent;
  loadstart: ImageLoadEvent;
  error: ImageErrorEvent;
}

export class UzImageElement extends UzElement<ImageEventMap> {
  private _generation = 0;
  private _src: string | undefined;

  constructor(window: Window) {
    super('image', window);
  }

  get src(): string | undefined {
    return this._src;
  }

  set src(value: string | undefined | null) {
    const next =
      typeof value === 'string' && value.length > 0 ? value : undefined;
    if (next === this._src) return;
    this._src = next;
    this._load(next);
  }

  /** @internal */
  private _load(src: string | undefined): void {
    const generation = ++this._generation;
    core.clearImageData(this.windowId, this.nodeId);
    core.requestRedraw(this.windowId);

    if (!src) return;

    this._safeEmit('loadstart', { src });

    if (core.applyCachedImage(this.windowId, this.nodeId, src)) {
      if (!this._isCurrent(generation)) return;
      core.requestRedraw(this.windowId);
      this._safeEmit('load', { src });
      return;
    }

    void this._loadAsync(src, generation);
  }

  /** @internal */
  private async _loadAsync(src: string, generation: number): Promise<void> {
    try {
      const data = await loadImageBytes(src);
      if (!this._isCurrent(generation)) return;
      core.setEncodedImageData(this.windowId, this.nodeId, src, data);
      core.requestRedraw(this.windowId);
      this._safeEmit('load', { src });
    } catch (error) {
      if (!this._isCurrent(generation)) return;
      core.clearImageData(this.windowId, this.nodeId);
      core.requestRedraw(this.windowId);
      const message = error instanceof Error ? error.message : String(error);
      const errorEntries = this._emitter._listeners('error');
      if (errorEntries && errorEntries.length > 0) {
        this._safeEmit('error', { src, message });
      } else {
        console.error(new Error(`Failed to load image "${src}": ${message}`));
      }
    }
  }

  /** @internal */
  private _safeEmit<K extends 'load' | 'loadstart' | 'error'>(
    name: K,
    event: ImageEventMap[K],
  ): void {
    this._emitter.emit(name, event);
  }

  /** @internal */
  private _isCurrent(generation: number): boolean {
    return !this.window.isDisposed && generation === this._generation;
  }
}

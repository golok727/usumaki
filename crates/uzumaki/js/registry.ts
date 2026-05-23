import type { UzNode } from 'ext:uzumaki/node.ts';
import type { Window } from 'ext:uzumaki/window.ts';
import type { NodeId } from 'ext:uzumaki/types.ts';

const NODES = Symbol('uz.nodes');
const FINALIZER = Symbol('uz.finalizer');

type NodeMap = Map<NodeId, WeakRef<UzNode>>;

interface NodeHost {
  [NODES]?: NodeMap;
  [FINALIZER]?: FinalizationRegistry<NodeId>;
}

function nodes(window: Window, create: true): NodeMap;
function nodes(window: Window, create?: boolean): NodeMap | undefined;
function nodes(window: Window, create = false): NodeMap | undefined {
  const host = window as unknown as NodeHost;
  if (!host[NODES] && create) host[NODES] = new Map();
  return host[NODES];
}

function finalizer(window: Window, create: true): FinalizationRegistry<NodeId>;
function finalizer(
  window: Window,
  create?: boolean,
): FinalizationRegistry<NodeId> | undefined;
function finalizer(window: Window, create = false) {
  const host = window as unknown as NodeHost;
  if (!host[FINALIZER] && create) {
    const map = nodes(window, true);
    host[FINALIZER] = new FinalizationRegistry<NodeId>((nodeId) => {
      // Wrapper got collected, drop the dead WeakRef. The native node's
      // cppgc finalizer separately decides whether to free the slab entry
      // based on whether it's still connected to the tree.
      const ref = map.get(nodeId);
      if (ref && ref.deref() === undefined) map.delete(nodeId);
    });
  }
  return host[FINALIZER];
}

export function registerNode(node: UzNode): void {
  nodes(node.window, true).set(node.nodeId, new WeakRef(node));
  finalizer(node.window, true).register(node, node.nodeId);
}

export function getNode(window: Window, nodeId: NodeId): UzNode | undefined {
  const ref = nodes(window)?.get(nodeId);
  if (!ref) return undefined;
  const node = ref.deref();
  if (!node) {
    nodes(window)?.delete(nodeId);
    return undefined;
  }
  return node;
}

export function clearWindowNodes(window: Window): void {
  const host = window as unknown as NodeHost;
  delete host[NODES];
  delete host[FINALIZER];
}

export function nodeCount(window: Window): number {
  const map = nodes(window);
  if (!map) return 0;
  let count = 0;
  for (const ref of map.values()) {
    if (ref.deref() !== undefined) count++;
  }
  return count;
}

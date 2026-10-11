/** Map layout ids to registry entries, preserving order and dropping unknown ids. */
export function resolvePanels<T>(ids: string[], registry: Record<string, T>): { id: string; component: T }[] {
  return ids.filter((id) => Object.hasOwn(registry, id)).map((id) => ({ id, component: registry[id] }));
}

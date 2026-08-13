import { ref, type Ref } from 'vue';

export function useExpandable<T extends string | number = number>() {
  // Explicit Ref<Set<T>>: ref() unwrapping mangles the generic element type.
  const expanded = ref(new Set<T>()) as Ref<Set<T>>;

  function toggle(id: T) {
    if (expanded.value.has(id)) {
      expanded.value.delete(id);
    } else {
      expanded.value.add(id);
    }
    // Trigger reactivity
    expanded.value = new Set(expanded.value);
  }

  function isExpanded(id: T): boolean {
    return expanded.value.has(id);
  }

  function expand(id: T) {
    expanded.value.add(id);
    expanded.value = new Set(expanded.value);
  }

  function collapse(id: T) {
    expanded.value.delete(id);
    expanded.value = new Set(expanded.value);
  }

  function expandAll(ids: T[]) {
    for (const id of ids) {
      expanded.value.add(id);
    }
    expanded.value = new Set(expanded.value);
  }

  function collapseAll() {
    expanded.value = new Set();
  }

  return {
    expanded,
    toggle,
    isExpanded,
    expand,
    collapse,
    expandAll,
    collapseAll,
  };
}

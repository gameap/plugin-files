import { computed, ref, type Ref } from 'vue';

export interface RemoteSearchPage<T> {
  items: T[];
  /** Matched count before truncation — may exceed items.length. */
  total: number;
}

/**
 * Remote-search state for one select: debounce, a monotonic sequence guard
 * against out-of-order responses, and the fetched/total pair for the
 * "showing N of M" hint.
 */
export function useRemoteSearch<T>(
  fetcher: (q: string) => Promise<RemoteSearchPage<T>>,
  onError?: (error: unknown) => void
) {
  const items = ref<T[]>([]) as Ref<T[]>;
  const loading = ref(false);
  const total = ref(0);
  const loaded = ref(false);
  const hasMore = computed(() => total.value > items.value.length);
  let seq = 0;
  let timer: ReturnType<typeof setTimeout> | null = null;

  async function run(q: string): Promise<void> {
    const my = ++seq;
    loading.value = true;
    try {
      const page = await fetcher(q);
      if (my !== seq) return;
      items.value = page.items;
      total.value = page.total;
      loaded.value = true;
    } catch (error) {
      if (my !== seq) return;
      // The select keeps whatever it had; the failure is surfaced through
      // onError because an empty list is indistinguishable from "no matches".
      onError?.(error);
    } finally {
      if (my === seq) loading.value = false;
    }
  }

  function search(q: string, immediate = false): void {
    if (timer) clearTimeout(timer);
    if (immediate) {
      void run(q);
      return;
    }
    timer = setTimeout(() => void run(q), 400);
  }

  function stop(): void {
    if (timer) clearTimeout(timer);
    seq += 1;
  }

  return { items, loading, total, loaded, hasMore, search, stop };
}

/**
 * The selected option is merged into the list so naive-ui keeps rendering its
 * label after a new search replaces the options.
 */
export function mergeSelected<T extends { value: unknown }>(selected: T | null, options: T[]): T[] {
  if (!selected || options.some((option) => option.value === selected.value)) {
    return options;
  }
  return [selected, ...options];
}

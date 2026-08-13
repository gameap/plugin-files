import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { mergeSelected, useRemoteSearch, type RemoteSearchPage } from '@/composables/useRemoteSearch';

function page(items: string[], total = items.length): RemoteSearchPage<string> {
  return { items, total };
}

describe('useRemoteSearch', () => {
  beforeEach(() => {
    vi.useFakeTimers();
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it('debounces rapid searches into one fetch of the latest query', async () => {
    const fetcher = vi.fn().mockResolvedValue(page(['a']));
    const list = useRemoteSearch(fetcher);

    list.search('a');
    list.search('ab');
    list.search('abc');
    expect(fetcher).not.toHaveBeenCalled();

    await vi.advanceTimersByTimeAsync(400);
    expect(fetcher).toHaveBeenCalledTimes(1);
    expect(fetcher).toHaveBeenCalledWith('abc');
    expect(list.items.value).toEqual(['a']);
    expect(list.loaded.value).toBe(true);
    expect(list.loading.value).toBe(false);
  });

  it('fires immediately when asked and exposes the total', async () => {
    const fetcher = vi.fn().mockResolvedValue(page(['x'], 5));
    const list = useRemoteSearch(fetcher);

    list.search('', true);
    expect(fetcher).toHaveBeenCalledTimes(1);

    await vi.advanceTimersByTimeAsync(0);
    expect(list.items.value).toEqual(['x']);
    expect(list.total.value).toBe(5);
    expect(list.hasMore.value).toBe(true);
  });

  it('drops out-of-order responses', async () => {
    let resolveFirst!: (value: RemoteSearchPage<string>) => void;
    const first = new Promise<RemoteSearchPage<string>>((resolve) => {
      resolveFirst = resolve;
    });
    const fetcher = vi.fn().mockReturnValueOnce(first).mockResolvedValueOnce(page(['second']));
    const list = useRemoteSearch(fetcher);

    list.search('one', true);
    list.search('two', true);
    await vi.advanceTimersByTimeAsync(0);
    expect(list.items.value).toEqual(['second']);

    resolveFirst(page(['first']));
    await vi.advanceTimersByTimeAsync(0);
    expect(list.items.value).toEqual(['second']);
    expect(list.loading.value).toBe(false);
  });

  it('stop() discards both a pending response and a pending debounce', async () => {
    const fetcher = vi.fn().mockResolvedValue(page(['late']));
    const list = useRemoteSearch(fetcher);

    list.search('x', true);
    list.stop();
    await vi.advanceTimersByTimeAsync(0);
    expect(list.items.value).toEqual([]);

    list.search('y');
    list.stop();
    await vi.advanceTimersByTimeAsync(1000);
    expect(fetcher).toHaveBeenCalledTimes(1);
  });

  it('keeps previous items and reports the error on failure', async () => {
    const onError = vi.fn();
    const fetcher = vi
      .fn()
      .mockResolvedValueOnce(page(['keep']))
      .mockRejectedValueOnce(new Error('boom'));
    const list = useRemoteSearch(fetcher, onError);

    list.search('', true);
    await vi.advanceTimersByTimeAsync(0);
    list.search('fail', true);
    await vi.advanceTimersByTimeAsync(0);

    expect(list.items.value).toEqual(['keep']);
    expect(onError).toHaveBeenCalledWith(expect.any(Error));
    expect(list.loading.value).toBe(false);
  });
});

describe('mergeSelected', () => {
  it('prepends the selected option only when absent from the list', () => {
    const selected = { label: 'S', value: 1 };
    const others = [{ label: 'A', value: 2 }];
    expect(mergeSelected(selected, others)).toEqual([selected, ...others]);
    expect(mergeSelected(selected, [selected, ...others])).toEqual([selected, ...others]);
    expect(mergeSelected(null, others)).toBe(others);
  });
});

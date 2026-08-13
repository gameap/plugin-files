import { computed, type ComputedRef } from 'vue';
import { useIsAdmin } from '@gameap/plugin-sdk';

/**
 * Safe version of useIsAdmin that doesn't throw if plugin context is
 * unavailable. Defaults to true — admin-only UI it gates (NodeStatusCard) is
 * backed by admin-only API routes anyway.
 */
export function useSafeIsAdmin(): ComputedRef<boolean> {
  try {
    return useIsAdmin();
  } catch {
    return computed(() => true);
  }
}

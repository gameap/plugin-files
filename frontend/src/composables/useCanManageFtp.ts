import { computed, type ComputedRef } from 'vue';
import { usePluginContext } from '@gameap/plugin-sdk';

export const FTP_MANAGE_ABILITY = 'plugin:files:ftp-users-manage';
export const FTP_VIEW_ABILITY = 'plugin:files:ftp-users-view';

// The panel store keeps abilities as a name → bool map while the SDK types
// them as a string array; accept both (same guard as plugin-firewall).
function hasAbility(abilities: unknown, key: string): boolean {
  if (Array.isArray(abilities)) {
    return abilities.includes(key);
  }
  if (abilities && typeof abilities === 'object') {
    return Boolean((abilities as Record<string, unknown>)[key]);
  }
  return false;
}

/**
 * Whether the current user may manage FTP users: admins always, others via
 * the plugin-scoped ability. Degrades to read-only when the plugin context
 * is missing — the tab itself is gated by the view ability, so a permissive
 * fallback would leak manage UI to view-only users.
 */
export function useCanManageFtp(): ComputedRef<boolean> {
  const ctx = (() => {
    try {
      return usePluginContext();
    } catch {
      return null;
    }
  })();

  return computed(() => {
    if (!ctx) return false;
    if (ctx.user.value?.isAdmin) return true;
    return hasAbility(ctx.server.value?.abilities, FTP_MANAGE_ABILITY);
  });
}

<template>
  <div>
    <div
      class="py-3 px-4 cursor-pointer select-none hover:bg-stone-50 dark:hover:bg-stone-800"
      @click="emit('toggle')"
    >
      <div class="flex items-center gap-3">
        <GIcon
          :name="expanded ? 'chevron-down' : 'chevron-right'"
          class="text-stone-400"
        />
        <GGameIcon :game="server.game_id" size="medium" />
        <div>
          <span class="font-medium text-stone-800 dark:text-stone-200">
            {{ server.server_name }}
          </span>
          <span class="ml-2 text-sm text-stone-500 dark:text-stone-400">
            {{ server.users.length }} {{ trans('users_label') }}
          </span>
        </div>
      </div>
    </div>

    <!-- Users -->
    <!-- Inline padding: pl-14 is purged from the panel build. -->
    <div v-if="expanded && server.users.length > 0" class="pb-2" style="padding-left: 3.5rem">
      <div class="space-y-1">
        <UserRow
          v-for="user in server.users"
          :key="user.username"
          :user="user"
          :server-id="server.server_id"
          @edit="emit('editUser', { serverId: server.server_id, username: user.username })"
          @delete="emit('deleteUser', { serverId: server.server_id, username: user.username })"
        />
      </div>
    </div>

    <div v-else-if="expanded && server.users.length === 0" class="pb-3" style="padding-left: 3.5rem">
      <p class="text-sm text-stone-400 dark:text-stone-500 italic">
        {{ trans('no_users') }}
      </p>
    </div>
  </div>
</template>

<script setup lang="ts">
import { usePluginTrans } from '@gameap/plugin-sdk';
import type { GroupedServer } from '@/types';
import UserRow from './UserRow.vue';

defineProps<{
  server: GroupedServer;
  expanded: boolean;
}>();

const emit = defineEmits<{
  toggle: [];
  editUser: [{ serverId: number; username: string }];
  deleteUser: [{ serverId: number; username: string }];
}>();

const { trans } = usePluginTrans();
</script>

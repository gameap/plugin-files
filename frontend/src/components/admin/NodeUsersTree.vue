<template>
  <div class="space-y-2">
    <!-- Tree controls -->
    <div class="flex justify-end gap-2">
      <GButton color="white" size="small" @click="expandAllServers">
        {{ trans('expand_all') }}
      </GButton>
      <GButton color="white" size="small" @click="collapseAll">
        {{ trans('collapse_all') }}
      </GButton>
    </div>

    <!-- Empty state -->
    <div v-if="serverList.length === 0" class="py-6">
      <GEmpty :description="trans('no_users_found')" />
    </div>

    <!-- Tree - direct servers without node level -->
    <n-card v-else size="small" content-style="padding: 0">
      <div class="divide-y divide-stone-100 dark:divide-stone-700">
        <ServerTreeItem
          v-for="server in serverList"
          :key="server.server_id"
          :server="server"
          :expanded="isExpanded(`server-${server.server_id}`)"
          @toggle="toggle(`server-${server.server_id}`)"
          @edit-user="emit('editUser', $event)"
          @delete-user="emit('deleteUser', $event)"
        />
      </div>
    </n-card>
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue';
import { NCard } from 'naive-ui';
import { usePluginTrans } from '@gameap/plugin-sdk';
import { useExpandable } from '@/composables';
import type { GroupedServer } from '@/types';
import ServerTreeItem from './ServerTreeItem.vue';

const props = defineProps<{
  servers: Record<number, GroupedServer>;
}>();

const emit = defineEmits<{
  editUser: [{ serverId: number; username: string }];
  deleteUser: [{ serverId: number; username: string }];
}>();

const { trans } = usePluginTrans();

const { toggle, isExpanded, expandAll, collapseAll } = useExpandable<string>();

const serverList = computed(() => Object.values(props.servers));

function expandAllServers() {
  const keys = serverList.value.map((server) => `server-${server.server_id}`);
  expandAll(keys);
}
</script>

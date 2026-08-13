<template>
  <div
    class="flex items-center justify-between py-2 px-3 rounded-lg hover:bg-stone-100 dark:hover:bg-stone-800"
  >
    <div class="flex items-center gap-3">
      <div class="w-6 h-6 rounded-full bg-stone-200 dark:bg-stone-700 flex items-center justify-center">
        <GIcon name="user" size="sm" class="text-stone-500 dark:text-stone-400" />
      </div>
      <div>
        <span class="font-medium text-stone-800 dark:text-stone-200">
          {{ user.username }}
        </span>
        <span v-if="!user.enabled" class="badge-stone ml-2">
          {{ trans('disabled') }}
        </span>
      </div>
    </div>

    <div class="flex items-center gap-1" @click.stop>
      <GButton color="blue" size="small" @click="emit('edit')">
        <GIcon name="edit" />
        <span class="hidden lg:inline ml-1">{{ trans('edit') }}</span>
      </GButton>
      <GButton color="red" size="small" @click="emit('delete')">
        <GIcon name="delete" />
        <span class="hidden lg:inline ml-1">{{ trans('delete') }}</span>
      </GButton>
    </div>
  </div>
</template>

<script setup lang="ts">
import { usePluginTrans } from '@gameap/plugin-sdk';
import type { AdminUser } from '@/types';

defineProps<{
  user: AdminUser;
  serverId: number;
}>();

const emit = defineEmits<{
  edit: [];
  delete: [];
}>();

const { trans } = usePluginTrans();
</script>

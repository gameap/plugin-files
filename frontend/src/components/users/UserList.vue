<template>
  <div class="space-y-3">
    <div v-if="loading && users.length === 0" class="py-10">
      <Loading />
    </div>

    <!-- Error state -->
    <n-card v-else-if="error" size="small">
      <div class="py-8 text-center">
        <div class="text-sm text-red-500 dark:text-red-400 mb-3">{{ error }}</div>
        <GButton color="white" size="small" @click="emit('refresh')">
          <GIcon name="refresh" />
          <span class="ml-1">{{ trans('retry') }}</span>
        </GButton>
      </div>
    </n-card>

    <template v-else>
      <GEmpty v-if="users.length === 0" :description="trans('no_users')">
        <template v-if="canManage" #extra>
          <GButton color="green" size="small" @click="emit('create')">
            <GIcon name="add" />
            <span class="ml-1">{{ trans('create_user') }}</span>
          </GButton>
        </template>
      </GEmpty>

      <GDataTable
        v-else
        :columns="columns"
        :data="users"
        :loading="loading"
        :pagination="users.length > 20 ? { pageSize: 20 } : false"
      >
        <template #loading><Loading /></template>
      </GDataTable>
    </template>
  </div>
</template>

<script setup lang="ts">
import { computed, h, resolveComponent } from 'vue';
import { NCard, type DataTableColumns } from 'naive-ui';
import { usePluginTrans } from '@gameap/plugin-sdk';
import type { FtpUser } from '@/types';

const props = defineProps<{
  users: FtpUser[];
  loading: boolean;
  error: string | null;
  canManage: boolean;
}>();

const emit = defineEmits<{
  create: [];
  edit: [user: FtpUser];
  delete: [user: FtpUser];
  refresh: [];
}>();

const { trans } = usePluginTrans();

const GButton = resolveComponent('GButton');
const GIcon = resolveComponent('GIcon');
const GStatusBadge = resolveComponent('GStatusBadge');

function formatQuota(bytes: number): string {
  if (bytes === 0) return trans('unlimited');
  const units = ['B', 'KB', 'MB', 'GB', 'TB'];
  let unitIndex = 0;
  let size = bytes;
  while (size >= 1024 && unitIndex < units.length - 1) {
    size /= 1024;
    unitIndex++;
  }
  const formatted = size % 1 === 0 ? size.toString() : size.toFixed(1);
  return `${formatted} ${units[unitIndex]}`;
}

const columns = computed<DataTableColumns<FtpUser>>(() => {
  const cols: DataTableColumns<FtpUser> = [
    {
      title: trans('username'),
      key: 'username',
      render: (row) =>
        h('div', { class: 'flex flex-col min-w-0' }, [
          h('span', { class: 'font-medium text-stone-800 dark:text-stone-100' }, row.username),
          row.description
            ? h('span', { class: 'text-xs text-stone-500 dark:text-stone-400 truncate' }, row.description)
            : null,
        ]),
    },
    {
      title: trans('home_dir'),
      key: 'home_dir',
      render: (row) =>
        h(
          'span',
          { class: 'text-xs font-mono text-stone-500 dark:text-stone-400 break-all' },
          row.home_dir
        ),
    },
    {
      title: trans('status'),
      key: 'enabled',
      width: 130,
      render: (row) =>
        h(GStatusBadge, {
          status: row.enabled ? 'success' : 'canceled',
          text: row.enabled ? trans('enabled') : trans('disabled'),
        }),
    },
    {
      title: trans('quota'),
      key: 'quota_bytes',
      width: 110,
      render: (row) =>
        h('span', { class: 'text-sm text-stone-600 dark:text-stone-300 whitespace-nowrap' },
          formatQuota(row.quota_bytes)),
    },
  ];

  if (props.canManage) {
    cols.push({
      title: trans('actions'),
      key: 'actions',
      width: 190,
      render: (row) =>
        h('div', { class: 'flex gap-1 justify-end' }, [
          h(
            GButton,
            { color: 'blue', size: 'small', onClick: () => emit('edit', row) },
            () => [
              h(GIcon, { name: 'edit' }),
              h('span', { class: 'hidden lg:inline ml-1' }, trans('edit')),
            ]
          ),
          h(
            GButton,
            { color: 'red', size: 'small', onClick: () => emit('delete', row) },
            () => [
              h(GIcon, { name: 'delete' }),
              h('span', { class: 'hidden lg:inline ml-1' }, trans('delete')),
            ]
          ),
        ]),
    });
  }

  return cols;
});
</script>

<template>
  <div class="flex flex-wrap items-center gap-2">
    <!-- Search -->
    <!-- Inline width: min-w-*/w-72 utilities are purged from the panel build. -->
    <n-input
      :value="localSearch"
      :placeholder="trans('search_users')"
      size="small"
      clearable
      style="width: 220px"
      @update:value="onSearchInput"
    >
      <template #prefix>
        <GIcon name="search" size="sm" class="text-stone-400" />
      </template>
    </n-input>

    <!-- Server filter -->
    <NSelect
      :value="filters.server_id"
      :placeholder="trans('all_servers')"
      :options="serverOptions"
      :render-label="renderServerLabel"
      size="small"
      clearable
      style="width: 200px"
      @update:value="emit('update:serverId', $event)"
    />

    <!-- Status filter -->
    <NSelect
      :value="statusValue"
      :placeholder="trans('all_statuses')"
      :options="statusOptions"
      size="small"
      clearable
      style="width: 150px"
      @update:value="onStatusChange"
    />
  </div>
</template>

<script setup lang="ts">
import { computed, ref, watch, h, resolveComponent } from 'vue';
import { NInput, NSelect, type SelectOption, type SelectRenderLabel } from 'naive-ui';
import { usePluginTrans } from '@gameap/plugin-sdk';
import type { NodeUserFilters } from '@/composables/useNodeUsers';

interface ServerOption {
  label: string;
  value: number;
  gameId: string;
}

const props = defineProps<{
  filters: NodeUserFilters;
  servers: ServerOption[];
}>();

const GGameIcon = resolveComponent('GGameIcon');

const renderServerLabel: SelectRenderLabel = (option) => {
  const server = props.servers.find((s) => s.value === option.value);
  return h('div', { class: 'flex items-center gap-2' }, [
    h(GGameIcon, { game: server?.gameId, size: 'small' }),
    h('span', option.label as string),
  ]);
};

const emit = defineEmits<{
  'update:search': [value: string];
  'update:serverId': [value: number | undefined];
  'update:enabled': [value: boolean | undefined];
}>();

const { trans } = usePluginTrans();

// Local state for search input to handle debounce properly
const localSearch = ref(props.filters.search || '');

// Sync from parent when filters.search changes externally
watch(() => props.filters.search, (newValue) => {
  localSearch.value = newValue || '';
});

function onSearchInput(value: string | number | null) {
  const strValue = String(value ?? '');
  localSearch.value = strValue;
  emit('update:search', strValue);
}

const serverOptions = computed<SelectOption[]>(() =>
  props.servers.map((server) => ({ label: server.label, value: server.value }))
);

// NSelect values must be string|number; the boolean filter maps through names.
const statusValue = computed(() => {
  if (props.filters.enabled === undefined) return null;
  return props.filters.enabled ? 'enabled' : 'disabled';
});

const statusOptions = computed<SelectOption[]>(() => [
  { label: trans('enabled'), value: 'enabled' },
  { label: trans('disabled'), value: 'disabled' },
]);

function onStatusChange(value: string | number | null) {
  emit('update:enabled', value === null ? undefined : value === 'enabled');
}
</script>

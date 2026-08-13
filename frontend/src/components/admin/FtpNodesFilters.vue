<template>
  <div class="mb-5 flex flex-col gap-3 md:flex-row md:flex-wrap md:items-center md:gap-4">
    <div class="flex-1 md:max-w-md" style="min-width: 220px">
      <n-input
        v-model:value="searchInput"
        :placeholder="trans('search_nodes')"
        clearable
      >
        <template #prefix>
          <GIcon name="search" class="text-stone-400" />
        </template>
      </n-input>
    </div>

    <n-radio-group :value="statusFilter" @update:value="setStatus">
      <n-radio-button v-for="opt in statusOptions" :key="opt.value" :value="opt.value">
        {{ opt.label }}
      </n-radio-button>
    </n-radio-group>
  </div>
</template>

<script setup lang="ts">
import { computed, ref, watch } from 'vue';
import { NInput, NRadioGroup, NRadioButton } from 'naive-ui';
import { usePluginTrans } from '@gameap/plugin-sdk';

export interface NodeListFilters {
  search: string;
  status: 'all' | 'installed' | 'not_installed';
}

const props = defineProps<{
  modelValue: NodeListFilters;
}>();

const emit = defineEmits<{
  'update:modelValue': [value: NodeListFilters];
}>();

const { trans } = usePluginTrans();

const searchInput = ref(props.modelValue.search || '');
const statusFilter = ref<NodeListFilters['status']>(props.modelValue.status || 'all');

const statusOptions = computed(() => [
  { value: 'all', label: trans('all') },
  { value: 'installed', label: trans('installed_short') },
  { value: 'not_installed', label: trans('not_installed_short') },
]);

let searchTimer: ReturnType<typeof setTimeout> | null = null;
watch(searchInput, (value) => {
  if (searchTimer) clearTimeout(searchTimer);
  searchTimer = setTimeout(() => emitUpdate({ search: value }), 200);
});

function setStatus(value: NodeListFilters['status']) {
  statusFilter.value = value;
  emitUpdate({ status: value });
}

function emitUpdate(patch: Partial<NodeListFilters>) {
  emit('update:modelValue', {
    search: searchInput.value,
    status: statusFilter.value,
    ...patch,
  });
}

watch(
  () => props.modelValue,
  (value) => {
    searchInput.value = value.search || '';
    statusFilter.value = value.status || 'all';
  },
  { deep: true }
);
</script>

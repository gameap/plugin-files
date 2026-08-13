<template>
  <NSelect
    :value="value"
    filterable
    remote
    clearable
    :loading="servers.loading.value"
    :options="options"
    :render-label="renderServerLabel"
    :placeholder="trans('picker_search_servers')"
    :disabled="disabled"
    style="width: 100%"
    @search="(q: string) => servers.search(q)"
    @update:value="onSelect"
  >
    <template v-if="servers.hasMore.value" #action>
      <span class="text-sm text-stone-500 dark:text-stone-400">{{ moreLabel }}</span>
    </template>
    <template #empty>
      <span>{{ trans('picker_no_servers') }}</span>
    </template>
  </NSelect>
</template>

<script setup lang="ts">
import { computed, h, onMounted, onUnmounted, ref, resolveComponent, watch } from 'vue';
import { NSelect, type SelectRenderLabel } from 'naive-ui';
import { usePluginTrans } from '@gameap/plugin-sdk';
import { pickersApi } from '@/api';
import { showApiError } from '@/api/client';
import { mergeSelected, useRemoteSearch } from '@/composables';

// A type alias (not an interface) so it satisfies naive-ui's implicit
// index-signature check on SelectMixedOption.
type ServerOption = {
  label: string;
  value: number;
  gameId: string;
};

const props = defineProps<{
  value: number | null;
  nodeId: number;
  disabled?: boolean;
}>();

const emit = defineEmits<{
  'update:value': [value: number | null];
}>();

const { trans } = usePluginTrans();

const GGameIcon = resolveComponent('GGameIcon');

const servers = useRemoteSearch<ServerOption>(
  async (q) => {
    const { data } = await pickersApi.searchServers(q, props.nodeId);
    return {
      items: data.items.map((s) => ({ label: s.name, value: s.id, gameId: s.game_id })),
      total: data.total,
    };
  },
  (error) => showApiError(error, trans('picker_load_failed'))
);

// The selected option survives a new search replacing the fetched list.
const selected = ref<ServerOption | null>(null);
const options = computed(() => mergeSelected(selected.value, servers.items.value));

const moreLabel = computed(() =>
  trans('picker_more', {
    shown: String(servers.items.value.length),
    total: String(servers.total.value),
  })
);

const renderServerLabel: SelectRenderLabel = (option) =>
  h('div', { class: 'flex items-center gap-2' }, [
    h(GGameIcon, { game: (option as unknown as ServerOption).gameId, size: 'small' }),
    h('span', option.label as string),
  ]);

function onSelect(value: number | null) {
  selected.value = options.value.find((option) => option.value === value) ?? null;
  emit('update:value', value);
}

onMounted(() => {
  servers.search('', true);
});

watch(
  () => props.nodeId,
  () => {
    servers.search('', true);
  }
);

onUnmounted(() => {
  servers.stop();
});
</script>

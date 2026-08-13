<template>
  <div class="space-y-3">
    <div class="font-medium text-stone-800 dark:text-stone-200">
      {{ trans('virtual_paths') }}
    </div>

    <Loading v-if="loading" />

    <div v-else-if="localPaths.length === 0" class="text-center py-4 text-stone-500 dark:text-stone-400">
      {{ trans('no_virtual_paths') }}
    </div>

    <div v-else class="overflow-x-auto">
      <table class="stone-table">
        <thead class="stone-table-header">
          <tr>
            <!-- Inline widths: min-w-* utilities are purged from the panel build. -->
            <th class="px-3 py-2 font-medium" style="min-width: 10rem">{{ trans('virtual') }}</th>
            <th class="px-3 py-2 font-medium" style="min-width: 12rem">{{ trans('target') }}</th>
            <th class="px-3 py-2 font-medium text-center" style="width: 6rem">{{ trans('read_only') }}</th>
            <th class="px-3 py-2 font-medium text-center" style="width: 8rem">{{ trans('permissions') }}</th>
            <th class="px-3 py-2" style="width: 3rem"></th>
          </tr>
        </thead>
        <tbody>
          <VirtualPathRow
            v-for="(path, index) in localPaths"
            :key="index"
            :path="path"
            :disabled="disabled"
            @update="updatePath(index, $event)"
            @remove="removePath(index)"
          />
        </tbody>
      </table>
    </div>

    <div v-if="!disabled" class="flex justify-center">
      <GButton size="small" color="white" @click="addPath">
        <GIcon name="add" />
        <span class="hidden lg:inline ml-1">{{ trans('add_path') }}</span>
      </GButton>
    </div>

    <div v-if="hasChanges && !disabled" class="flex justify-end gap-2 pt-2">
      <GButton color="white" :disabled="saving" @click="reset">
        {{ trans('cancel') }}
      </GButton>
      <GButton color="green" :loading="saving" @click="save">
        <GIcon name="save" />
        <span class="hidden lg:inline ml-1">{{ trans('save') }}</span>
      </GButton>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, watch } from 'vue';
import { usePluginTrans } from '@gameap/plugin-sdk';
import type { VirtualPath } from '@/types';
import VirtualPathRow from './VirtualPathRow.vue';

const props = withDefaults(defineProps<{
  paths: VirtualPath[];
  loading?: boolean;
  saving?: boolean;
  disabled?: boolean;
}>(), {
  loading: false,
  saving: false,
  disabled: false,
});

const emit = defineEmits<{
  save: [paths: VirtualPath[]];
}>();

const { trans } = usePluginTrans();

const localPaths = ref<VirtualPath[]>([]);

watch(() => props.paths, (paths) => {
  localPaths.value = JSON.parse(JSON.stringify(paths));
}, { immediate: true, deep: true });

const hasChanges = computed(() =>
  JSON.stringify(localPaths.value) !== JSON.stringify(props.paths)
);

function addPath() {
  localPaths.value.push({
    virtual: '/shared',
    target: '',
    permissions: ['read', 'list'],
    read_only: true,
  });
}

function updatePath(index: number, path: VirtualPath) {
  localPaths.value[index] = path;
}

function removePath(index: number) {
  localPaths.value.splice(index, 1);
}

function reset() {
  localPaths.value = JSON.parse(JSON.stringify(props.paths));
}

function save() {
  emit('save', localPaths.value);
}
</script>

<template>
  <div class="space-y-3">
    <div class="font-medium text-stone-800 dark:text-stone-200">
      {{ trans('virtual_paths') }}
    </div>

    <div v-if="modelValue.length === 0" class="text-center py-4 text-stone-500 dark:text-stone-400">
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
            v-for="(path, index) in modelValue"
            :key="index"
            :path="path"
            @update="updatePath(index, $event)"
            @remove="removePath(index)"
          />
        </tbody>
      </table>
    </div>

    <div class="flex justify-center">
      <GButton type="button" size="small" color="white" @click="addPath">
        <GIcon name="add" />
        <span class="hidden lg:inline ml-1">{{ trans('add_path') }}</span>
      </GButton>
    </div>
  </div>
</template>

<script setup lang="ts">
import { usePluginTrans } from '@gameap/plugin-sdk';
import type { VirtualPath } from '@/types';
import VirtualPathRow from './VirtualPathRow.vue';

const modelValue = defineModel<VirtualPath[]>({ default: () => [] });

const { trans } = usePluginTrans();

function addPath() {
  modelValue.value = [...modelValue.value, {
    virtual: '/shared',
    target: '',
    permissions: ['read', 'list'],
    read_only: true,
  }];
}

function updatePath(index: number, path: VirtualPath) {
  const newPaths = [...modelValue.value];
  newPaths[index] = path;
  modelValue.value = newPaths;
}

function removePath(index: number) {
  modelValue.value = modelValue.value.filter((_, i) => i !== index);
}
</script>

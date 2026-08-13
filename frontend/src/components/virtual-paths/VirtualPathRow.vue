<template>
  <tr class="stone-table-row">
    <td class="px-3 py-2">
      <n-input
        :value="localPath.virtual"
        size="small"
        :placeholder="trans('virtual_placeholder')"
        :disabled="disabled"
        @update:value="onFieldInput('virtual', $event)"
      />
    </td>
    <td class="px-3 py-2">
      <n-input
        :value="localPath.target"
        size="small"
        :placeholder="trans('target_placeholder')"
        :disabled="disabled"
        @update:value="onFieldInput('target', $event)"
      />
    </td>
    <td class="px-3 py-2 text-center">
      <n-checkbox
        :checked="localPath.read_only"
        :disabled="disabled"
        @update:checked="toggleReadOnly"
      />
    </td>
    <td class="px-3 py-2 text-center">
      <div class="flex items-center justify-center gap-1 text-xs">
        <span
          v-for="perm in allPermissions"
          :key="perm"
          :class="[
            'px-1.5 py-0.5 rounded cursor-pointer select-none',
            hasPermission(perm)
              ? 'bg-primary text-white'
              : 'bg-stone-200 text-stone-500 dark:bg-stone-700 dark:text-stone-400'
          ]"
          :title="trans(perm)"
          @click="!disabled && togglePermission(perm)"
        >
          {{ perm.charAt(0).toUpperCase() }}
        </span>
      </div>
    </td>
    <td class="px-3 py-2 text-center">
      <GButton
        v-if="!disabled"
        color="red"
        size="small"
        @click="emit('remove')"
      >
        <GIcon name="trash" />
      </GButton>
    </td>
  </tr>
</template>

<script setup lang="ts">
import { reactive, watch } from 'vue';
import { NInput, NCheckbox } from 'naive-ui';
import { usePluginTrans } from '@gameap/plugin-sdk';
import type { VirtualPath, Permission } from '@/types';

const props = defineProps<{
  path: VirtualPath;
  disabled?: boolean;
}>();

const emit = defineEmits<{
  update: [path: VirtualPath];
  remove: [];
}>();

const { trans } = usePluginTrans();

const allPermissions: Permission[] = ['read', 'write', 'delete', 'list'];

const localPath = reactive<VirtualPath>({
  virtual: '',
  target: '',
  permissions: [],
  read_only: false,
});

watch(() => props.path, (path) => {
  localPath.virtual = path.virtual;
  localPath.target = path.target;
  localPath.permissions = [...path.permissions];
  localPath.read_only = path.read_only;
}, { immediate: true, deep: true });

function hasPermission(perm: Permission): boolean {
  return localPath.permissions.includes(perm);
}

function togglePermission(perm: Permission) {
  const index = localPath.permissions.indexOf(perm);
  if (index === -1) {
    localPath.permissions.push(perm);
  } else {
    localPath.permissions.splice(index, 1);
  }
  emitUpdate();
}

function toggleReadOnly(checked: boolean) {
  localPath.read_only = checked;
  emitUpdate();
}

function onFieldInput(field: 'virtual' | 'target', value: string) {
  localPath[field] = value;
  emitUpdate();
}

function emitUpdate() {
  emit('update', {
    ...localPath,
    permissions: [...localPath.permissions],
  });
}
</script>

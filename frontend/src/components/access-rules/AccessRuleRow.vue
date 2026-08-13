<template>
  <tr class="stone-table-row">
    <td class="px-3 py-2">
      <n-input
        :value="localRule.path"
        size="small"
        :placeholder="trans('path_placeholder')"
        :disabled="disabled"
        @update:value="onPathInput"
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
import { NInput } from 'naive-ui';
import { usePluginTrans } from '@gameap/plugin-sdk';
import type { AccessRule, Permission } from '@/types';

const props = defineProps<{
  rule: AccessRule;
  disabled?: boolean;
}>();

const emit = defineEmits<{
  update: [rule: AccessRule];
  remove: [];
}>();

const { trans } = usePluginTrans();

const allPermissions: Permission[] = ['read', 'write', 'delete', 'list'];

const localRule = reactive<AccessRule>({
  path: '',
  permissions: [],
});

watch(() => props.rule, (rule) => {
  localRule.path = rule.path;
  localRule.permissions = [...rule.permissions];
}, { immediate: true, deep: true });

function hasPermission(perm: Permission): boolean {
  return localRule.permissions.includes(perm);
}

function togglePermission(perm: Permission) {
  const index = localRule.permissions.indexOf(perm);
  if (index === -1) {
    localRule.permissions.push(perm);
  } else {
    localRule.permissions.splice(index, 1);
  }
  emitUpdate();
}

function onPathInput(value: string) {
  localRule.path = value;
  emitUpdate();
}

function emitUpdate() {
  emit('update', { ...localRule, permissions: [...localRule.permissions] });
}
</script>

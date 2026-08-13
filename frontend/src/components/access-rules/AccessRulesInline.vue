<template>
  <div class="space-y-3">
    <div class="font-medium text-stone-800 dark:text-stone-200">
      {{ trans('access_rules') }}
    </div>

    <div v-if="modelValue.length === 0" class="text-center py-4 text-stone-500 dark:text-stone-400">
      {{ trans('no_access_rules') }}
    </div>

    <div v-else class="overflow-x-auto">
      <table class="stone-table">
        <thead class="stone-table-header">
          <tr>
            <!-- Inline width: min-w-* utilities are purged from the panel build. -->
            <th class="px-3 py-2 font-medium" style="min-width: 12rem">{{ trans('path') }}</th>
            <th class="px-3 py-2 font-medium text-center" style="width: 8rem">{{ trans('permissions') }}</th>
            <th class="px-3 py-2" style="width: 3rem"></th>
          </tr>
        </thead>
        <tbody>
          <AccessRuleRow
            v-for="(rule, index) in modelValue"
            :key="index"
            :rule="rule"
            @update="updateRule(index, $event)"
            @remove="removeRule(index)"
          />
        </tbody>
      </table>
    </div>

    <div class="flex justify-center">
      <GButton type="button" size="small" color="white" @click="addRule">
        <GIcon name="add" />
        <span class="hidden lg:inline ml-1">{{ trans('add_rule') }}</span>
      </GButton>
    </div>
  </div>
</template>

<script setup lang="ts">
import { usePluginTrans } from '@gameap/plugin-sdk';
import type { AccessRule } from '@/types';
import AccessRuleRow from './AccessRuleRow.vue';

const modelValue = defineModel<AccessRule[]>({ default: () => [] });

const { trans } = usePluginTrans();

function addRule() {
  modelValue.value = [...modelValue.value, {
    path: '/**',
    permissions: ['read', 'list'],
  }];
}

function updateRule(index: number, rule: AccessRule) {
  const newRules = [...modelValue.value];
  newRules[index] = rule;
  modelValue.value = newRules;
}

function removeRule(index: number) {
  modelValue.value = modelValue.value.filter((_, i) => i !== index);
}
</script>

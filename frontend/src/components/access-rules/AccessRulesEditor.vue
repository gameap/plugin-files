<template>
  <div class="space-y-3">
    <div class="font-medium text-stone-800 dark:text-stone-200">
      {{ trans('access_rules') }}
    </div>

    <Loading v-if="loading" />

    <div v-else-if="localRules.length === 0" class="text-center py-4 text-stone-500 dark:text-stone-400">
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
            v-for="(rule, index) in localRules"
            :key="index"
            :rule="rule"
            :disabled="disabled"
            @update="updateRule(index, $event)"
            @remove="removeRule(index)"
          />
        </tbody>
      </table>
    </div>

    <div v-if="!disabled" class="flex justify-center">
      <GButton size="small" color="white" @click="addRule">
        <GIcon name="add" />
        <span class="hidden lg:inline ml-1">{{ trans('add_rule') }}</span>
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
import type { AccessRule } from '@/types';
import AccessRuleRow from './AccessRuleRow.vue';

const props = withDefaults(defineProps<{
  rules: AccessRule[];
  loading?: boolean;
  saving?: boolean;
  disabled?: boolean;
}>(), {
  loading: false,
  saving: false,
  disabled: false,
});

const emit = defineEmits<{
  save: [rules: AccessRule[]];
}>();

const { trans } = usePluginTrans();

const localRules = ref<AccessRule[]>([]);

watch(() => props.rules, (rules) => {
  localRules.value = JSON.parse(JSON.stringify(rules));
}, { immediate: true, deep: true });

const hasChanges = computed(() =>
  JSON.stringify(localRules.value) !== JSON.stringify(props.rules)
);

function addRule() {
  localRules.value.push({
    path: '/**',
    permissions: ['read', 'list'],
  });
}

function updateRule(index: number, rule: AccessRule) {
  localRules.value[index] = rule;
}

function removeRule(index: number) {
  localRules.value.splice(index, 1);
}

function reset() {
  localRules.value = JSON.parse(JSON.stringify(props.rules));
}

function save() {
  emit('save', localRules.value);
}
</script>

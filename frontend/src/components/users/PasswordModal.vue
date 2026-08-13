<template>
  <GModal
    v-model:show="isVisible"
    :title="trans('user_created')"
    :closable="false"
    :mask-closable="false"
    style="width: 520px; max-width: 92vw"
  >
    <div class="space-y-4">
      <n-alert type="warning" :show-icon="true">
        {{ trans('password_warning') }}
      </n-alert>

      <div>
        <div class="text-xs text-stone-500 dark:text-stone-400 mb-1">
          {{ trans('username') }}
        </div>
        <div class="p-3 bg-stone-100 dark:bg-stone-700 rounded-md font-mono text-stone-900 dark:text-white">
          {{ username }}
        </div>
      </div>

      <div>
        <div class="text-xs text-stone-500 dark:text-stone-400 mb-1">
          {{ trans('password') }}
        </div>
        <div class="flex items-center gap-2">
          <div class="flex-1 p-3 bg-stone-100 dark:bg-stone-700 rounded-md font-mono text-stone-900 dark:text-white">
            {{ password }}
          </div>
          <GButton color="white" size="small" @click="copyPassword">
            <GIcon :name="copied ? 'check' : 'copy'" />
            <span class="hidden lg:inline ml-1">{{ copied ? trans('copied') : trans('copy') }}</span>
          </GButton>
        </div>
      </div>
    </div>

    <template #footer>
      <div class="flex justify-end">
        <GButton color="green" @click="emit('update:modelValue', false)">
          <GIcon name="check" />
          <span class="hidden lg:inline ml-1">{{ trans('close') }}</span>
        </GButton>
      </div>
    </template>
  </GModal>
</template>

<script setup lang="ts">
import { computed, ref } from 'vue';
import { NAlert } from 'naive-ui';
import { usePluginTrans } from '@gameap/plugin-sdk';

const props = defineProps<{
  modelValue: boolean;
  username: string;
  password: string;
}>();

const emit = defineEmits<{
  'update:modelValue': [value: boolean];
}>();

const { trans } = usePluginTrans();

const copied = ref(false);

async function copyPassword() {
  try {
    await navigator.clipboard.writeText(props.password);
    copied.value = true;
    window.$message?.success(trans('copied'));
    setTimeout(() => {
      copied.value = false;
    }, 2000);
  } catch {
    window.$message?.error(trans('copy_failed'));
  }
}

const isVisible = computed({
  get: () => props.modelValue,
  set: (value) => emit('update:modelValue', value),
});
</script>

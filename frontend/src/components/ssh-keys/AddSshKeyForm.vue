<template>
  <div class="space-y-3 p-4 border border-stone-200 dark:border-stone-700 rounded-lg">
    <FormField :label="trans('ssh_public_key')" :hint="trans('ssh_key_hint')">
      <n-input
        v-model:value="newKey"
        type="textarea"
        :rows="4"
        :placeholder="trans('ssh_key_placeholder')"
      />
    </FormField>

    <div class="flex justify-end gap-2">
      <GButton type="button" color="white" :disabled="loading" @click="emit('cancel')">
        {{ trans('cancel') }}
      </GButton>
      <GButton
        type="button"
        color="green"
        :disabled="!isValid"
        :loading="loading"
        @click="submit"
      >
        <GIcon name="add" />
        <span class="ml-1">{{ trans('add') }}</span>
      </GButton>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed } from 'vue';
import { NInput } from 'naive-ui';
import { usePluginTrans } from '@gameap/plugin-sdk';
import FormField from '@/components/form/FormField.vue';

defineProps<{
  loading?: boolean;
}>();

const emit = defineEmits<{
  add: [key: string];
  cancel: [];
}>();

const { trans } = usePluginTrans();

const newKey = ref('');

const isValid = computed(() => {
  const key = newKey.value.trim();
  return (
    key.startsWith('ssh-') ||
    key.startsWith('ecdsa-') ||
    key.startsWith('sk-ssh-') ||
    key.startsWith('sk-ecdsa-')
  );
});

function submit() {
  if (isValid.value) {
    emit('add', newKey.value.trim());
    newKey.value = '';
  }
}
</script>

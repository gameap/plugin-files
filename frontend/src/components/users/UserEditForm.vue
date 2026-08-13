<template>
  <form class="space-y-4" @submit.prevent="handleSubmit">
    <div class="text-sm text-stone-600 dark:text-stone-400">
      <span class="font-medium">{{ trans('username') }}:</span>
      {{ user.username }}
    </div>

    <FormField :label="trans('new_password')" :hint="trans('new_password_hint')">
      <n-input
        v-model:value="form.password"
        type="password"
        show-password-on="click"
        :placeholder="trans('new_password_placeholder')"
      />
    </FormField>

    <FormField :label="trans('home_dir')">
      <n-input
        v-model:value="form.home_dir"
        :placeholder="trans('home_dir_placeholder')"
      />
    </FormField>

    <FormField
      :label="trans('quota')"
      :hint="trans('quota_hint')"
      :error="quotaInput.errorMessage.value ? trans(quotaInput.errorMessage.value) : undefined"
    >
      <n-input
        :value="quotaInput.inputValue.value"
        :placeholder="trans('quota_placeholder')"
        @update:value="quotaInput.updateInput"
      />
    </FormField>

    <n-checkbox v-model:checked="form.enabled">{{ trans('enabled') }}</n-checkbox>

    <FormField :label="trans('description')">
      <n-input
        v-model:value="form.description"
        type="textarea"
        :rows="2"
        :placeholder="trans('description_placeholder')"
      />
    </FormField>

    <div class="flex justify-end gap-2 pt-4">
      <GButton color="white" :disabled="loading" @click="emit('cancel')">
        {{ trans('cancel') }}
      </GButton>
      <GButton color="green" :loading="loading" @click="handleSubmit">
        <GIcon name="save" />
        <span class="ml-1">{{ trans('save') }}</span>
      </GButton>
    </div>
  </form>
</template>

<script setup lang="ts">
import { reactive, watch, toRef } from 'vue';
import { NInput, NCheckbox } from 'naive-ui';
import { usePluginTrans } from '@gameap/plugin-sdk';
import type { FtpUser, UpdateUserRequest } from '@/types';
import { useQuotaInput } from '@/composables';
import FormField from '@/components/form/FormField.vue';

const props = defineProps<{
  user: FtpUser;
  loading?: boolean;
}>();

const emit = defineEmits<{
  submit: [data: UpdateUserRequest];
  cancel: [];
}>();

const { trans } = usePluginTrans();

// Initialize quota input with user's current quota
const quotaInput = useQuotaInput(toRef(() => props.user.quota_bytes));

const form = reactive({
  password: '',
  home_dir: '',
  enabled: true,
  description: '',
});

// Initialize form with user data
watch(() => props.user, (user) => {
  form.password = '';
  form.home_dir = user.home_dir;
  form.enabled = user.enabled;
  form.description = user.description;
}, { immediate: true });

function handleSubmit() {
  // Validate quota format
  if (!quotaInput.isValid.value) {
    return;
  }

  const data: UpdateUserRequest = {};

  if (form.password) {
    data.password = form.password;
  }

  if (form.home_dir !== props.user.home_dir) {
    data.home_dir = form.home_dir;
  }

  if (quotaInput.bytesValue.value !== props.user.quota_bytes) {
    data.quota_bytes = quotaInput.bytesValue.value;
  }

  if (form.enabled !== props.user.enabled) {
    data.enabled = form.enabled;
  }

  if (form.description !== props.user.description) {
    data.description = form.description;
  }

  emit('submit', data);
}
</script>

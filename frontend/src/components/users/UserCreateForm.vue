<template>
  <form class="space-y-4" @submit.prevent="handleSubmit">
    <FormField :label="trans('username')" :error="errors.username" required>
      <n-input
        v-model:value="form.username"
        :placeholder="trans('username_placeholder')"
      />
    </FormField>

    <FormField :label="trans('password')" :hint="trans('password_hint')">
      <n-input
        v-model:value="form.password"
        type="password"
        show-password-on="click"
        :placeholder="trans('password_placeholder')"
      />
    </FormField>

    <FormField :label="trans('home_dir')" :hint="trans('home_dir_hint')">
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

    <hr class="border-stone-200 dark:border-stone-700" />

    <AccessRulesInline v-model="accessRules" />

    <hr class="border-stone-200 dark:border-stone-700" />

    <VirtualPathsInline v-model="virtualPaths" />

    <hr class="border-stone-200 dark:border-stone-700" />

    <SshKeysInline v-model="sshKeys" />

    <div class="flex justify-end gap-2 pt-4">
      <GButton color="white" :disabled="loading" @click="emit('cancel')">
        {{ trans('cancel') }}
      </GButton>
      <GButton color="green" :loading="loading" @click="handleSubmit">
        <GIcon name="add-square" />
        <span class="ml-1">{{ trans('create') }}</span>
      </GButton>
    </div>
  </form>
</template>

<script setup lang="ts">
import { ref, reactive } from 'vue';
import { NInput, NCheckbox } from 'naive-ui';
import { usePluginTrans } from '@gameap/plugin-sdk';
import type { CreateUserRequest, AccessRule, VirtualPath } from '@/types';
import { useQuotaInput } from '@/composables';
import FormField from '@/components/form/FormField.vue';
import AccessRulesInline from '@/components/access-rules/AccessRulesInline.vue';
import VirtualPathsInline from '@/components/virtual-paths/VirtualPathsInline.vue';
import SshKeysInline from '@/components/ssh-keys/SshKeysInline.vue';

export interface CreateUserFormData extends CreateUserRequest {
  accessRules?: AccessRule[];
  virtualPaths?: VirtualPath[];
  sshKeys?: string[];
}

defineProps<{
  loading?: boolean;
}>();

const emit = defineEmits<{
  submit: [data: CreateUserFormData];
  cancel: [];
}>();

const { trans } = usePluginTrans();

const quotaInput = useQuotaInput(0);

const form = reactive({
  username: '',
  password: '',
  home_dir: '',
  enabled: true,
  description: '',
});

const accessRules = ref<AccessRule[]>([]);
const virtualPaths = ref<VirtualPath[]>([]);
const sshKeys = ref<string[]>([]);

const errors = ref<Record<string, string>>({});

function validate(): boolean {
  errors.value = {};

  if (!form.username) {
    errors.value.username = trans('error_username_required');
    return false;
  }

  const usernameRegex = /^[a-zA-Z][a-zA-Z0-9_]{2,31}$/;
  if (!usernameRegex.test(form.username)) {
    errors.value.username = trans('error_username_format');
    return false;
  }

  if (!quotaInput.isValid.value) {
    return false;
  }

  return true;
}

function handleSubmit() {
  if (!validate()) return;

  const data: CreateUserFormData = {
    username: form.username,
    enabled: form.enabled,
    description: form.description || undefined,
  };

  if (form.password) {
    data.password = form.password;
  }

  if (form.home_dir) {
    data.home_dir = form.home_dir;
  }

  if (quotaInput.bytesValue.value > 0) {
    data.quota_bytes = quotaInput.bytesValue.value;
  }

  if (accessRules.value.length > 0) {
    data.accessRules = accessRules.value;
  }

  if (virtualPaths.value.length > 0) {
    data.virtualPaths = virtualPaths.value;
  }

  if (sshKeys.value.length > 0) {
    data.sshKeys = sshKeys.value;
  }

  emit('submit', data);
}
</script>

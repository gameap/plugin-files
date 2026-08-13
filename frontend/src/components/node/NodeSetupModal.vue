<template>
  <GModal
    v-model:show="isVisible"
    :title="modalTitle"
    style="width: 560px; max-width: 92vw"
  >
    <form class="space-y-4" @submit.prevent="handleSubmit">
      <!-- FTP Settings -->
      <div>
        <div class="font-medium mb-2 text-stone-800 dark:text-stone-200">
          {{ trans('ftp_settings') }}
        </div>

        <div class="space-y-3">
          <FormField :label="trans('ftp_address')" :hint="trans('ftp_address_hint')">
            <n-input v-model:value="config.ftp.address" />
          </FormField>

          <FormField :label="trans('ftp_port')">
            <n-input-number
              v-model:value="config.ftp.port"
              placeholder="21"
              :min="1"
              :max="65535"
              :show-button="false"
              class="w-full"
            />
          </FormField>

          <div class="grid grid-cols-2 gap-3">
            <FormField :label="trans('passive_port_min')">
              <n-input-number
                v-model:value="config.ftp.passive_port_min"
                placeholder="30000"
                :min="1"
                :max="65535"
                :show-button="false"
                class="w-full"
              />
            </FormField>
            <FormField :label="trans('passive_port_max')">
              <n-input-number
                v-model:value="config.ftp.passive_port_max"
                placeholder="30100"
                :min="1"
                :max="65535"
                :show-button="false"
                class="w-full"
              />
            </FormField>
          </div>

          <FormField :label="trans('public_host')" :hint="trans('public_host_hint')">
            <n-input v-model:value="config.ftp.public_host" />
          </FormField>

          <n-checkbox v-model:checked="config.ftp.tls_enabled">
            {{ trans('enable_tls') }}
          </n-checkbox>

          <FormField v-if="config.ftp.tls_enabled" :label="trans('tls_implicit_port')">
            <n-input-number
              v-model:value="config.ftp.tls_implicit_port"
              placeholder="990"
              :min="1"
              :max="65535"
              :show-button="false"
              class="w-full"
            />
          </FormField>
        </div>
      </div>

      <!-- SFTP Settings -->
      <div>
        <div class="font-medium mb-2 text-stone-800 dark:text-stone-200">
          {{ trans('sftp_settings') }}
        </div>

        <FormField :label="trans('sftp_port')">
          <n-input-number
            v-model:value="config.sftp.port"
            placeholder="2222"
            :min="1"
            :max="65535"
            :show-button="false"
            class="w-full"
          />
        </FormField>
      </div>
    </form>

    <template #footer>
      <div class="flex justify-end gap-2">
        <GButton color="white" :disabled="loading" @click="emit('update:modelValue', false)">
          {{ trans('cancel') }}
        </GButton>
        <GButton color="green" :loading="loading" @click="handleSubmit">
          <GIcon :name="mode === 'configure' ? 'save' : 'download'" />
          <span class="ml-1">{{ submitButtonText }}</span>
        </GButton>
      </div>
    </template>
  </GModal>
</template>

<script setup lang="ts">
import { computed, reactive, watch } from 'vue';
import { NInput, NInputNumber, NCheckbox } from 'naive-ui';
import { usePluginTrans } from '@gameap/plugin-sdk';
import FormField from '@/components/form/FormField.vue';
import type { NodeSetupConfig, FTPConfig, SFTPConfig, NodeConfigResponse } from '@/types';

const props = withDefaults(defineProps<{
  modelValue: boolean;
  loading?: boolean;
  mode?: 'install' | 'configure';
  initialConfig?: NodeConfigResponse | null;
}>(), {
  loading: false,
  mode: 'install',
  initialConfig: null,
});

const emit = defineEmits<{
  'update:modelValue': [value: boolean];
  confirm: [config: NodeSetupConfig | undefined];
}>();

const { trans } = usePluginTrans();

const modalTitle = computed(() =>
  props.mode === 'configure' ? trans('settings') : trans('configure_installation')
);

const submitButtonText = computed(() =>
  props.mode === 'configure' ? trans('save') : trans('install')
);

interface FormFTPConfig {
  address: string;
  port: number | null;
  passive_port_min: number | null;
  passive_port_max: number | null;
  public_host: string;
  tls_enabled: boolean;
  tls_implicit_port: number | null;
}

interface FormSFTPConfig {
  port: number | null;
}

const config = reactive<{ ftp: FormFTPConfig; sftp: FormSFTPConfig }>({
  ftp: {
    address: '',
    port: null,
    passive_port_min: null,
    passive_port_max: null,
    public_host: '',
    tls_enabled: false,
    tls_implicit_port: null,
  },
  sftp: {
    port: null,
  },
});

const isVisible = computed({
  get: () => props.modelValue,
  set: (value) => emit('update:modelValue', value),
});

// Initialize form when component mounts or initialConfig changes
watch(() => props.initialConfig, (initialConfig) => {
  if (props.mode === 'configure' && initialConfig?.ftp && initialConfig?.sftp) {
    // Prefill with current config in configure mode
    config.ftp.address = initialConfig.ftp.address;
    config.ftp.port = initialConfig.ftp.port;
    config.ftp.passive_port_min = initialConfig.ftp.passive_port_min;
    config.ftp.passive_port_max = initialConfig.ftp.passive_port_max;
    config.ftp.public_host = initialConfig.ftp.public_host;
    config.ftp.tls_enabled = initialConfig.ftp.tls_enabled;
    config.ftp.tls_implicit_port = initialConfig.ftp.tls_implicit_port;
    config.sftp.port = initialConfig.sftp.port;
  } else if (props.mode === 'install') {
    // Reset to empty in install mode
    config.ftp.address = '';
    config.ftp.port = null;
    config.ftp.passive_port_min = null;
    config.ftp.passive_port_max = null;
    config.ftp.public_host = '';
    config.ftp.tls_enabled = false;
    config.ftp.tls_implicit_port = null;
    config.sftp.port = null;
  }
}, { immediate: true });

function handleSubmit() {
  const result: NodeSetupConfig = {};

  // Build FTP config with only provided values
  const ftpConfig: FTPConfig = {};
  if (config.ftp.address) ftpConfig.address = config.ftp.address;
  if (config.ftp.port !== null) ftpConfig.port = config.ftp.port;
  if (config.ftp.passive_port_min !== null) ftpConfig.passive_port_min = config.ftp.passive_port_min;
  if (config.ftp.passive_port_max !== null) ftpConfig.passive_port_max = config.ftp.passive_port_max;
  if (config.ftp.public_host) ftpConfig.public_host = config.ftp.public_host;
  if (config.ftp.tls_enabled) ftpConfig.tls_enabled = true;
  if (config.ftp.tls_implicit_port !== null) ftpConfig.tls_implicit_port = config.ftp.tls_implicit_port;

  if (Object.keys(ftpConfig).length > 0) {
    result.ftp = ftpConfig;
  }

  // Build SFTP config
  const sftpConfig: SFTPConfig = {};
  if (config.sftp.port !== null) sftpConfig.port = config.sftp.port;

  if (Object.keys(sftpConfig).length > 0) {
    result.sftp = sftpConfig;
  }

  emit('confirm', Object.keys(result).length > 0 ? result : undefined);
}
</script>

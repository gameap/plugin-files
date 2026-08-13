<template>
  <div class="space-y-3">
    <div class="font-medium text-stone-800 dark:text-stone-200">
      {{ trans('ssh_keys') }}
      <span v-if="keys.length > 0" class="badge-stone ml-2">{{ keys.length }}</span>
    </div>

    <Loading v-if="loading" />

    <div
      v-else-if="keys.length === 0 && !showAddForm"
      class="text-center py-4 text-stone-500 dark:text-stone-400"
    >
      <GIcon name="key" size="xl" class="text-stone-300 dark:text-stone-600" />
      <p class="mt-2">{{ trans('no_ssh_keys') }}</p>
    </div>

    <div v-else-if="keys.length > 0" class="space-y-2">
      <SshKeyItem
        v-for="(key, index) in keys"
        :key="index"
        :ssh-key="key"
        :index="index"
        :disabled="disabled"
        @delete="handleDelete"
      />
    </div>

    <!-- Add form -->
    <AddSshKeyForm
      v-if="showAddForm"
      :loading="adding"
      @add="handleAdd"
      @cancel="showAddForm = false"
    />

    <div v-if="!disabled && !showAddForm" class="flex justify-center">
      <GButton size="small" color="white" @click="showAddForm = true">
        <GIcon name="add" />
        <span class="hidden lg:inline ml-1">{{ trans('add_key') }}</span>
      </GButton>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref } from 'vue';
import { usePluginTrans } from '@gameap/plugin-sdk';
import SshKeyItem from './SshKeyItem.vue';
import AddSshKeyForm from './AddSshKeyForm.vue';

withDefaults(defineProps<{
  keys: string[];
  loading?: boolean;
  disabled?: boolean;
}>(), {
  loading: false,
  disabled: false,
});

const emit = defineEmits<{
  add: [key: string];
  delete: [index: number];
}>();

const { trans } = usePluginTrans();

const showAddForm = ref(false);
const adding = ref(false);

async function handleAdd(key: string) {
  adding.value = true;
  try {
    emit('add', key);
    showAddForm.value = false;
  } finally {
    adding.value = false;
  }
}

function handleDelete(index: number) {
  emit('delete', index);
}
</script>

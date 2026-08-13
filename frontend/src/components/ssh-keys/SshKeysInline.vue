<template>
  <div class="space-y-3">
    <div class="font-medium text-stone-800 dark:text-stone-200">
      {{ trans('ssh_keys') }}
      <span v-if="modelValue.length > 0" class="badge-stone ml-2">{{ modelValue.length }}</span>
    </div>

    <div
      v-if="modelValue.length === 0 && !showAddForm"
      class="text-center py-4 text-stone-500 dark:text-stone-400"
    >
      <GIcon name="key" size="xl" class="text-stone-300 dark:text-stone-600" />
      <p class="mt-2">{{ trans('no_ssh_keys') }}</p>
    </div>

    <div v-else-if="modelValue.length > 0" class="space-y-2">
      <SshKeyItem
        v-for="(key, index) in modelValue"
        :key="index"
        :ssh-key="key"
        :index="index"
        @delete="handleDelete"
      />
    </div>

    <!-- Add form -->
    <AddSshKeyForm
      v-if="showAddForm"
      @add="handleAdd"
      @cancel="showAddForm = false"
    />

    <div v-if="!showAddForm" class="flex justify-center">
      <GButton type="button" size="small" color="white" @click="showAddForm = true">
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

const modelValue = defineModel<string[]>({ default: () => [] });

const { trans } = usePluginTrans();

const showAddForm = ref(false);

function handleAdd(key: string) {
  modelValue.value = [...modelValue.value, key];
  showAddForm.value = false;
}

function handleDelete(index: number) {
  modelValue.value = modelValue.value.filter((_, i) => i !== index);
}
</script>

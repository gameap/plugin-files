<template>
  <div class="flex items-center justify-between p-3 bg-stone-50 dark:bg-stone-900 rounded-lg">
    <div class="flex-1 min-w-0">
      <p class="text-sm font-mono text-stone-700 dark:text-stone-300 truncate">
        {{ truncatedKey }}
      </p>
      <p v-if="keyType" class="text-xs text-stone-500 dark:text-stone-400 mt-1">
        {{ keyType }}
      </p>
    </div>
    <GButton
      v-if="!disabled"
      class="ml-4"
      color="red"
      size="small"
      @click="emit('delete', index)"
    >
      <GIcon name="trash" />
    </GButton>
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue';

const props = defineProps<{
  sshKey: string;
  index: number;
  disabled?: boolean;
}>();

const emit = defineEmits<{
  delete: [index: number];
}>();

const truncatedKey = computed(() => {
  const key = props.sshKey;
  if (key.length <= 60) return key;
  return `${key.slice(0, 30)}...${key.slice(-20)}`;
});

const keyType = computed(() => {
  const key = props.sshKey;
  if (key.startsWith('ssh-rsa')) return 'RSA';
  if (key.startsWith('ssh-ed25519')) return 'Ed25519';
  if (key.startsWith('ecdsa-sha2')) return 'ECDSA';
  if (key.startsWith('sk-ssh-ed25519')) return 'Ed25519-SK';
  if (key.startsWith('sk-ecdsa-sha2')) return 'ECDSA-SK';
  return null;
});
</script>

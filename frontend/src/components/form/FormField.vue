<template>
  <div>
    <div v-if="label" class="text-xs text-stone-500 dark:text-stone-400 mb-1">
      {{ label }}<span v-if="required" class="text-red-500 dark:text-red-400"> *</span>
    </div>
    <slot />
    <div v-if="error" class="text-xs text-red-500 dark:text-red-400 mt-1">{{ error }}</div>
    <div v-else-if="hint" class="text-xs text-stone-400 dark:text-stone-500 mt-1">{{ hint }}</div>
  </div>
</template>

<script setup lang="ts">
defineProps<{
  label?: string;
  hint?: string;
  error?: string;
  required?: boolean;
}>();
</script>

<script setup>
defineProps({
  icon: { type: [Object, Function], default: null },
  value: { type: [String, Number], default: '-' },
  label: { type: String, required: true },
})
</script>

<template>
  <div class="bg-white dark:bg-gray-800 rounded-xl border border-gray-200 dark:border-gray-700 p-4">
    <div class="flex items-center gap-3">
      <div v-if="icon" class="p-2 rounded-lg bg-sky-50 dark:bg-sky-900/30">
        <component :is="icon" class="w-5 h-5 text-sky-600 dark:text-sky-400" />
      </div>
      <div>
        <div class="text-2xl font-bold text-gray-900 dark:text-gray-100">{{ value }}</div>
        <div class="text-xs text-gray-500 dark:text-gray-400">{{ label }}</div>
      </div>
    </div>
  </div>
</template>

<script setup>
import { Bars3Icon } from '@heroicons/vue/24/outline'
import ThemeToggle from './ThemeToggle.vue'

defineProps({ title: String })
const emit = defineEmits(['toggleSidebar'])
</script>

<template>
  <header class="h-12 flex items-center justify-between px-4 border-b bg-white dark:bg-gray-800 dark:border-gray-700 shrink-0">
    <div class="flex items-center gap-3">
      <button @click="emit('toggleSidebar')" class="p-1 text-gray-500 hover:text-gray-700 dark:text-gray-400 dark:hover:text-gray-200">
        <Bars3Icon class="w-5 h-5" />
      </button>
      <span class="text-sm font-semibold text-gray-900 dark:text-gray-100">{{ title }}</span>
    </div>
    <div class="flex items-center gap-2">
      <ThemeToggle />
    </div>
  </header>
</template>

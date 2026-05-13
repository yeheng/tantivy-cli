<script setup>
import { useTheme } from '../composables/useTheme.js'
import { SunIcon, MoonIcon } from '@heroicons/vue/24/outline'

const { theme, setTheme } = useTheme()

function toggle() {
  if (theme.value === 'dark') setTheme('light')
  else setTheme('dark')
}
</script>

<template>
  <button
    @click="toggle"
    class="p-1.5 rounded-lg text-gray-500 hover:bg-gray-100 dark:text-gray-400 dark:hover:bg-gray-700 transition-colors"
    :title="theme === 'dark' ? 'Switch to light' : 'Switch to dark'"
  >
    <SunIcon v-if="theme === 'dark'" class="w-4.5 h-4.5" />
    <MoonIcon v-else class="w-4.5 h-4.5" />
  </button>
</template>

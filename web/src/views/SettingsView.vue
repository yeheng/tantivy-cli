<script setup>
import { RadioGroup, RadioGroupLabel, RadioGroupOption } from '@headlessui/vue'
import { useTheme } from '../composables/useTheme.js'

const { theme, setTheme } = useTheme()

const themes = [
  { value: 'light', label: 'Light', desc: 'Always use light theme' },
  { value: 'dark', label: 'Dark', desc: 'Always use dark theme' },
  { value: 'system', label: 'System', desc: 'Follow your OS preference' },
]
</script>

<template>
  <div class="p-6 max-w-2xl mx-auto space-y-6">
    <div class="bg-white dark:bg-gray-800 rounded-xl border border-gray-200 dark:border-gray-700 p-5">
      <h3 class="text-sm font-semibold text-gray-900 dark:text-gray-100 mb-4">Appearance</h3>
      <RadioGroup :model-value="theme" @update:model-value="setTheme">
        <RadioGroupLabel class="text-xs font-medium text-gray-600 dark:text-gray-400 mb-2 block">Theme</RadioGroupLabel>
        <div class="space-y-2">
          <RadioGroupOption v-for="t in themes" :key="t.value" :value="t.value" v-slot="{ active, checked }">
            <div :class="[
              'flex items-center gap-3 px-4 py-3 rounded-lg border cursor-pointer transition-colors',
              checked ? 'border-sky-500 bg-sky-50 dark:bg-sky-900/20' : 'border-gray-200 dark:border-gray-600 hover:bg-gray-50 dark:hover:bg-gray-700'
            ]">
              <div :class="[
                'w-4 h-4 rounded-full border-2 flex items-center justify-center',
                checked ? 'border-sky-500' : 'border-gray-300 dark:border-gray-500'
              ]">
                <div v-if="checked" class="w-2 h-2 rounded-full bg-sky-500" />
              </div>
              <div>
                <div class="text-sm font-medium text-gray-900 dark:text-gray-100">{{ t.label }}</div>
                <div class="text-xs text-gray-500 dark:text-gray-400">{{ t.desc }}</div>
              </div>
            </div>
          </RadioGroupOption>
        </div>
      </RadioGroup>
    </div>

    <div class="bg-white dark:bg-gray-800 rounded-xl border border-gray-200 dark:border-gray-700 p-5">
      <h3 class="text-sm font-semibold text-gray-900 dark:text-gray-100 mb-3">Server</h3>
      <dl class="space-y-2 text-sm">
        <div class="flex justify-between">
          <dt class="text-gray-500 dark:text-gray-400">URL</dt>
          <dd class="text-gray-900 dark:text-gray-100 font-mono">{{ window.location.origin }}</dd>
        </div>
        <div class="flex justify-between">
          <dt class="text-gray-500 dark:text-gray-400">Version</dt>
          <dd class="text-gray-900 dark:text-gray-100">0.1.0</dd>
        </div>
      </dl>
    </div>
  </div>
</template>

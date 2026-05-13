<script setup>
import { Disclosure, DisclosureButton, DisclosurePanel } from '@headlessui/vue'
import { ChevronDownIcon } from '@heroicons/vue/24/outline'

defineProps({
  hit: { type: Object, required: true },
  index: { type: Number, required: true },
  rank: { type: Number, default: 0 },
})
</script>

<template>
  <div class="border border-gray-100 dark:border-gray-700 rounded-lg p-3 bg-gray-50/50 dark:bg-gray-800/50">
    <div class="flex items-center justify-between mb-2">
      <span class="text-xs font-semibold text-gray-500 dark:text-gray-400">#{{ rank }}</span>
      <span v-if="hit.score != null" class="text-xs text-gray-400 dark:text-gray-500">score {{ hit.score.toFixed(4) }}</span>
    </div>
    <div v-if="hit.doc" class="space-y-1">
      <div v-for="(value, key) in hit.doc" :key="key" class="text-sm">
        <span class="font-medium text-gray-600 dark:text-gray-400">{{ key }}:</span>
        <span class="text-gray-800 dark:text-gray-200 ml-1">{{ value }}</span>
      </div>
    </div>
    <div v-if="hit.snippets" class="mt-2 space-y-0.5">
      <div v-for="(text, key) in hit.snippets" :key="key" class="text-xs">
        <span class="font-medium text-gray-600 dark:text-gray-400">{{ key }}:</span>
        <span class="text-gray-700 dark:text-gray-300" v-html="Array.isArray(text) ? text.join(' ... ') : text" />
      </div>
    </div>
    <Disclosure class="mt-2" v-slot="{ open }">
      <DisclosureButton class="flex items-center gap-1 text-xs text-gray-400 dark:text-gray-500 hover:text-gray-600 dark:hover:text-gray-300">
        <ChevronDownIcon :class="['w-3 h-3 transition-transform', open && 'rotate-180']" />
        Raw JSON
      </DisclosureButton>
      <DisclosurePanel class="mt-1">
        <pre class="text-xs text-gray-600 dark:text-gray-400 overflow-x-auto bg-white dark:bg-gray-900 rounded p-2">{{ JSON.stringify(hit.doc, null, 2) }}</pre>
      </DisclosurePanel>
    </Disclosure>
  </div>
</template>

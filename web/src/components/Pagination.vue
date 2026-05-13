<script setup>
import { computed } from 'vue'

const props = defineProps({
  offset: { type: Number, default: 0 },
  limit: { type: Number, default: 10 },
  total: { type: Number, default: 0 },
})
const emit = defineEmits(['update:offset'])

const page = computed(() => Math.floor(props.offset / props.limit) + 1)
const totalPages = computed(() => Math.max(1, Math.ceil(props.total / props.limit)))
const showing = computed(() => `${props.offset + 1}-${Math.min(props.offset + props.limit, props.total)} of ${props.total}`)

function prev() { if (props.offset > 0) emit('update:offset', Math.max(0, props.offset - props.limit)) }
function next() { if (page.value < totalPages.value) emit('update:offset', props.offset + props.limit) }
</script>

<template>
  <div class="flex items-center justify-between py-3 text-sm text-gray-600 dark:text-gray-400">
    <span v-if="total > 0">Showing {{ showing }}</span>
    <span v-else>No results</span>
    <div class="flex items-center gap-2">
      <button @click="prev" :disabled="offset === 0"
        class="px-3 py-1 rounded-lg border border-gray-200 dark:border-gray-600 hover:bg-gray-100 dark:hover:bg-gray-700 disabled:opacity-50 transition-colors">Prev</button>
      <span>{{ page }} / {{ totalPages }}</span>
      <button @click="next" :disabled="page >= totalPages"
        class="px-3 py-1 rounded-lg border border-gray-200 dark:border-gray-600 hover:bg-gray-100 dark:hover:bg-gray-700 disabled:opacity-50 transition-colors">Next</button>
    </div>
  </div>
</template>

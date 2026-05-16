<script setup>
import { FwbModal } from 'flowbite-vue'

defineProps({
  show: Boolean,
  doc: { type: Object, default: null },
})
const emit = defineEmits(['update:show'])
</script>

<template>
  <FwbModal
    v-if="show"
    size="lg"
    @close="emit('update:show', false)"
    @click:outside="emit('update:show', false)"
  >
    <template #header>
      <h3 class="text-lg font-semibold text-gray-900 dark:text-gray-100">Document Detail</h3>
    </template>
    <template #body>
      <pre v-if="doc" class="text-xs text-gray-700 dark:text-gray-300 whitespace-pre-wrap">{{ JSON.stringify(doc, null, 2) }}</pre>
    </template>
  </FwbModal>
</template>

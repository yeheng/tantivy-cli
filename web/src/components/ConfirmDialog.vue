<script setup>
import { FwbModal } from 'flowbite-vue'

defineProps({
  show: Boolean,
  title: { type: String, default: 'Confirm' },
  message: { type: String, required: true },
  confirmLabel: { type: String, default: 'Confirm' },
  danger: Boolean,
  loading: Boolean,
})
const emit = defineEmits(['update:show', 'confirm'])
</script>

<template>
  <FwbModal
    v-if="show"
    size="md"
    @close="emit('update:show', false)"
    @click:outside="emit('update:show', false)"
  >
    <template #header>
      <h3 class="text-lg font-semibold text-gray-900 dark:text-gray-100">{{ title }}</h3>
    </template>
    <template #body>
      <p class="text-sm text-gray-600 dark:text-gray-400">{{ message }}</p>
    </template>
    <template #footer>
      <div class="flex justify-end gap-2">
        <button
          @click="emit('update:show', false)"
          class="px-4 py-2 text-sm font-medium text-gray-600 dark:text-gray-300 hover:bg-gray-100 dark:hover:bg-gray-700 rounded-lg transition-colors"
        >Cancel</button>
        <button
          @click="emit('confirm')"
          :disabled="loading"
          :class="[
            'px-4 py-2 text-sm font-medium text-white rounded-lg transition-colors disabled:opacity-50',
            danger ? 'bg-red-600 hover:bg-red-700' : 'bg-sky-600 hover:bg-sky-700'
          ]"
        >{{ loading ? '...' : confirmLabel }}</button>
      </div>
    </template>
  </FwbModal>
</template>

<script setup>
import { Dialog, DialogPanel, DialogTitle, TransitionChild, TransitionRoot } from '@headlessui/vue'

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
  <TransitionRoot appear :show="show" as="template">
    <Dialog as="div" @close="emit('update:show', false)" class="relative z-50">
      <TransitionChild
        enter="duration-200 ease-out" enter-from="opacity-0" enter-to="opacity-100"
        leave="duration-150 ease-in" leave-from="opacity-100" leave-to="opacity-0"
      >
        <div class="fixed inset-0 bg-black/30" />
      </TransitionChild>
      <div class="fixed inset-0 flex items-center justify-center p-4">
        <TransitionChild
          enter="duration-200 ease-out" enter-from="opacity-0 scale-95" enter-to="opacity-100 scale-100"
          leave="duration-150 ease-in" leave-from="opacity-100 scale-100" leave-to="opacity-0 scale-95"
        >
          <DialogPanel class="w-full max-w-md bg-white dark:bg-gray-800 rounded-xl shadow-xl p-6">
            <DialogTitle class="text-lg font-semibold text-gray-900 dark:text-gray-100">{{ title }}</DialogTitle>
            <p class="mt-2 text-sm text-gray-600 dark:text-gray-400">{{ message }}</p>
            <div class="mt-4 flex justify-end gap-2">
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
          </DialogPanel>
        </TransitionChild>
      </div>
    </Dialog>
  </TransitionRoot>
</template>

<script setup>
import { Dialog, DialogPanel, TransitionChild, TransitionRoot } from '@headlessui/vue'
import { XMarkIcon } from '@heroicons/vue/24/outline'

defineProps({
  show: Boolean,
  doc: { type: Object, default: null },
})
const emit = defineEmits(['update:show'])
</script>

<template>
  <TransitionRoot :show="show" as="template">
    <Dialog class="relative z-50" @close="emit('update:show', false)">
      <TransitionChild
        enter="duration-200 ease-out" enter-from="opacity-0" enter-to="opacity-100"
        leave="duration-150 ease-in" leave-from="opacity-100" leave-to="opacity-0"
      >
        <div class="fixed inset-0 bg-black/20" />
      </TransitionChild>
      <div class="fixed inset-0 flex justify-end">
        <TransitionChild
          enter="duration-200 ease-out" enter-from="translate-x-full" enter-to="translate-x-0"
          leave="duration-150 ease-in" leave-from="translate-x-0" leave-to="translate-x-full"
        >
          <DialogPanel class="w-full max-w-lg bg-white dark:bg-gray-800 flex flex-col h-full">
            <div class="flex items-center justify-between px-4 py-3 border-b dark:border-gray-700">
              <span class="text-sm font-semibold text-gray-900 dark:text-gray-100">Document Detail</span>
              <button @click="emit('update:show', false)" class="p-1 text-gray-400 hover:text-gray-600 dark:hover:text-gray-200">
                <XMarkIcon class="w-5 h-5" />
              </button>
            </div>
            <div v-if="doc" class="flex-1 overflow-auto p-4">
              <pre class="text-xs text-gray-700 dark:text-gray-300 whitespace-pre-wrap">{{ JSON.stringify(doc, null, 2) }}</pre>
            </div>
          </DialogPanel>
        </TransitionChild>
      </div>
    </Dialog>
  </TransitionRoot>
</template>

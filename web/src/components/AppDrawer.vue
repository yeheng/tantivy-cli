<script setup>
defineProps({
  show: Boolean,
  title: { type: String, default: '' },
  position: { type: String, default: 'right' },
})
const emit = defineEmits(['update:show'])
</script>

<template>
  <Teleport to="body">
    <Transition
      enter-active-class="transition duration-200 ease-out"
      enter-from-class="opacity-0"
      enter-to-class="opacity-100"
      leave-active-class="transition duration-150 ease-in"
      leave-from-class="opacity-100"
      leave-to-class="opacity-0"
    >
      <div v-if="show" class="fixed inset-0 z-50 flex" :class="position === 'right' ? 'justify-end' : 'justify-start'">
        <div class="fixed inset-0 bg-black/20" @click="emit('update:show', false)" />
        <div class="relative w-full max-w-lg bg-white dark:bg-gray-800 flex flex-col h-full shadow-xl">
          <div v-if="title" class="flex items-center justify-between px-4 py-3 border-b dark:border-gray-700">
            <span class="text-sm font-semibold text-gray-900 dark:text-gray-100">{{ title }}</span>
            <button
              type="button"
              @click="emit('update:show', false)"
              class="p-1 text-gray-400 hover:text-gray-600 dark:hover:text-gray-200"
            >
              <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
              </svg>
            </button>
          </div>
          <slot />
        </div>
      </div>
    </Transition>
  </Teleport>
</template>

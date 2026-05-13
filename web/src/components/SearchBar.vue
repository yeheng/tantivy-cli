<script setup>
const props = defineProps({
  modelValue: { type: String, default: '' },
  limit: { type: Number, default: 10 },
  loading: Boolean,
  placeholder: { type: String, default: 'Search...' },
})
const emit = defineEmits(['update:modelValue', 'update:limit', 'search'])
</script>

<template>
  <div class="flex gap-3">
    <input
      :value="modelValue"
      @input="emit('update:modelValue', $event.target.value)"
      @keydown.enter="emit('search')"
      :placeholder="placeholder"
      class="flex-1 border border-gray-300 dark:border-gray-600 rounded-lg px-3 py-2 text-sm bg-white dark:bg-gray-800 text-gray-900 dark:text-gray-100 focus:outline-none focus:ring-2 focus:ring-sky-500"
    />
    <select
      :value="limit"
      @change="emit('update:limit', Number($event.target.value))"
      class="w-20 border border-gray-300 dark:border-gray-600 rounded-lg px-2 py-2 text-sm bg-white dark:bg-gray-800 text-gray-900 dark:text-gray-100"
    >
      <option :value="10">10</option>
      <option :value="20">20</option>
      <option :value="50">50</option>
      <option :value="100">100</option>
    </select>
    <button
      @click="emit('search')"
      :disabled="loading"
      class="px-4 py-2 bg-sky-600 hover:bg-sky-700 disabled:opacity-50 text-white text-sm font-medium rounded-lg transition-colors"
    >{{ loading ? '...' : 'Search' }}</button>
  </div>
</template>

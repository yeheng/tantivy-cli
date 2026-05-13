<script setup>
defineProps({
  docs: { type: Array, default: () => [] },
  columns: { type: Array, default: () => [] },
  loading: Boolean,
})
const emit = defineEmits(['rowClick'])
</script>

<template>
  <div v-if="loading" class="text-center py-6 text-gray-400 text-sm">Loading documents...</div>
  <div v-else-if="!docs.length" class="text-center py-6 text-gray-400 dark:text-gray-500 text-sm">No documents found.</div>
  <div v-else class="overflow-x-auto">
    <table class="w-full">
      <thead>
        <tr class="text-left text-xs font-medium text-gray-500 dark:text-gray-400 uppercase tracking-wider border-b dark:border-gray-700">
          <th class="pb-2 px-3">#</th>
          <th v-for="col in columns" :key="col" class="pb-2 px-3">{{ col }}</th>
        </tr>
      </thead>
      <tbody class="divide-y dark:divide-gray-700">
        <tr v-for="(doc, i) in docs" :key="i"
            class="hover:bg-gray-50 dark:hover:bg-gray-800/50 cursor-pointer transition-colors"
            @click="emit('rowClick', doc)">
          <td class="py-2 px-3 text-xs text-gray-400">{{ i + 1 }}</td>
          <td v-for="col in columns" :key="col" class="py-2 px-3 text-sm text-gray-800 dark:text-gray-200 truncate max-w-xs">
            {{ typeof doc[col] === 'object' ? JSON.stringify(doc[col]) : doc[col] }}
          </td>
        </tr>
      </tbody>
    </table>
  </div>
</template>

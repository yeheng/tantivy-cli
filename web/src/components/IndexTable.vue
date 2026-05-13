<script setup>
import { useRouter } from 'vue-router'
import { TrashIcon, MagnifyingGlassIcon } from '@heroicons/vue/24/outline'

const props = defineProps({
  indexes: { type: Array, default: () => [] },
  loading: Boolean,
})
const emit = defineEmits(['delete'])
const router = useRouter()

function statusDot(status) {
  if (status === 'dirty') return 'text-yellow-500'
  if (status && status !== 'ready') return 'text-gray-400'
  return 'text-emerald-500'
}
</script>

<template>
  <div v-if="loading" class="text-center py-8 text-gray-400 text-sm">Loading indexes...</div>
  <div v-else-if="!indexes.length" class="text-center py-8 text-gray-400 dark:text-gray-500 text-sm">No indexes yet. Create one to get started.</div>
  <table v-else class="w-full">
    <thead>
      <tr class="text-left text-xs font-medium text-gray-500 dark:text-gray-400 uppercase tracking-wider border-b dark:border-gray-700">
        <th class="pb-2 px-3">Name</th>
        <th class="pb-2 px-3">Documents</th>
        <th class="pb-2 px-3">Segments</th>
        <th class="pb-2 px-3">Status</th>
        <th class="pb-2 px-3 w-20"></th>
      </tr>
    </thead>
    <tbody class="divide-y dark:divide-gray-700">
      <tr v-for="idx in indexes" :key="idx.name"
          class="group hover:bg-gray-50 dark:hover:bg-gray-800/50 cursor-pointer transition-colors"
          @click="router.push(`/index/${idx.name}`)">
        <td class="py-3 px-3 text-sm font-medium text-gray-900 dark:text-gray-100">{{ idx.name }}</td>
        <td class="py-3 px-3 text-sm text-gray-600 dark:text-gray-400">{{ idx.num_docs?.toLocaleString() ?? '-' }}</td>
        <td class="py-3 px-3 text-sm text-gray-600 dark:text-gray-400">{{ idx.num_segments ?? '-' }}</td>
        <td class="py-3 px-3">
          <span class="inline-flex items-center gap-1.5 text-sm">
            <span :class="[statusDot(idx.status), 'w-2 h-2 rounded-full bg-current']" />
            <span class="text-gray-600 dark:text-gray-400 capitalize">{{ idx.status || 'active' }}</span>
          </span>
        </td>
        <td class="py-3 px-3">
          <div class="flex items-center gap-1 opacity-0 group-hover:opacity-100 transition-opacity">
            <button @click.stop="router.push(`/index/${idx.name}`)"
              class="p-1 text-gray-400 hover:text-sky-600 dark:hover:text-sky-400">
              <MagnifyingGlassIcon class="w-4 h-4" />
            </button>
            <button @click.stop="emit('delete', idx.name)"
              class="p-1 text-gray-400 hover:text-red-600 dark:hover:text-red-400">
              <TrashIcon class="w-4 h-4" />
            </button>
          </div>
        </td>
      </tr>
    </tbody>
  </table>
</template>

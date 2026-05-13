<script setup>
import { ref, onMounted } from 'vue'
import { useRouter } from 'vue-router'
import { PlusIcon } from '@heroicons/vue/24/outline'
import IndexTable from '../components/IndexTable.vue'
import ConfirmDialog from '../components/ConfirmDialog.vue'
import { listIndexes, getIndexStats, createIndex, deleteIndex } from '../api.js'

const router = useRouter()
const indexes = ref([])
const loading = ref(true)
const error = ref('')

const showDelete = ref(false)
const deleteName = ref('')
const deleteLoading = ref(false)

async function load() {
  loading.value = true
  error.value = ''
  try {
    const names = await listIndexes()
    const results = await Promise.allSettled(
      names.map(async (name) => {
        const stats = await getIndexStats(name)
        return { name, ...stats }
      })
    )
    indexes.value = results.filter((r) => r.status === 'fulfilled').map((r) => r.value)
  } catch (e) {
    error.value = e.message
  } finally {
    loading.value = false
  }
}

function askDelete(name) {
  deleteName.value = name
  showDelete.value = true
}

async function onConfirmDelete() {
  deleteLoading.value = true
  try {
    await deleteIndex(deleteName.value)
    showDelete.value = false
    await load()
  } catch (e) {
    alert(e.message)
  } finally {
    deleteLoading.value = false
  }
}

onMounted(load)
</script>

<template>
  <div class="p-6 max-w-4xl mx-auto space-y-6">
    <div class="flex items-center justify-between">
      <h2 class="text-lg font-semibold text-gray-900 dark:text-gray-100">All Indexes</h2>
    </div>
    <div class="bg-white dark:bg-gray-800 rounded-xl border border-gray-200 dark:border-gray-700 p-5">
      <p v-if="error" class="text-sm text-red-600 dark:text-red-400 mb-3">{{ error }}</p>
      <IndexTable :indexes="indexes" :loading="loading" @delete="askDelete" />
    </div>

    <ConfirmDialog v-model:show="showDelete" title="Delete Index"
      :message="`Permanently delete index &quot;${deleteName}&quot;?`" confirm-label="Delete" :danger="true" :loading="deleteLoading"
      @confirm="onConfirmDelete" />
  </div>
</template>

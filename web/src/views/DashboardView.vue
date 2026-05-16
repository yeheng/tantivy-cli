<script setup>
import { ref, computed } from 'vue'
import { useRouter } from 'vue-router'
import { FwbModal } from 'flowbite-vue'
import { PlusIcon, ArrowPathIcon, PauseIcon, PlayIcon, DocumentTextIcon, PuzzlePieceIcon, CubeIcon, CheckCircleIcon } from '@heroicons/vue/24/outline'
import StatCard from '../components/StatCard.vue'
import IndexTable from '../components/IndexTable.vue'
import ConfirmDialog from '../components/ConfirmDialog.vue'
import { useAutoRefresh } from '../composables/useAutoRefresh.js'
import { listIndexes, getIndexStats, createIndex, deleteIndex } from '../api.js'

const router = useRouter()
const indexes = ref([])
const loading = ref(true)
const error = ref('')

const showCreate = ref(false)
const newName = ref('')
const newSchema = ref(JSON.stringify({
  fields: [
    { name: 'title', kind: 'Text', stored: true, indexed: true, fast: false },
    { name: 'body', kind: 'Text', stored: true, indexed: true, fast: false }
  ]
}, null, 2))
const createError = ref('')
const createLoading = ref(false)

const showDelete = ref(false)
const deleteName = ref('')
const deleteLoading = ref(false)

async function loadDashboard() {
  try {
    const names = await listIndexes()
    const results = await Promise.allSettled(
      names.map(async (name) => {
        const stats = await getIndexStats(name)
        return { name, ...stats }
      })
    )
    indexes.value = results
      .filter((r) => r.status === 'fulfilled')
      .map((r) => r.value)
  } catch (e) {
    error.value = e.message
  } finally {
    loading.value = false
  }
}

const { lastUpdated, isPaused, pause, resume } = useAutoRefresh(loadDashboard, 30000)

const totalDocs = computed(() => indexes.value.reduce((s, i) => s + (i.num_docs || 0), 0))
const totalSegments = computed(() => indexes.value.reduce((s, i) => s + (i.num_segments || 0), 0))
const allHealthy = computed(() => indexes.value.every((i) => !i.status || i.status === 'ready'))

async function onCreate() {
  createError.value = ''
  const name = newName.value.trim()
  if (!name) { createError.value = 'Name is required'; return }
  createLoading.value = true
  try {
    const schema = JSON.parse(newSchema.value)
    await createIndex(name, schema)
    showCreate.value = false
    newName.value = ''
    await loadDashboard()
    router.push(`/index/${name}`)
  } catch (e) {
    createError.value = e.message
  } finally {
    createLoading.value = false
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
    await loadDashboard()
  } catch (e) {
    alert(e.message)
  } finally {
    deleteLoading.value = false
  }
}
</script>

<template>
  <div class="p-6 max-w-6xl mx-auto space-y-6">
    <!-- Summary cards -->
    <div class="grid grid-cols-2 md:grid-cols-4 gap-4">
      <StatCard :icon="CubeIcon" :value="indexes.length" label="Indexes" />
      <StatCard :icon="DocumentTextIcon" :value="totalDocs.toLocaleString()" label="Documents" />
      <StatCard :icon="PuzzlePieceIcon" :value="totalSegments" label="Segments" />
      <StatCard :icon="CheckCircleIcon" :value="allHealthy ? 'Healthy' : 'Attention'" label="Status" />
    </div>

    <!-- Index table -->
    <div class="bg-white dark:bg-gray-800 rounded-xl border border-gray-200 dark:border-gray-700 p-5">
      <div class="flex items-center justify-between mb-4">
        <h2 class="text-base font-semibold text-gray-900 dark:text-gray-100">Indexes</h2>
        <div class="flex items-center gap-2">
          <button @click="isPaused ? resume() : pause()"
            class="inline-flex items-center gap-1 px-2.5 py-1.5 text-xs font-medium text-gray-600 dark:text-gray-400 hover:bg-gray-100 dark:hover:bg-gray-700 rounded-md transition-colors">
            <component :is="isPaused ? PlayIcon : PauseIcon" class="w-3.5 h-3.5" />
            {{ isPaused ? 'Resume' : 'Pause' }}
          </button>
          <button @click="loadDashboard"
            class="inline-flex items-center gap-1 px-2.5 py-1.5 text-xs font-medium text-gray-600 dark:text-gray-400 hover:bg-gray-100 dark:hover:bg-gray-700 rounded-md transition-colors">
            <ArrowPathIcon class="w-3.5 h-3.5" /> Refresh
          </button>
          <button @click="showCreate = true"
            class="inline-flex items-center gap-1 bg-sky-600 hover:bg-sky-700 text-white text-xs font-medium px-3 py-1.5 rounded-md transition-colors">
            <PlusIcon class="w-3.5 h-3.5" /> New Index
          </button>
        </div>
      </div>
      <p v-if="error" class="text-sm text-red-600 dark:text-red-400 mb-3">{{ error }}</p>
      <IndexTable :indexes="indexes" :loading="loading" @delete="askDelete" />
      <p v-if="lastUpdated" class="mt-3 text-xs text-gray-400 dark:text-gray-500">
        Last updated: {{ lastUpdated.toLocaleTimeString() }}
      </p>
    </div>

    <!-- Create dialog -->
    <FwbModal
      v-if="showCreate"
      size="2xl"
      @close="showCreate = false"
      @click:outside="showCreate = false"
    >
      <template #header>
        <h3 class="text-lg font-semibold text-gray-900 dark:text-gray-100">Create Index</h3>
      </template>
      <template #body>
        <div class="space-y-4">
          <div>
            <label class="block text-xs font-medium text-gray-600 dark:text-gray-400 mb-1">Name</label>
            <input v-model="newName"
              class="w-full border border-gray-300 dark:border-gray-600 rounded-lg px-3 py-2 text-sm bg-white dark:bg-gray-700 text-gray-900 dark:text-gray-100 focus:outline-none focus:ring-2 focus:ring-sky-500"
              placeholder="articles" />
          </div>
          <div>
            <label class="block text-xs font-medium text-gray-600 dark:text-gray-400 mb-1">Schema JSON</label>
            <textarea v-model="newSchema" rows="8"
              class="w-full border border-gray-300 dark:border-gray-600 rounded-lg px-3 py-2 text-xs font-mono bg-white dark:bg-gray-700 text-gray-900 dark:text-gray-100 focus:outline-none focus:ring-2 focus:ring-sky-500" />
          </div>
          <p v-if="createError" class="text-sm text-red-600 dark:text-red-400">{{ createError }}</p>
        </div>
      </template>
      <template #footer>
        <div class="flex justify-end gap-2">
          <button @click="showCreate = false"
            class="px-4 py-2 text-sm font-medium text-gray-600 dark:text-gray-300 hover:bg-gray-100 dark:hover:bg-gray-700 rounded-lg transition-colors">Cancel</button>
          <button @click="onCreate" :disabled="createLoading"
            class="px-4 py-2 text-sm font-medium text-white bg-sky-600 hover:bg-sky-700 disabled:opacity-50 rounded-lg transition-colors">
            {{ createLoading ? 'Creating...' : 'Create' }}
          </button>
        </div>
      </template>
    </FwbModal>

    <!-- Delete dialog -->
    <ConfirmDialog
      v-model:show="showDelete"
      title="Delete Index"
      :message="`Permanently delete index &quot;${deleteName}&quot;? This cannot be undone.`"
      confirm-label="Delete"
      :danger="true"
      :loading="deleteLoading"
      @confirm="onConfirmDelete"
    />
  </div>
</template>

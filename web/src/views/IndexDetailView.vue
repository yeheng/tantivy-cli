<script setup>
import { ref, onMounted, watch } from 'vue'
import { useRouter } from 'vue-router'
import { TabGroup, TabList, Tab, TabPanels, TabPanel, Dialog, DialogPanel, DialogTitle, TransitionChild, TransitionRoot } from '@headlessui/vue'
import {
  ArrowLeftIcon, ArrowPathIcon, DocumentTextIcon, PuzzlePieceIcon,
  DocumentMagnifyingGlassIcon, DocumentPlusIcon,
  ChartBarIcon, Cog6ToothIcon, TrashIcon,
} from '@heroicons/vue/24/outline'
import StatCard from '../components/StatCard.vue'
import SchemaTable from '../components/SchemaTable.vue'
import SearchBar from '../components/SearchBar.vue'
import SearchResult from '../components/SearchResult.vue'
import DocTable from '../components/DocTable.vue'
import DocDetail from '../components/DocDetail.vue'
import Pagination from '../components/Pagination.vue'
import ConfirmDialog from '../components/ConfirmDialog.vue'
import {
  getIndexStats, searchIndex, addDoc, bulkAddDocs, listDocs,
  rebuildIndex, compressIndex, deleteIndex
} from '../api.js'

const props = defineProps({ name: String })
const router = useRouter()

// -- Shared state --
const stats = ref({ num_docs: 0, num_segments: 0, schema: { fields: [] } })
const statsLoading = ref(false)
const selectedTab = ref(0)

async function loadStats() {
  statsLoading.value = true
  try {
    stats.value = await getIndexStats(props.name)
  } catch (e) {
    console.error('Failed to load stats:', e)
  } finally {
    statsLoading.value = false
  }
}

onMounted(loadStats)
watch(() => props.name, () => { loadStats(); selectedTab.value = 0; searchResult.value = null; docs.value = [] })

// -- Search tab state --
const searchQ = ref('')
const searchLimit = ref(10)
const searchResult = ref(null)
const searchLoading = ref(false)

async function onSearch() {
  if (!searchQ.value.trim()) return
  searchLoading.value = true
  try {
    searchResult.value = await searchIndex(props.name, searchQ.value, searchLimit.value)
  } catch (e) {
    searchResult.value = { error: e.message }
  } finally {
    searchLoading.value = false
  }
}

// -- Documents tab state --
const docs = ref([])
const docColumns = ref([])
const listLimit = ref(10)
const listOffset = ref(0)
const listTotal = ref(0)
const listLoading = ref(false)
const selectedDoc = ref(null)
const showDocDetail = ref(false)

const showAddDoc = ref(false)
const docJson = ref(JSON.stringify({ title: '', body: '' }, null, 2))
const addResult = ref('')
const addLoading = ref(false)

const showBulkImport = ref(false)
const bulkJson = ref('')
const bulkLoading = ref(false)
const bulkResult = ref('')

async function onListDocs() {
  listLoading.value = true
  try {
    const result = await listDocs(props.name, listLimit.value, listOffset.value)
    docs.value = result
    listTotal.value = result.length < listLimit.value ? listOffset.value + result.length : listOffset.value + listLimit.value + 1
    if (result.length > 0) {
      docColumns.value = Object.keys(result[0]).slice(0, 5)
    }
  } catch (e) {
    docs.value = []
  } finally {
    listLoading.value = false
  }
}

function openDocDetail(doc) {
  selectedDoc.value = doc
  showDocDetail.value = true
}

async function onAddDoc() {
  addLoading.value = true
  addResult.value = ''
  try {
    const doc = JSON.parse(docJson.value)
    await addDoc(props.name, doc)
    addResult.value = 'Document added successfully'
    await onListDocs()
    await loadStats()
  } catch (e) {
    addResult.value = `Error: ${e.message}`
  } finally {
    addLoading.value = false
  }
}

async function onBulkImport() {
  bulkLoading.value = true
  bulkResult.value = ''
  try {
    const lines = bulkJson.value.trim().split('\n').filter((l) => l.trim())
    const docs_arr = lines.map((l) => JSON.parse(l))
    const result = await bulkAddDocs(props.name, docs_arr)
    bulkResult.value = `Added ${result.count} documents`
    await onListDocs()
    await loadStats()
  } catch (e) {
    bulkResult.value = `Error: ${e.message}`
  } finally {
    bulkLoading.value = false
  }
}

// -- Operations tab state --
const showConfirmDelete = ref(false)
const deleteLoading = ref(false)
const opsMessage = ref('')
const opsError = ref('')

async function onRebuild() {
  opsError.value = ''
  opsMessage.value = ''
  try {
    const result = await rebuildIndex(props.name)
    opsMessage.value = result.status || 'Rebuild started'
  } catch (e) {
    opsError.value = e.message
  }
}

async function onCompress() {
  opsError.value = ''
  opsMessage.value = ''
  try {
    const result = await compressIndex(props.name)
    opsMessage.value = result.status || 'Compressed'
    await loadStats()
  } catch (e) {
    opsError.value = e.message
  }
}

async function onConfirmDeleteIndex() {
  deleteLoading.value = true
  try {
    await deleteIndex(props.name)
    router.push('/indexes')
  } catch (e) {
    opsError.value = e.message
  } finally {
    deleteLoading.value = false
  }
}
</script>

<template>
  <div class="p-6 max-w-6xl mx-auto space-y-6">
    <!-- Header -->
    <div>
      <button @click="router.push('/indexes')"
        class="inline-flex items-center gap-1 text-xs text-gray-500 dark:text-gray-400 hover:text-gray-800 dark:hover:text-gray-200 transition-colors mb-2">
        <ArrowLeftIcon class="w-3.5 h-3.5" /> Back to Indexes
      </button>
      <h1 class="text-xl font-bold text-gray-900 dark:text-gray-100">{{ name }}</h1>
    </div>

    <!-- Stat cards -->
    <div class="grid grid-cols-3 gap-4">
      <StatCard :icon="DocumentTextIcon" :value="stats.num_docs?.toLocaleString() ?? '-'" label="Documents" />
      <StatCard :icon="PuzzlePieceIcon" :value="stats.num_segments ?? '-'" label="Segments" />
      <StatCard :icon="ChartBarIcon" :value="stats.status ?? 'active'" label="Status" />
    </div>

    <!-- Tabs -->
    <TabGroup v-model="selectedTab">
      <TabList class="flex gap-1 bg-white dark:bg-gray-800 p-1 rounded-lg shadow-sm border border-gray-200 dark:border-gray-700 w-fit">
        <Tab v-slot="{ selected }">
          <button :class="[
            'inline-flex items-center gap-1.5 px-3 py-1.5 text-sm font-medium rounded-md transition-colors',
            selected ? 'bg-sky-50 text-sky-700 dark:bg-sky-900/30 dark:text-sky-400' : 'text-gray-500 dark:text-gray-400 hover:text-gray-700 dark:hover:text-gray-200 hover:bg-gray-50 dark:hover:bg-gray-700'
          ]">
            <ChartBarIcon class="w-4 h-4" /> Overview
          </button>
        </Tab>
        <Tab v-slot="{ selected }">
          <button :class="[
            'inline-flex items-center gap-1.5 px-3 py-1.5 text-sm font-medium rounded-md transition-colors',
            selected ? 'bg-sky-50 text-sky-700 dark:bg-sky-900/30 dark:text-sky-400' : 'text-gray-500 dark:text-gray-400 hover:text-gray-700 dark:hover:text-gray-200 hover:bg-gray-50 dark:hover:bg-gray-700'
          ]">
            <DocumentMagnifyingGlassIcon class="w-4 h-4" /> Search
          </button>
        </Tab>
        <Tab v-slot="{ selected }">
          <button :class="[
            'inline-flex items-center gap-1.5 px-3 py-1.5 text-sm font-medium rounded-md transition-colors',
            selected ? 'bg-sky-50 text-sky-700 dark:bg-sky-900/30 dark:text-sky-400' : 'text-gray-500 dark:text-gray-400 hover:text-gray-700 dark:hover:text-gray-200 hover:bg-gray-50 dark:hover:bg-gray-700'
          ]">
            <DocumentPlusIcon class="w-4 h-4" /> Documents
          </button>
        </Tab>
        <Tab v-slot="{ selected }">
          <button :class="[
            'inline-flex items-center gap-1.5 px-3 py-1.5 text-sm font-medium rounded-md transition-colors',
            selected ? 'bg-sky-50 text-sky-700 dark:bg-sky-900/30 dark:text-sky-400' : 'text-gray-500 dark:text-gray-400 hover:text-gray-700 dark:hover:text-gray-200 hover:bg-gray-50 dark:hover:bg-gray-700'
          ]">
            <Cog6ToothIcon class="w-4 h-4" /> Operations
          </button>
        </Tab>
      </TabList>

      <TabPanels>
        <!-- Overview Tab -->
        <TabPanel>
          <div class="bg-white dark:bg-gray-800 rounded-xl border border-gray-200 dark:border-gray-700 p-5">
            <h3 class="text-sm font-semibold text-gray-900 dark:text-gray-100 mb-3">Schema</h3>
            <SchemaTable :schema="stats.schema" />
          </div>
        </TabPanel>

        <!-- Search Tab -->
        <TabPanel>
          <div class="bg-white dark:bg-gray-800 rounded-xl border border-gray-200 dark:border-gray-700 p-5 max-w-3xl">
            <SearchBar
              v-model="searchQ"
              :limit="searchLimit"
              @update:limit="searchLimit = $event"
              :loading="searchLoading"
              placeholder="title:hello AND body:world"
              @search="onSearch"
            />
            <div v-if="searchResult" class="mt-4 space-y-3">
              <div v-if="searchResult.error" class="text-sm text-red-600 dark:text-red-400">{{ searchResult.error }}</div>
              <template v-else>
                <div class="text-xs text-gray-500 dark:text-gray-400">
                  {{ searchResult.total }} result{{ searchResult.total !== 1 ? 's' : '' }}
                </div>
                <div v-if="!searchResult.hits?.length" class="text-sm text-gray-400 dark:text-gray-500 italic">No results.</div>
                <SearchResult v-for="(hit, i) in searchResult.hits" :key="i" :hit="hit" :index="i" :rank="i + 1" />
              </template>
            </div>
          </div>
        </TabPanel>

        <!-- Documents Tab -->
        <TabPanel>
          <div class="space-y-4">
            <div class="bg-white dark:bg-gray-800 rounded-xl border border-gray-200 dark:border-gray-700 p-5">
              <div class="flex items-center gap-2 mb-4">
                <button @click="showAddDoc = true"
                  class="inline-flex items-center gap-1 px-3 py-1.5 text-xs font-medium bg-sky-600 hover:bg-sky-700 text-white rounded-md transition-colors">
                  <DocumentPlusIcon class="w-3.5 h-3.5" /> Add Document
                </button>
                <button @click="showBulkImport = true"
                  class="inline-flex items-center gap-1 px-3 py-1.5 text-xs font-medium bg-white dark:bg-gray-700 border border-gray-200 dark:border-gray-600 text-gray-700 dark:text-gray-300 hover:bg-gray-50 dark:hover:bg-gray-600 rounded-md transition-colors">
                  Bulk Import
                </button>
                <div class="flex-1" />
                <button @click="onListDocs" :disabled="listLoading"
                  class="px-3 py-1.5 text-xs font-medium text-gray-600 dark:text-gray-400 hover:bg-gray-100 dark:hover:bg-gray-700 rounded-md transition-colors">
                  Load
                </button>
              </div>
              <DocTable :docs="docs" :columns="docColumns" :loading="listLoading" @row-click="openDocDetail" />
              <Pagination v-model:offset="listOffset" :limit="listLimit" :total="listTotal" />
            </div>

            <!-- Add Doc Dialog -->
            <TransitionRoot appear :show="showAddDoc" as="template">
              <Dialog as="div" @close="showAddDoc = false" class="relative z-50">
                <TransitionChild enter="duration-200 ease-out" enter-from="opacity-0" enter-to="opacity-100"
                                 leave="duration-150 ease-in" leave-from="opacity-100" leave-to="opacity-0">
                  <div class="fixed inset-0 bg-black/30" />
                </TransitionChild>
                <div class="fixed inset-0 flex items-center justify-center p-4">
                  <TransitionChild enter="duration-200 ease-out" enter-from="opacity-0 scale-95" enter-to="opacity-100 scale-100"
                                   leave="duration-150 ease-in" leave-from="opacity-100 scale-100" leave-to="opacity-0 scale-95">
                    <DialogPanel class="w-full max-w-lg bg-white dark:bg-gray-800 rounded-xl shadow-xl p-6">
                      <DialogTitle class="text-lg font-semibold text-gray-900 dark:text-gray-100 mb-4">Add Document</DialogTitle>
                      <textarea v-model="docJson" rows="8"
                        class="w-full border border-gray-300 dark:border-gray-600 rounded-lg px-3 py-2 text-xs font-mono bg-white dark:bg-gray-700 text-gray-900 dark:text-gray-100 focus:outline-none focus:ring-2 focus:ring-sky-500 mb-3" />
                      <p v-if="addResult" class="text-sm mb-3" :class="addResult.startsWith('Error') ? 'text-red-600 dark:text-red-400' : 'text-emerald-600 dark:text-emerald-400'">{{ addResult }}</p>
                      <div class="flex justify-end gap-2">
                        <button @click="showAddDoc = false" class="px-4 py-2 text-sm text-gray-600 dark:text-gray-300 hover:bg-gray-100 dark:hover:bg-gray-700 rounded-lg">Cancel</button>
                        <button @click="onAddDoc" :disabled="addLoading" class="px-4 py-2 text-sm text-white bg-emerald-600 hover:bg-emerald-700 disabled:opacity-50 rounded-lg">{{ addLoading ? 'Adding...' : 'Add' }}</button>
                      </div>
                    </DialogPanel>
                  </TransitionChild>
                </div>
              </Dialog>
            </TransitionRoot>

            <!-- Bulk Import Dialog -->
            <TransitionRoot appear :show="showBulkImport" as="template">
              <Dialog as="div" @close="showBulkImport = false" class="relative z-50">
                <TransitionChild enter="duration-200 ease-out" enter-from="opacity-0" enter-to="opacity-100"
                                 leave="duration-150 ease-in" leave-from="opacity-100" leave-to="opacity-0">
                  <div class="fixed inset-0 bg-black/30" />
                </TransitionChild>
                <div class="fixed inset-0 flex items-center justify-center p-4">
                  <TransitionChild enter="duration-200 ease-out" enter-from="opacity-0 scale-95" enter-to="opacity-100 scale-100"
                                   leave="duration-150 ease-in" leave-from="opacity-100 scale-100" leave-to="opacity-0 scale-95">
                    <DialogPanel class="w-full max-w-lg bg-white dark:bg-gray-800 rounded-xl shadow-xl p-6">
                      <DialogTitle class="text-lg font-semibold text-gray-900 dark:text-gray-100 mb-4">Bulk Import</DialogTitle>
                      <p class="text-xs text-gray-500 dark:text-gray-400 mb-2">One JSON document per line:</p>
                      <textarea v-model="bulkJson" rows="10" placeholder='{"title": "Hello", "body": "World"}'
                        class="w-full border border-gray-300 dark:border-gray-600 rounded-lg px-3 py-2 text-xs font-mono bg-white dark:bg-gray-700 text-gray-900 dark:text-gray-100 focus:outline-none focus:ring-2 focus:ring-sky-500 mb-3" />
                      <p v-if="bulkResult" class="text-sm mb-3" :class="bulkResult.startsWith('Error') ? 'text-red-600 dark:text-red-400' : 'text-emerald-600 dark:text-emerald-400'">{{ bulkResult }}</p>
                      <div class="flex justify-end gap-2">
                        <button @click="showBulkImport = false" class="px-4 py-2 text-sm text-gray-600 dark:text-gray-300 hover:bg-gray-100 dark:hover:bg-gray-700 rounded-lg">Cancel</button>
                        <button @click="onBulkImport" :disabled="bulkLoading" class="px-4 py-2 text-sm text-white bg-emerald-600 hover:bg-emerald-700 disabled:opacity-50 rounded-lg">{{ bulkLoading ? 'Importing...' : 'Import' }}</button>
                      </div>
                    </DialogPanel>
                  </TransitionChild>
                </div>
              </Dialog>
            </TransitionRoot>

            <DocDetail v-model:show="showDocDetail" :doc="selectedDoc" />
          </div>
        </TabPanel>

        <!-- Operations Tab -->
        <TabPanel>
          <div class="space-y-4 max-w-xl">
            <div v-if="opsMessage" class="p-3 rounded-lg bg-emerald-50 dark:bg-emerald-900/20 text-sm text-emerald-700 dark:text-emerald-400">{{ opsMessage }}</div>
            <div v-if="opsError" class="p-3 rounded-lg bg-red-50 dark:bg-red-900/20 text-sm text-red-700 dark:text-red-400">{{ opsError }}</div>

            <div class="bg-white dark:bg-gray-800 rounded-xl border border-gray-200 dark:border-gray-700 p-5">
              <h3 class="text-sm font-semibold text-gray-900 dark:text-gray-100 mb-3">Maintenance</h3>
              <div class="flex gap-3">
                <button @click="onRebuild"
                  class="inline-flex items-center gap-1.5 px-4 py-2 text-sm font-medium bg-white dark:bg-gray-700 border border-gray-200 dark:border-gray-600 text-gray-700 dark:text-gray-300 hover:bg-gray-50 dark:hover:bg-gray-600 rounded-lg transition-colors">
                  <ArrowPathIcon class="w-4 h-4" /> Rebuild Index
                </button>
                <button @click="onCompress"
                  class="inline-flex items-center gap-1.5 px-4 py-2 text-sm font-medium bg-white dark:bg-gray-700 border border-gray-200 dark:border-gray-600 text-gray-700 dark:text-gray-300 hover:bg-gray-50 dark:hover:bg-gray-600 rounded-lg transition-colors">
                  <PuzzlePieceIcon class="w-4 h-4" /> Compress Index
                </button>
              </div>
            </div>

            <div class="bg-white dark:bg-gray-800 rounded-xl border border-red-200 dark:border-red-900/50 p-5">
              <h3 class="text-sm font-semibold text-red-700 dark:text-red-400 mb-1">Danger Zone</h3>
              <p class="text-xs text-gray-500 dark:text-gray-400 mb-3">This action cannot be undone.</p>
              <button @click="showConfirmDelete = true"
                class="inline-flex items-center gap-1.5 px-4 py-2 text-sm font-medium bg-red-600 hover:bg-red-700 text-white rounded-lg transition-colors">
                <TrashIcon class="w-4 h-4" /> Delete Index
              </button>
            </div>

            <ConfirmDialog
              v-model:show="showConfirmDelete"
              title="Delete Index"
              :message="`Permanently delete index &quot;${name}&quot;? This cannot be undone.`"
              confirm-label="Delete"
              :danger="true"
              :loading="deleteLoading"
              @confirm="onConfirmDeleteIndex"
            />
          </div>
        </TabPanel>
      </TabPanels>
    </TabGroup>
  </div>
</template>

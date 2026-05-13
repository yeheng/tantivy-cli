<script setup>
import { ref, onMounted, watch } from 'vue'
import { useRouter } from 'vue-router'
import { TabGroup, TabList, Tab, TabPanels, TabPanel } from '@headlessui/vue'
import {
  ArrowLeftIcon, DocumentMagnifyingGlassIcon, DocumentPlusIcon,
  ListBulletIcon, ChartBarIcon, CogIcon, TrashIcon
} from '@heroicons/vue/24/outline'
import {
  getIndexStats, searchIndex, addDoc, listDocs,
  rebuildIndex, compressIndex, deleteIndex
} from '../api.js'

const props = defineProps({ name: String })
const router = useRouter()

const stats = ref({ num_docs: 0, num_segments: 0, schema: {} })
const statsLoading = ref(false)

const searchQ = ref('')
const searchLimit = ref(10)
const searchResult = ref(null)
const searchLoading = ref(false)

const docJson = ref(JSON.stringify({ id: '1', title: 'Hello', body: 'World' }, null, 2))
const addResult = ref('')
const addLoading = ref(false)

const docs = ref([])
const listLimit = ref(10)
const listOffset = ref(0)
const listLoading = ref(false)

async function loadStats() {
  statsLoading.value = true
  try {
    stats.value = await getIndexStats(props.name)
  } catch (e) {
    console.error(e)
  } finally {
    statsLoading.value = false
  }
}

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

async function onAddDoc() {
  addLoading.value = true
  addResult.value = ''
  try {
    const doc = JSON.parse(docJson.value)
    const res = await addDoc(props.name, doc)
    addResult.value = `Added: ${res.id}`
    await loadStats()
  } catch (e) {
    addResult.value = `Error: ${e.message}`
  } finally {
    addLoading.value = false
  }
}

async function onListDocs() {
  listLoading.value = true
  try {
    docs.value = await listDocs(props.name, listLimit.value, listOffset.value)
  } catch (e) {
    docs.value = []
  } finally {
    listLoading.value = false
  }
}

async function onRebuild() {
  try {
    await rebuildIndex(props.name)
    alert('Rebuild started.')
  } catch (e) {
    alert(e.message)
  }
}

async function onCompress() {
  try {
    await compressIndex(props.name)
    alert('Compress done.')
    await loadStats()
  } catch (e) {
    alert(e.message)
  }
}

async function onDelete() {
  if (!confirm(`Delete "${props.name}"?`)) return
  try {
    await deleteIndex(props.name)
    router.push('/')
  } catch (e) {
    alert(e.message)
  }
}

onMounted(() => {
  loadStats()
})
watch(() => props.name, () => {
  loadStats()
  searchResult.value = null
  docs.value = []
})
</script>

<template>
  <div class="h-full flex">
    <aside class="w-80 bg-white border-r border-gray-200 flex flex-col">
      <div class="px-4 py-3 border-b border-gray-200">
        <button @click="router.push('/')" class="inline-flex items-center gap-1 text-xs text-gray-500 hover:text-gray-800 transition-colors mb-2">
          <ArrowLeftIcon class="w-4 h-4" /> Back
        </button>
        <h2 class="text-lg font-bold text-gray-900">{{ name }}</h2>
      </div>
      <div class="p-4 space-y-3">
        <div class="grid grid-cols-2 gap-3">
          <div class="bg-gray-50 rounded-lg p-3 text-center">
            <div class="text-xl font-bold text-sky-700">{{ stats.num_docs ?? '-' }}</div>
            <div class="text-xs text-gray-500 mt-0.5">Documents</div>
          </div>
          <div class="bg-gray-50 rounded-lg p-3 text-center">
            <div class="text-xl font-bold text-sky-700">{{ stats.num_segments ?? '-' }}</div>
            <div class="text-xs text-gray-500 mt-0.5">Segments</div>
          </div>
        </div>
        <div class="flex flex-wrap gap-2">
          <button @click="loadStats" class="inline-flex items-center gap-1 px-2.5 py-1.5 text-xs font-medium bg-gray-100 hover:bg-gray-200 text-gray-700 rounded-md transition-colors">
            <ChartBarIcon class="w-3.5 h-3.5" /> Refresh
          </button>
          <button @click="onRebuild" class="inline-flex items-center gap-1 px-2.5 py-1.5 text-xs font-medium bg-gray-100 hover:bg-gray-200 text-gray-700 rounded-md transition-colors">
            <CogIcon class="w-3.5 h-3.5" /> Rebuild
          </button>
          <button @click="onCompress" class="inline-flex items-center gap-1 px-2.5 py-1.5 text-xs font-medium bg-gray-100 hover:bg-gray-200 text-gray-700 rounded-md transition-colors">
            <CogIcon class="w-3.5 h-3.5" /> Compress
          </button>
          <button @click="onDelete" class="inline-flex items-center gap-1 px-2.5 py-1.5 text-xs font-medium bg-red-50 hover:bg-red-100 text-red-700 rounded-md transition-colors">
            <TrashIcon class="w-3.5 h-3.5" /> Delete
          </button>
        </div>
      </div>
    </aside>

    <main class="flex-1 overflow-auto bg-gray-50 p-6">
      <TabGroup>
        <TabList class="flex gap-1 bg-white p-1 rounded-lg shadow-sm border border-gray-200 w-fit mb-4">
          <Tab v-slot="{ selected }">
            <button :class="[
              'inline-flex items-center gap-1.5 px-3 py-1.5 text-sm font-medium rounded-md transition-colors',
              selected ? 'bg-sky-50 text-sky-700' : 'text-gray-500 hover:text-gray-700 hover:bg-gray-50'
            ]">
              <DocumentMagnifyingGlassIcon class="w-4 h-4" /> Search
            </button>
          </Tab>
          <Tab v-slot="{ selected }">
            <button :class="[
              'inline-flex items-center gap-1.5 px-3 py-1.5 text-sm font-medium rounded-md transition-colors',
              selected ? 'bg-sky-50 text-sky-700' : 'text-gray-500 hover:text-gray-700 hover:bg-gray-50'
            ]">
              <DocumentPlusIcon class="w-4 h-4" /> Add Doc
            </button>
          </Tab>
          <Tab v-slot="{ selected }">
            <button :class="[
              'inline-flex items-center gap-1.5 px-3 py-1.5 text-sm font-medium rounded-md transition-colors',
              selected ? 'bg-sky-50 text-sky-700' : 'text-gray-500 hover:text-gray-700 hover:bg-gray-50'
            ]">
              <ListBulletIcon class="w-4 h-4" /> Documents
            </button>
          </Tab>
        </TabList>

        <TabPanels>
          <!-- Search -->
          <TabPanel>
            <div class="bg-white rounded-xl shadow-sm border border-gray-200 p-5 max-w-3xl">
              <div class="flex gap-3 mb-4">
                <input v-model="searchQ" @keydown.enter="onSearch" placeholder="title:hello AND body:world"
                       class="flex-1 border border-gray-300 rounded-lg px-3 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-sky-500" />
                <input v-model.number="searchLimit" type="number" min="1" max="100"
                       class="w-20 border border-gray-300 rounded-lg px-3 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-sky-500" />
                <button @click="onSearch" :disabled="searchLoading"
                        class="px-4 py-2 bg-sky-600 hover:bg-sky-700 disabled:opacity-50 text-white text-sm font-medium rounded-lg transition-colors">
                  {{ searchLoading ? '...' : 'Search' }}
                </button>
              </div>
              <div v-if="searchResult">
                <div v-if="searchResult.error" class="text-sm text-red-600">{{ searchResult.error }}</div>
                <div v-else>
                  <div class="text-xs text-gray-500 mb-2">
                    {{ searchResult.hits?.total?.value ?? searchResult.hits?.hits?.length ?? 0 }} results
                  </div>
                  <div v-if="!searchResult.hits?.hits?.length" class="text-sm text-gray-400 italic">No results.</div>
                  <div v-else class="space-y-3">
                    <div v-for="(hit, i) in searchResult.hits.hits" :key="i" class="border border-gray-100 rounded-lg p-3 bg-gray-50/50">
                      <div class="flex items-center justify-between mb-1">
                        <span class="text-xs font-semibold text-gray-500">#{{ i + 1 }}</span>
                        <span class="text-xs text-gray-400">score {{ hit._score?.toFixed(4) ?? '-' }}</span>
                      </div>
                      <pre class="text-xs text-gray-700 overflow-x-auto">{{ JSON.stringify(hit._source, null, 2) }}</pre>
                      <div v-if="hit.highlight" class="mt-2 space-y-0.5">
                        <div v-for="(vals, key) in hit.highlight" :key="key" class="text-xs">
                          <span class="font-medium text-gray-600">{{ key }}:</span>
                          <span class="text-gray-700" v-html="(Array.isArray(vals) ? vals.join(' ... ') : vals).replace(/<em>/g, '<mark class=\'bg-yellow-200 rounded px-0.5\'>').replace(/<\/em>/g, '</mark>')" />
                        </div>
                      </div>
                    </div>
                  </div>
                </div>
              </div>
            </div>
          </TabPanel>

          <!-- Add Doc -->
          <TabPanel>
            <div class="bg-white rounded-xl shadow-sm border border-gray-200 p-5 max-w-3xl">
              <div class="mb-3">
                <label class="block text-xs font-medium text-gray-600 mb-1">Document JSON</label>
                <textarea v-model="docJson" rows="10"
                          class="w-full border border-gray-300 rounded-lg px-3 py-2 text-xs font-mono focus:outline-none focus:ring-2 focus:ring-sky-500" />
              </div>
              <button @click="onAddDoc" :disabled="addLoading"
                      class="px-4 py-2 bg-emerald-600 hover:bg-emerald-700 disabled:opacity-50 text-white text-sm font-medium rounded-lg transition-colors">
                {{ addLoading ? 'Adding...' : 'Add Document' }}
              </button>
              <p v-if="addResult" class="mt-2 text-sm" :class="addResult.startsWith('Error') ? 'text-red-600' : 'text-emerald-700'">{{ addResult }}</p>
            </div>
          </TabPanel>

          <!-- List Docs -->
          <TabPanel>
            <div class="bg-white rounded-xl shadow-sm border border-gray-200 p-5 max-w-3xl">
              <div class="flex gap-3 mb-4">
                <input v-model.number="listLimit" type="number" min="1" max="100" placeholder="Limit"
                       class="w-24 border border-gray-300 rounded-lg px-3 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-sky-500" />
                <input v-model.number="listOffset" type="number" min="0" placeholder="Offset"
                       class="w-24 border border-gray-300 rounded-lg px-3 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-sky-500" />
                <button @click="onListDocs" :disabled="listLoading"
                        class="px-4 py-2 bg-sky-600 hover:bg-sky-700 disabled:opacity-50 text-white text-sm font-medium rounded-lg transition-colors">
                  {{ listLoading ? '...' : 'List' }}
                </button>
              </div>
              <div v-if="!docs.length" class="text-sm text-gray-400 italic">No documents loaded.</div>
              <div v-else class="space-y-2">
                <div v-for="(doc, i) in docs" :key="i" class="border border-gray-100 rounded-lg p-3 bg-gray-50/50">
                  <span class="text-xs font-semibold text-gray-500">#{{ listOffset + i + 1 }}</span>
                  <pre class="text-xs text-gray-700 overflow-x-auto mt-1">{{ JSON.stringify(doc, null, 2) }}</pre>
                </div>
              </div>
            </div>
          </TabPanel>
        </TabPanels>
      </TabGroup>
    </main>
  </div>
</template>

<script setup>
import { ref, onMounted, watch } from 'vue'
import { useRouter } from 'vue-router'
import { TabGroup, TabList, Tab, TabPanels, TabPanel } from '@headlessui/vue'
import {
  ArrowLeftIcon, ArrowPathIcon, DocumentTextIcon, PuzzlePieceIcon,
  DocumentMagnifyingGlassIcon, DocumentPlusIcon,
  ChartBarIcon, Cog6ToothIcon, TrashIcon,
} from '@heroicons/vue/24/outline'
import StatCard from '../components/StatCard.vue'
import SchemaTable from '../components/SchemaTable.vue'
import { getIndexStats } from '../api.js'

const props = defineProps({ name: String })
const router = useRouter()

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
watch(() => props.name, () => { loadStats(); selectedTab.value = 0 })
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

        <!-- Search Tab (placeholder) -->
        <TabPanel>
          <div class="bg-white dark:bg-gray-800 rounded-xl border border-gray-200 dark:border-gray-700 p-5">
            <p class="text-sm text-gray-400">Search tab</p>
          </div>
        </TabPanel>

        <!-- Documents Tab (placeholder) -->
        <TabPanel>
          <div class="bg-white dark:bg-gray-800 rounded-xl border border-gray-200 dark:border-gray-700 p-5">
            <p class="text-sm text-gray-400">Documents tab</p>
          </div>
        </TabPanel>

        <!-- Operations Tab (placeholder) -->
        <TabPanel>
          <div class="bg-white dark:bg-gray-800 rounded-xl border border-gray-200 dark:border-gray-700 p-5">
            <p class="text-sm text-gray-400">Operations tab</p>
          </div>
        </TabPanel>
      </TabPanels>
    </TabGroup>
  </div>
</template>

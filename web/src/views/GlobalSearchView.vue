<script setup>
import { ref } from 'vue'
import { FwbAccordion, FwbAccordionContent, FwbAccordionHeader, FwbAccordionPanel, FwbDropdown } from 'flowbite-vue'
import { ChevronDownIcon, MagnifyingGlassIcon } from '@heroicons/vue/24/outline'
import SearchBar from '../components/SearchBar.vue'
import SearchResult from '../components/SearchResult.vue'
import { listIndexes, searchIndex } from '../api.js'

const searchQ = ref('')
const searchLimit = ref(10)
const searchLoading = ref(false)
const selectedIndex = ref(null)
const indexOptions = ref([])
const results = ref([])

async function loadIndexOptions() {
  try {
    indexOptions.value = await listIndexes()
  } catch (e) {
    console.error(e)
  }
}
loadIndexOptions()

async function onSearch() {
  if (!searchQ.value.trim()) return
  searchLoading.value = true
  results.value = []
  try {
    const targets = selectedIndex.value ? [selectedIndex.value] : indexOptions.value
    const responses = await Promise.allSettled(
      targets.map(async (name) => {
        const result = await searchIndex(name, searchQ.value, searchLimit.value)
        return { name, result }
      })
    )
    results.value = responses
      .filter((r) => r.status === 'fulfilled' && r.value.result.total > 0)
      .map((r) => r.value)
  } catch (e) {
    console.error(e)
  } finally {
    searchLoading.value = false
  }
}

function selectIndex(name) {
  selectedIndex.value = name
}
</script>

<template>
  <div class="p-6 max-w-4xl mx-auto space-y-6">
    <div class="text-center py-8">
      <MagnifyingGlassIcon class="w-12 h-12 mx-auto mb-3 text-gray-300 dark:text-gray-600" />
      <h2 class="text-xl font-semibold text-gray-900 dark:text-gray-100 mb-4">Search Across Indexes</h2>
      <div class="max-w-2xl mx-auto space-y-3">
        <SearchBar
          v-model="searchQ"
          :limit="searchLimit"
          @update:limit="searchLimit = $event"
          :loading="searchLoading"
          placeholder="Search all indexes..."
          @search="onSearch"
        />
        <div class="flex justify-center">
          <FwbDropdown close-inside>
            <template #trigger>
              <button class="inline-flex items-center gap-1 px-3 py-1.5 text-sm border border-gray-200 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-800 text-gray-700 dark:text-gray-300 hover:bg-gray-50 dark:hover:bg-gray-700">
                {{ selectedIndex || 'All Indexes' }}
                <ChevronDownIcon class="w-4 h-4" />
              </button>
            </template>
            <div @click="selectIndex(null)" class="px-4 py-2 text-sm cursor-pointer hover:bg-gray-100 dark:hover:bg-gray-700 dark:text-gray-200">
              All Indexes
            </div>
            <div v-for="name in indexOptions" :key="name" @click="selectIndex(name)" class="px-4 py-2 text-sm cursor-pointer hover:bg-gray-100 dark:hover:bg-gray-700 dark:text-gray-200">
              {{ name }}
            </div>
          </FwbDropdown>
        </div>
      </div>
    </div>
    <div v-if="results.length" class="space-y-6">
      <div v-for="group in results" :key="group.name" class="bg-white dark:bg-gray-800 rounded-xl border border-gray-200 dark:border-gray-700">
        <FwbAccordion>
          <FwbAccordionPanel>
            <FwbAccordionHeader>
              <div class="flex items-center gap-2">
                <span class="text-sm font-semibold">{{ group.name }}</span>
                <span class="text-xs text-gray-400">{{ group.result.total }} results</span>
              </div>
            </FwbAccordionHeader>
            <FwbAccordionContent>
              <div class="space-y-2">
                <SearchResult v-for="(hit, i) in group.result.hits" :key="i" :hit="hit" :index="i" :rank="i + 1" />
              </div>
            </FwbAccordionContent>
          </FwbAccordionPanel>
        </FwbAccordion>
      </div>
    </div>
  </div>
</template>

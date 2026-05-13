<script setup>
import { ref } from 'vue'
import { Listbox, ListboxButton, ListboxOption, ListboxOptions, Disclosure, DisclosureButton, DisclosurePanel } from '@headlessui/vue'
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
          <div class="relative">
            <Listbox v-model="selectedIndex">
              <ListboxButton class="inline-flex items-center gap-1 px-3 py-1.5 text-sm border border-gray-200 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-800 text-gray-700 dark:text-gray-300 hover:bg-gray-50 dark:hover:bg-gray-700">
                {{ selectedIndex || 'All Indexes' }}
                <ChevronDownIcon class="w-4 h-4" />
              </ListboxButton>
              <ListboxOptions class="absolute mt-1 w-48 bg-white dark:bg-gray-800 rounded-lg shadow-lg border border-gray-200 dark:border-gray-700 py-1 z-10">
                <ListboxOption :value="null" v-slot="{ active, selected }">
                  <div :class="[active && 'bg-sky-50 dark:bg-sky-900/30', 'px-3 py-1.5 text-sm cursor-pointer', selected && 'font-medium text-sky-700 dark:text-sky-400']">All Indexes</div>
                </ListboxOption>
                <ListboxOption v-for="name in indexOptions" :key="name" :value="name" v-slot="{ active, selected }">
                  <div :class="[active && 'bg-sky-50 dark:bg-sky-900/30', 'px-3 py-1.5 text-sm cursor-pointer', selected && 'font-medium text-sky-700 dark:text-sky-400']">{{ name }}</div>
                </ListboxOption>
              </ListboxOptions>
            </Listbox>
          </div>
        </div>
      </div>
    </div>
    <div v-if="results.length" class="space-y-6">
      <Disclosure v-for="group in results" :key="group.name" :default-open="true" v-slot="{ open }">
        <div class="bg-white dark:bg-gray-800 rounded-xl border border-gray-200 dark:border-gray-700">
          <DisclosureButton class="flex items-center justify-between w-full px-5 py-3">
            <div class="flex items-center gap-2">
              <span class="text-sm font-semibold text-gray-900 dark:text-gray-100">{{ group.name }}</span>
              <span class="text-xs text-gray-400 dark:text-gray-500">{{ group.result.total }} results</span>
            </div>
            <ChevronDownIcon :class="['w-4 h-4 text-gray-400 transition-transform', open && 'rotate-180']" />
          </DisclosureButton>
          <DisclosurePanel class="px-5 pb-4 space-y-2">
            <SearchResult v-for="(hit, i) in group.result.hits" :key="i" :hit="hit" :index="i" :rank="i + 1" />
          </DisclosurePanel>
        </div>
      </Disclosure>
    </div>
  </div>
</template>

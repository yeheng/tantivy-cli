<script setup>
import { ref, onMounted } from 'vue'
import { useRouter } from 'vue-router'
import { Dialog, DialogPanel, DialogTitle, TransitionChild, TransitionRoot } from '@headlessui/vue'
import { PlusIcon, TrashIcon, MagnifyingGlassIcon } from '@heroicons/vue/24/outline'
import { listIndexes, createIndex, deleteIndex } from '../api.js'

const router = useRouter()
const indexes = ref([])
const loading = ref(false)
const error = ref('')

const showCreate = ref(false)
const newName = ref('')
const newSchema = ref(JSON.stringify({
  fields: [
    { name: 'title', kind: 'Text', stored: true, indexed: true, fast: false },
    { name: 'body',  kind: 'Text', stored: true, indexed: true, fast: false }
  ]
}, null, 2))
const createError = ref('')
const createLoading = ref(false)

async function load() {
  loading.value = true
  error.value = ''
  try {
    indexes.value = await listIndexes()
  } catch (e) {
    error.value = e.message
  } finally {
    loading.value = false
  }
}

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
    await load()
    router.push(`/index/${name}`)
  } catch (e) {
    createError.value = e.message
  } finally {
    createLoading.value = false
  }
}

async function onDelete(name) {
  if (!confirm(`Delete index "${name}" permanently?`)) return
  try {
    await deleteIndex(name)
    await load()
  } catch (e) {
    alert(e.message)
  }
}

onMounted(load)
</script>

<template>
  <div class="h-full flex">
    <aside class="w-80 bg-white border-r border-gray-200 flex flex-col">
      <div class="px-4 py-3 border-b border-gray-200 flex items-center justify-between">
        <h2 class="text-sm font-semibold text-gray-700 uppercase tracking-wider">Indexes</h2>
        <button @click="showCreate = true" class="inline-flex items-center gap-1 bg-sky-600 hover:bg-sky-700 text-white text-xs font-medium px-2.5 py-1.5 rounded transition-colors">
          <PlusIcon class="w-4 h-4" /> New
        </button>
      </div>
      <div class="flex-1 overflow-y-auto p-2">
        <div v-if="loading" class="text-center py-8 text-gray-400 text-sm">Loading...</div>
        <div v-else-if="error" class="text-center py-8 text-red-500 text-sm">{{ error }}</div>
        <div v-else-if="!indexes.length" class="text-center py-8 text-gray-400 text-sm">No indexes yet.</div>
        <ul v-else class="space-y-1">
          <li v-for="name in indexes" :key="name"
              class="group flex items-center justify-between px-3 py-2 rounded-lg cursor-pointer hover:bg-gray-50 transition-colors"
              @click="router.push(`/index/${name}`)">
            <span class="text-sm font-medium text-gray-800">{{ name }}</span>
            <div class="flex items-center gap-1 opacity-0 group-hover:opacity-100 transition-opacity">
              <button @click.stop="router.push(`/index/${name}`)" class="p-1 text-gray-400 hover:text-sky-600 transition-colors">
                <MagnifyingGlassIcon class="w-4 h-4" />
              </button>
              <button @click.stop="onDelete(name)" class="p-1 text-gray-400 hover:text-red-600 transition-colors">
                <TrashIcon class="w-4 h-4" />
              </button>
            </div>
          </li>
        </ul>
      </div>
    </aside>

    <section class="flex-1 flex items-center justify-center bg-gray-50">
      <div class="text-center text-gray-400">
        <MagnifyingGlassIcon class="w-12 h-12 mx-auto mb-3 opacity-30" />
        <p class="text-sm">Select an index from the sidebar, or create a new one.</p>
      </div>
    </section>

    <TransitionRoot appear :show="showCreate" as="template">
      <Dialog as="div" @close="showCreate = false" class="relative z-50">
        <TransitionChild enter="duration-200 ease-out" enter-from="opacity-0" enter-to="opacity-100"
                         leave="duration-150 ease-in" leave-from="opacity-100" leave-to="opacity-0">
          <div class="fixed inset-0 bg-black/30" />
        </TransitionChild>
        <div class="fixed inset-0 flex items-center justify-center p-4">
          <TransitionChild enter="duration-200 ease-out" enter-from="opacity-0 scale-95" enter-to="opacity-100 scale-100"
                           leave="duration-150 ease-in" leave-from="opacity-100 scale-100" leave-to="opacity-0 scale-95">
            <DialogPanel class="w-full max-w-lg bg-white rounded-xl shadow-xl p-6">
              <DialogTitle class="text-lg font-semibold text-gray-900 mb-4">Create Index</DialogTitle>
              <div class="space-y-4">
                <div>
                  <label class="block text-xs font-medium text-gray-600 mb-1">Name</label>
                  <input v-model="newName" class="w-full border border-gray-300 rounded-lg px-3 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-sky-500" placeholder="articles" />
                </div>
                <div>
                  <label class="block text-xs font-medium text-gray-600 mb-1">Schema JSON</label>
                  <textarea v-model="newSchema" rows="8" class="w-full border border-gray-300 rounded-lg px-3 py-2 text-xs font-mono focus:outline-none focus:ring-2 focus:ring-sky-500" />
                </div>
                <p v-if="createError" class="text-sm text-red-600">{{ createError }}</p>
                <div class="flex justify-end gap-2 pt-2">
                  <button @click="showCreate = false" class="px-4 py-2 text-sm font-medium text-gray-600 hover:bg-gray-100 rounded-lg transition-colors">Cancel</button>
                  <button @click="onCreate" :disabled="createLoading" class="px-4 py-2 text-sm font-medium text-white bg-sky-600 hover:bg-sky-700 disabled:opacity-50 rounded-lg transition-colors">
                    {{ createLoading ? 'Creating...' : 'Create' }}
                  </button>
                </div>
              </div>
            </DialogPanel>
          </TransitionChild>
        </div>
      </Dialog>
    </TransitionRoot>
  </div>
</template>

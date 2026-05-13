# Web UI Dashboard First — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Transform the existing Tantivy CLI web prototype into a team dashboard with collapsible sidebar, dark/light themes, dashboard overview, advanced search, document management, and monitoring.

**Architecture:** Vue 3 SPA with collapsible sidebar + top bar layout. Views own state and call API functions; components are presentational (props in, events out). Composables handle cross-cutting concerns (theme, auto-refresh). Headless UI provides accessible interactive primitives.

**Tech Stack:** Vue 3, Vue Router, @headlessui/vue, @heroicons/vue, Tailwind CSS (dark mode via `class` strategy), Vite, Axios

**Spec:** `docs/superpowers/specs/2026-05-13-web-ui-dashboard-design.md`

---

## File Structure Map

**Create:**
```
web/src/composables/useTheme.js
web/src/composables/useAutoRefresh.js
web/src/components/AppSidebar.vue
web/src/components/AppTopbar.vue
web/src/components/StatCard.vue
web/src/components/IndexTable.vue
web/src/components/SearchBar.vue
web/src/components/SearchResult.vue
web/src/components/DocTable.vue
web/src/components/DocDetail.vue
web/src/components/SchemaTable.vue
web/src/components/ConfirmDialog.vue
web/src/components/ThemeToggle.vue
web/src/components/Pagination.vue
web/src/views/DashboardView.vue
web/src/views/IndexListView.vue
web/src/views/IndexDetailView.vue
web/src/views/GlobalSearchView.vue
web/src/views/SettingsView.vue
```

**Modify:**
```
web/tailwind.config.js          — enable dark mode 'class'
web/src/style.css               — add dark mode base styles
web/src/main.js                 — rewrite router + routes
web/src/App.vue                 — rewrite as shell layout
web/src/api.js                  — update to match actual API response shapes
```

**Delete:**
```
web/src/views/HomeView.vue
web/src/views/IndexView.vue
```

---

## API Response Reference

These are the exact shapes returned by the server (from `src/server/*.rs`):

```
GET  /indexes                      → string[]
GET  /indexes/:name/stats          → { num_docs: u64, num_segments: usize, schema: object, status: string|null }
GET  /indexes/:name/search?q=&limit=&offset=&highlight= → { total, hits: [{ doc, score, snippets }], query, limit, offset }
POST /indexes/:name  body:{schema} → { "index": name }
DELETE /indexes/:name              → 204
POST /indexes/:name/docs  body:{}  → { "id": string }
POST /indexes/:name/docs/_bulk body:[] → { "count": u64 }
GET  /indexes/:name/docs?limit=&offset= → JsonValue[]
GET  /indexes/:name/docs/:field/:value  → JsonValue
DELETE /indexes/:name/docs/:field/:value → { "status": "scheduled" }
POST /indexes/:name/rebuild       → { "status": "rebuilding_started" }
POST /indexes/:name/compress      → { "status": "compressed" }
```

---

### Task 1: Tailwind Dark Mode + Style Base

**Files:**
- Modify: `web/tailwind.config.js`
- Modify: `web/src/style.css`

- [ ] **Step 1: Enable dark mode in tailwind config**

Replace `web/tailwind.config.js` with:

```js
/** @type {import('tailwindcss').Config} */
export default {
  content: [
    "./index.html",
    "./src/**/*.{vue,js,ts}",
  ],
  darkMode: 'class',
  theme: {
    extend: {},
  },
  plugins: [],
}
```

- [ ] **Step 2: Update style.css with dark mode base styles**

Replace `web/src/style.css` with:

```css
@tailwind base;
@tailwind components;
@tailwind utilities;

html, body, #app {
  height: 100%;
}

@layer base {
  body {
    @apply bg-gray-50 text-gray-900 dark:bg-gray-900 dark:text-gray-100;
  }
}
```

- [ ] **Step 3: Verify build**

Run: `cd web && npm run build`
Expected: Build succeeds with no errors.

- [ ] **Step 4: Commit**

```bash
git add web/tailwind.config.js web/src/style.css
git commit -m "feat(ui): enable tailwind dark mode and update base styles"
```

---

### Task 2: Composables (useTheme, useAutoRefresh)

**Files:**
- Create: `web/src/composables/useTheme.js`
- Create: `web/src/composables/useAutoRefresh.js`

- [ ] **Step 1: Create useTheme composable**

Create `web/src/composables/useTheme.js`:

```js
import { ref, watchEffect, onMounted } from 'vue'

const theme = ref(localStorage.getItem('theme') || 'system')

function applyTheme(t) {
  const dark = t === 'dark' || (t === 'system' && window.matchMedia('(prefers-color-scheme: dark)').matches)
  document.documentElement.classList.toggle('dark', dark)
}

export function useTheme() {
  onMounted(() => {
    const mq = window.matchMedia('(prefers-color-scheme: dark)')
    const handler = () => { if (theme.value === 'system') applyTheme('system') }
    mq.addEventListener('change', handler)
  })

  watchEffect(() => {
    localStorage.setItem('theme', theme.value)
    applyTheme(theme.value)
  })

  return {
    theme,
    setTheme: (t) => { theme.value = t },
    isDark: () => document.documentElement.classList.contains('dark'),
  }
}
```

- [ ] **Step 2: Create useAutoRefresh composable**

Create `web/src/composables/useAutoRefresh.js`:

```js
import { ref, onMounted, onUnmounted } from 'vue'

export function useAutoRefresh(fetchFn, intervalMs = 30000) {
  const lastUpdated = ref(null)
  const isPaused = ref(false)
  let timer = null

  function start() {
    stop()
    timer = setInterval(async () => {
      if (!isPaused.value) {
        await fetchFn()
        lastUpdated.value = new Date()
      }
    }, intervalMs)
  }

  function stop() {
    if (timer) { clearInterval(timer); timer = null }
  }

  function pause() { isPaused.value = true }
  function resume() { isPaused.value = false }

  onMounted(() => { fetchFn(); start() })
  onUnmounted(stop)

  return { lastUpdated, isPaused, pause, resume, refresh: fetchFn }
}
```

- [ ] **Step 3: Verify build**

Run: `cd web && npm run build`
Expected: Build succeeds.

- [ ] **Step 4: Commit**

```bash
git add web/src/composables/
git commit -m "feat(ui): add useTheme and useAutoRefresh composables"
```

---

### Task 3: Layout Shell (App.vue, Sidebar, Topbar, Router)

**Files:**
- Modify: `web/src/main.js`
- Modify: `web/src/App.vue`
- Create: `web/src/components/AppSidebar.vue`
- Create: `web/src/components/AppTopbar.vue`

- [ ] **Step 1: Rewrite main.js with new routes**

Replace `web/src/main.js` with:

```js
import { createApp } from 'vue'
import { createRouter, createWebHistory } from 'vue-router'
import App from './App.vue'
import './style.css'

import DashboardView from './views/DashboardView.vue'
import IndexListView from './views/IndexListView.vue'
import IndexDetailView from './views/IndexDetailView.vue'
import GlobalSearchView from './views/GlobalSearchView.vue'
import SettingsView from './views/SettingsView.vue'

const router = createRouter({
  history: createWebHistory(),
  routes: [
    { path: '/', component: DashboardView },
    { path: '/indexes', component: IndexListView },
    { path: '/index/:name', component: IndexDetailView, props: true },
    { path: '/search', component: GlobalSearchView },
    { path: '/settings', component: SettingsView },
  ],
})

createApp(App).use(router).mount('#app')
```

Note: Views don't exist yet — build will fail until Task 6. That's expected. We'll create placeholder views first.

- [ ] **Step 2: Create minimal placeholder views for build**

Create `web/src/views/DashboardView.vue`:

```vue
<template>
  <div class="p-6 text-gray-500">Dashboard placeholder</div>
</template>
```

Create `web/src/views/IndexListView.vue`:

```vue
<template>
  <div class="p-6 text-gray-500">Index list placeholder</div>
</template>
```

Create `web/src/views/IndexDetailView.vue`:

```vue
<script setup>
defineProps({ name: String })
</script>
<template>
  <div class="p-6 text-gray-500">Index detail: {{ name }}</div>
</template>
```

Create `web/src/views/GlobalSearchView.vue`:

```vue
<template>
  <div class="p-6 text-gray-500">Search placeholder</div>
</template>
```

Create `web/src/views/SettingsView.vue`:

```vue
<template>
  <div class="p-6 text-gray-500">Settings placeholder</div>
</template>
```

- [ ] **Step 3: Create AppSidebar component**

Create `web/src/components/AppSidebar.vue`:

```vue
<script setup>
import { computed } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { Dialog, DialogPanel, TransitionChild, TransitionRoot } from '@headlessui/vue'
import {
  ChartBarIcon, FolderIcon, MagnifyingGlassIcon, Cog6ToothyIcon as CogIcon,
  Bars3Icon, XMarkIcon,
} from '@heroicons/vue/24/outline'

const props = defineProps({
  collapsed: Boolean,
  mobileOpen: Boolean,
})
const emit = defineEmits(['update:collapsed', 'update:mobileOpen'])

const route = useRoute()
const router = useRouter()

const navItems = [
  { icon: ChartBarIcon, label: 'Dashboard', to: '/' },
  { icon: FolderIcon, label: 'Indexes', to: '/indexes' },
  { icon: MagnifyingGlassIcon, label: 'Search', to: '/search' },
  { icon: CogIcon, label: 'Settings', to: '/settings' },
]

const isActive = (to) => {
  if (to === '/') return route.path === '/'
  return route.path.startsWith(to)
}

function navigate(to) {
  router.push(to)
  emit('update:mobileOpen', false)
}
</script>

<template>
  <!-- Desktop sidebar -->
  <aside
    :class="[
      'hidden md:flex flex-col border-r bg-white dark:bg-gray-800 dark:border-gray-700 transition-all duration-200 shrink-0',
      collapsed ? 'w-16' : 'w-52'
    ]"
  >
    <div class="flex-1 py-3 px-2 space-y-1">
      <button
        v-for="item in navItems" :key="item.to"
        @click="navigate(item.to)"
        :class="[
          'flex items-center gap-3 w-full px-3 py-2 rounded-lg text-sm font-medium transition-colors',
          isActive(item.to)
            ? 'bg-sky-50 text-sky-700 dark:bg-sky-900/30 dark:text-sky-400'
            : 'text-gray-600 hover:bg-gray-100 dark:text-gray-400 dark:hover:bg-gray-700'
        ]"
        :title="collapsed ? item.label : ''"
      >
        <component :is="item.icon" class="w-5 h-5 shrink-0" />
        <span v-if="!collapsed" class="truncate">{{ item.label }}</span>
      </button>
    </div>
  </aside>

  <!-- Mobile sidebar (drawer) -->
  <TransitionRoot :show="mobileOpen" as="template">
    <Dialog class="relative z-50 md:hidden" @close="emit('update:mobileOpen', false)">
      <TransitionChild
        enter="duration-200 ease-out" enter-from="opacity-0" enter-to="opacity-100"
        leave="duration-150 ease-in" leave-from="opacity-100" leave-to="opacity-0"
      >
        <div class="fixed inset-0 bg-black/30" />
      </TransitionChild>
      <div class="fixed inset-0 flex">
        <TransitionChild
          enter="duration-200 ease-out" enter-from="-translate-x-full" enter-to="translate-x-0"
          leave="duration-150 ease-in" leave-from="translate-x-0" leave-to="-translate-x-full"
        >
          <DialogPanel class="relative w-64 bg-white dark:bg-gray-800 flex flex-col">
            <div class="flex items-center justify-between px-4 py-3 border-b dark:border-gray-700">
              <span class="text-sm font-semibold text-gray-900 dark:text-gray-100">Navigation</span>
              <button @click="emit('update:mobileOpen', false)" class="p-1 text-gray-400 hover:text-gray-600">
                <XMarkIcon class="w-5 h-5" />
              </button>
            </div>
            <div class="flex-1 py-3 px-2 space-y-1">
              <button
                v-for="item in navItems" :key="item.to"
                @click="navigate(item.to)"
                :class="[
                  'flex items-center gap-3 w-full px-3 py-2 rounded-lg text-sm font-medium transition-colors',
                  isActive(item.to)
                    ? 'bg-sky-50 text-sky-700 dark:bg-sky-900/30 dark:text-sky-400'
                    : 'text-gray-600 hover:bg-gray-100 dark:text-gray-400 dark:hover:bg-gray-700'
                ]"
              >
                <component :is="item.icon" class="w-5 h-5 shrink-0" />
                <span>{{ item.label }}</span>
              </button>
            </div>
          </DialogPanel>
        </TransitionChild>
      </div>
    </Dialog>
  </TransitionRoot>
</template>
```

- [ ] **Step 4: Create AppTopbar component**

Create `web/src/components/AppTopbar.vue`:

```vue
<script setup>
import { Bars3Icon } from '@heroicons/vue/24/outline'
import ThemeToggle from './ThemeToggle.vue'

defineProps({ title: String })
const emit = defineEmits(['toggleSidebar'])
</script>

<template>
  <header class="h-12 flex items-center justify-between px-4 border-b bg-white dark:bg-gray-800 dark:border-gray-700 shrink-0">
    <div class="flex items-center gap-3">
      <button @click="emit('toggleSidebar')" class="p-1 text-gray-500 hover:text-gray-700 dark:text-gray-400 dark:hover:text-gray-200">
        <Bars3Icon class="w-5 h-5" />
      </button>
      <span class="text-sm font-semibold text-gray-900 dark:text-gray-100">{{ title }}</span>
    </div>
    <div class="flex items-center gap-2">
      <ThemeToggle />
    </div>
  </header>
</template>
```

- [ ] **Step 5: Create ThemeToggle component**

Create `web/src/components/ThemeToggle.vue`:

```vue
<script setup>
import { Switch } from '@headlessui/vue'
import { useTheme } from '../composables/useTheme.js'
import { SunIcon, MoonIcon } from '@heroicons/vue/24/outline'

const { theme, setTheme } = useTheme()

function toggle() {
  if (theme.value === 'dark') setTheme('light')
  else setTheme('dark')
}
</script>

<template>
  <button
    @click="toggle"
    class="p-1.5 rounded-lg text-gray-500 hover:bg-gray-100 dark:text-gray-400 dark:hover:bg-gray-700 transition-colors"
    :title="theme === 'dark' ? 'Switch to light' : 'Switch to dark'"
  >
    <SunIcon v-if="theme === 'dark'" class="w-4.5 h-4.5" />
    <MoonIcon v-else class="w-4.5 h-4.5" />
  </button>
</template>
```

- [ ] **Step 6: Rewrite App.vue as shell layout**

Replace `web/src/App.vue` with:

```vue
<script setup>
import { ref, computed } from 'vue'
import { useRoute } from 'vue-router'
import AppSidebar from './components/AppSidebar.vue'
import AppTopbar from './components/AppTopbar.vue'

const route = useRoute()
const sidebarCollapsed = ref(false)
const mobileOpen = ref(false)

const pageTitle = computed(() => {
  if (route.path === '/') return 'Dashboard'
  if (route.path === '/indexes') return 'Indexes'
  if (route.path.startsWith('/index/')) return route.params.name
  if (route.path === '/search') return 'Search'
  if (route.path === '/settings') return 'Settings'
  return 'Tantivy CLI'
})
</script>

<template>
  <div class="h-full flex flex-col">
    <div class="flex flex-1 overflow-hidden">
      <AppSidebar
        v-model:collapsed="sidebarCollapsed"
        v-model:mobileOpen="mobileOpen"
      />
      <div class="flex-1 flex flex-col overflow-hidden">
        <AppTopbar :title="pageTitle" @toggle-sidebar="sidebarCollapsed = !sidebarCollapsed" />
        <main class="flex-1 overflow-auto">
          <router-view />
        </main>
      </div>
    </div>
  </div>
</template>
```

- [ ] **Step 7: Verify build**

Run: `cd web && npm run build`
Expected: Build succeeds.

- [ ] **Step 8: Commit**

```bash
git add web/src/main.js web/src/App.vue web/src/components/AppSidebar.vue web/src/components/AppTopbar.vue web/src/components/ThemeToggle.vue web/src/views/DashboardView.vue web/src/views/IndexListView.vue web/src/views/IndexDetailView.vue web/src/views/GlobalSearchView.vue web/src/views/SettingsView.vue
git commit -m "feat(ui): layout shell with collapsible sidebar, topbar, and routing"
```

---

### Task 4: Reusable Components

**Files:**
- Create: `web/src/components/StatCard.vue`
- Create: `web/src/components/ConfirmDialog.vue`
- Create: `web/src/components/Pagination.vue`
- Create: `web/src/components/SearchBar.vue`

- [ ] **Step 1: Create StatCard component**

Create `web/src/components/StatCard.vue`:

```vue
<script setup>
defineProps({
  icon: { type: [Object, Function], default: null },
  value: { type: [String, Number], default: '-' },
  label: { type: String, required: true },
})
</script>

<template>
  <div class="bg-white dark:bg-gray-800 rounded-xl border border-gray-200 dark:border-gray-700 p-4">
    <div class="flex items-center gap-3">
      <div v-if="icon" class="p-2 rounded-lg bg-sky-50 dark:bg-sky-900/30">
        <component :is="icon" class="w-5 h-5 text-sky-600 dark:text-sky-400" />
      </div>
      <div>
        <div class="text-2xl font-bold text-gray-900 dark:text-gray-100">{{ value }}</div>
        <div class="text-xs text-gray-500 dark:text-gray-400">{{ label }}</div>
      </div>
    </div>
  </div>
</template>
```

- [ ] **Step 2: Create ConfirmDialog component**

Create `web/src/components/ConfirmDialog.vue`:

```vue
<script setup>
import { Dialog, DialogPanel, DialogTitle, TransitionChild, TransitionRoot } from '@headlessui/vue'

defineProps({
  show: Boolean,
  title: { type: String, default: 'Confirm' },
  message: { type: String, required: true },
  confirmLabel: { type: String, default: 'Confirm' },
  danger: Boolean,
  loading: Boolean,
})
const emit = defineEmits(['update:show', 'confirm'])
</script>

<template>
  <TransitionRoot appear :show="show" as="template">
    <Dialog as="div" @close="emit('update:show', false)" class="relative z-50">
      <TransitionChild
        enter="duration-200 ease-out" enter-from="opacity-0" enter-to="opacity-100"
        leave="duration-150 ease-in" leave-from="opacity-100" leave-to="opacity-0"
      >
        <div class="fixed inset-0 bg-black/30" />
      </TransitionChild>
      <div class="fixed inset-0 flex items-center justify-center p-4">
        <TransitionChild
          enter="duration-200 ease-out" enter-from="opacity-0 scale-95" enter-to="opacity-100 scale-100"
          leave="duration-150 ease-in" leave-from="opacity-100 scale-100" leave-to="opacity-0 scale-95"
        >
          <DialogPanel class="w-full max-w-md bg-white dark:bg-gray-800 rounded-xl shadow-xl p-6">
            <DialogTitle class="text-lg font-semibold text-gray-900 dark:text-gray-100">{{ title }}</DialogTitle>
            <p class="mt-2 text-sm text-gray-600 dark:text-gray-400">{{ message }}</p>
            <div class="mt-4 flex justify-end gap-2">
              <button
                @click="emit('update:show', false)"
                class="px-4 py-2 text-sm font-medium text-gray-600 dark:text-gray-300 hover:bg-gray-100 dark:hover:bg-gray-700 rounded-lg transition-colors"
              >Cancel</button>
              <button
                @click="emit('confirm')"
                :disabled="loading"
                :class="[
                  'px-4 py-2 text-sm font-medium text-white rounded-lg transition-colors disabled:opacity-50',
                  danger ? 'bg-red-600 hover:bg-red-700' : 'bg-sky-600 hover:bg-sky-700'
                ]"
              >{{ loading ? '...' : confirmLabel }}</button>
            </div>
          </DialogPanel>
        </TransitionChild>
      </div>
    </Dialog>
  </TransitionRoot>
</template>
```

- [ ] **Step 3: Create Pagination component**

Create `web/src/components/Pagination.vue`:

```vue
<script setup>
const props = defineProps({
  offset: { type: Number, default: 0 },
  limit: { type: Number, default: 10 },
  total: { type: Number, default: 0 },
})
const emit = defineEmits(['update:offset'])

const page = computed(() => Math.floor(props.offset / props.limit) + 1)
const totalPages = computed(() => Math.max(1, Math.ceil(props.total / props.limit)))
const showing = computed(() => `${props.offset + 1}-${Math.min(props.offset + props.limit, props.total)} of ${props.total}`)

import { computed } from 'vue'

function prev() { if (props.offset > 0) emit('update:offset', Math.max(0, props.offset - props.limit)) }
function next() { if (page.value < totalPages.value) emit('update:offset', props.offset + props.limit) }
</script>

<template>
  <div class="flex items-center justify-between py-3 text-sm text-gray-600 dark:text-gray-400">
    <span v-if="total > 0">Showing {{ showing }}</span>
    <span v-else>No results</span>
    <div class="flex items-center gap-2">
      <button @click="prev" :disabled="offset === 0"
        class="px-3 py-1 rounded-lg border border-gray-200 dark:border-gray-600 hover:bg-gray-100 dark:hover:bg-gray-700 disabled:opacity-50 transition-colors">Prev</button>
      <span>{{ page }} / {{ totalPages }}</span>
      <button @click="next" :disabled="page >= totalPages"
        class="px-3 py-1 rounded-lg border border-gray-200 dark:border-gray-600 hover:bg-gray-100 dark:hover:bg-gray-700 disabled:opacity-50 transition-colors">Next</button>
    </div>
  </div>
</template>
```

- [ ] **Step 4: Create SearchBar component**

Create `web/src/components/SearchBar.vue`:

```vue
<script setup>
const props = defineProps({
  modelValue: { type: String, default: '' },
  limit: { type: Number, default: 10 },
  loading: Boolean,
  placeholder: { type: String, default: 'Search...' },
})
const emit = defineEmits(['update:modelValue', 'update:limit', 'search'])
</script>

<template>
  <div class="flex gap-3">
    <input
      :value="modelValue"
      @input="emit('update:modelValue', $event.target.value)"
      @keydown.enter="emit('search')"
      :placeholder="placeholder"
      class="flex-1 border border-gray-300 dark:border-gray-600 rounded-lg px-3 py-2 text-sm bg-white dark:bg-gray-800 text-gray-900 dark:text-gray-100 focus:outline-none focus:ring-2 focus:ring-sky-500"
    />
    <select
      :value="limit"
      @change="emit('update:limit', Number($event.target.value))"
      class="w-20 border border-gray-300 dark:border-gray-600 rounded-lg px-2 py-2 text-sm bg-white dark:bg-gray-800 text-gray-900 dark:text-gray-100"
    >
      <option :value="10">10</option>
      <option :value="20">20</option>
      <option :value="50">50</option>
      <option :value="100">100</option>
    </select>
    <button
      @click="emit('search')"
      :disabled="loading"
      class="px-4 py-2 bg-sky-600 hover:bg-sky-700 disabled:opacity-50 text-white text-sm font-medium rounded-lg transition-colors"
    >{{ loading ? '...' : 'Search' }}</button>
  </div>
</template>
```

- [ ] **Step 5: Verify build**

Run: `cd web && npm run build`
Expected: Build succeeds.

- [ ] **Step 6: Commit**

```bash
git add web/src/components/StatCard.vue web/src/components/ConfirmDialog.vue web/src/components/Pagination.vue web/src/components/SearchBar.vue
git commit -m "feat(ui): add reusable StatCard, ConfirmDialog, Pagination, SearchBar components"
```

---

### Task 5: Update API Layer

**Files:**
- Modify: `web/src/api.js`

- [ ] **Step 1: Rewrite api.js to match actual server response shapes**

Replace `web/src/api.js` with:

```js
import axios from 'axios'

const api = axios.create({
  baseURL: '',
  headers: { 'Content-Type': 'application/json' },
})

api.interceptors.response.use(
  (r) => r,
  (err) => {
    const msg = err.response?.data?.error || err.message
    return Promise.reject(new Error(msg))
  }
)

export default api

// Index operations
export async function listIndexes() {
  const { data } = await api.get('/indexes')
  return data // string[]
}

export async function createIndex(name, schema) {
  const { data } = await api.post(`/indexes/${name}`, { schema })
  return data // { index: string }
}

export async function deleteIndex(name) {
  await api.delete(`/indexes/${name}`) // 204
}

export async function getIndexInfo(name) {
  const { data } = await api.get(`/indexes/${name}`)
  return data // { name, num_docs, schema }
}

// Stats & maintenance
export async function getIndexStats(name) {
  const { data } = await api.get(`/indexes/${name}/stats`)
  return data // { num_docs, num_segments, schema, status }
}

export async function rebuildIndex(name) {
  const { data } = await api.post(`/indexes/${name}/rebuild`)
  return data // { status }
}

export async function compressIndex(name) {
  const { data } = await api.post(`/indexes/${name}/compress`)
  return data // { status }
}

// Document operations
export async function addDoc(name, doc) {
  const { data } = await api.post(`/indexes/${name}/docs`, doc)
  return data // { id }
}

export async function bulkAddDocs(name, docs) {
  const { data } = await api.post(`/indexes/${name}/docs/_bulk`, docs)
  return data // { count }
}

export async function listDocs(name, limit = 10, offset = 0) {
  const { data } = await api.get(`/indexes/${name}/docs`, { params: { limit, offset } })
  return data // JsonValue[]
}

export async function getDoc(name, field, value) {
  const { data } = await api.get(`/indexes/${name}/docs/${field}/${encodeURIComponent(value)}`)
  return data // JsonValue
}

export async function deleteDoc(name, field, value) {
  const { data } = await api.delete(`/indexes/${name}/docs/${field}/${encodeURIComponent(value)}`)
  return data // { status }
}

// Search
export async function searchIndex(name, q, limit = 10, offset = 0, highlight = []) {
  const { data } = await api.get(`/indexes/${name}/search`, {
    params: { q, limit, offset, highlight }
  })
  return data // { total, hits: [{ doc, score, snippets }], query, limit, offset }
}

export async function searchIndexPost(name, body) {
  const { data } = await api.post(`/indexes/${name}/search`, body)
  return data // { total, hits: [{ doc, score, snippets }], query, limit, offset }
}
```

- [ ] **Step 2: Verify build**

Run: `cd web && npm run build`
Expected: Build succeeds.

- [ ] **Step 3: Commit**

```bash
git add web/src/api.js
git commit -m "feat(ui): update API layer with correct response shapes and all endpoints"
```

---

### Task 6: Dashboard Page

**Files:**
- Replace: `web/src/views/DashboardView.vue` (placeholder from Task 3)
- Create: `web/src/components/IndexTable.vue`

- [ ] **Step 1: Create IndexTable component**

Create `web/src/components/IndexTable.vue`:

```vue
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
```

- [ ] **Step 2: Implement DashboardView**

Replace `web/src/views/DashboardView.vue` with:

```vue
<script setup>
import { ref, computed } from 'vue'
import { useRouter } from 'vue-router'
import { Dialog, DialogPanel, DialogTitle, TransitionChild, TransitionRoot } from '@headlessui/vue'
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
    <TransitionRoot appear :show="showCreate" as="template">
      <Dialog as="div" @close="showCreate = false" class="relative z-50">
        <TransitionChild enter="duration-200 ease-out" enter-from="opacity-0" enter-to="opacity-100"
                         leave="duration-150 ease-in" leave-from="opacity-100" leave-to="opacity-0">
          <div class="fixed inset-0 bg-black/30" />
        </TransitionChild>
        <div class="fixed inset-0 flex items-center justify-center p-4">
          <TransitionChild enter="duration-200 ease-out" enter-from="opacity-0 scale-95" enter-to="opacity-100 scale-100"
                           leave="duration-150 ease-in" leave-from="opacity-100 scale-100" leave-to="opacity-0 scale-95">
            <DialogPanel class="w-full max-w-lg bg-white dark:bg-gray-800 rounded-xl shadow-xl p-6">
              <DialogTitle class="text-lg font-semibold text-gray-900 dark:text-gray-100 mb-4">Create Index</DialogTitle>
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
                <div class="flex justify-end gap-2 pt-2">
                  <button @click="showCreate = false"
                    class="px-4 py-2 text-sm font-medium text-gray-600 dark:text-gray-300 hover:bg-gray-100 dark:hover:bg-gray-700 rounded-lg transition-colors">Cancel</button>
                  <button @click="onCreate" :disabled="createLoading"
                    class="px-4 py-2 text-sm font-medium text-white bg-sky-600 hover:bg-sky-700 disabled:opacity-50 rounded-lg transition-colors">
                    {{ createLoading ? 'Creating...' : 'Create' }}
                  </button>
                </div>
              </div>
            </DialogPanel>
          </TransitionChild>
        </div>
      </Dialog>
    </TransitionRoot>

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
```

- [ ] **Step 3: Verify build**

Run: `cd web && npm run build`
Expected: Build succeeds.

- [ ] **Step 4: Commit**

```bash
git add web/src/views/DashboardView.vue web/src/components/IndexTable.vue
git commit -m "feat(ui): implement Dashboard page with stat cards, index table, and auto-refresh"
```

---

### Task 7: Index Detail Page — Overview Tab

**Files:**
- Create: `web/src/components/SchemaTable.vue`
- Partially replace: `web/src/views/IndexDetailView.vue` (implement overview tab only)

- [ ] **Step 1: Create SchemaTable component**

Create `web/src/components/SchemaTable.vue`:

```vue
<script setup>
defineProps({
  schema: { type: Object, default: () => ({}) },
})
</script>

<template>
  <div v-if="!schema || !schema.fields || !schema.fields.length"
    class="text-sm text-gray-400 dark:text-gray-500 italic">No schema information available.</div>
  <table v-else class="w-full">
    <thead>
      <tr class="text-left text-xs font-medium text-gray-500 dark:text-gray-400 uppercase tracking-wider border-b dark:border-gray-700">
        <th class="pb-2 px-3">Field</th>
        <th class="pb-2 px-3">Type</th>
        <th class="pb-2 px-3">Stored</th>
        <th class="pb-2 px-3">Indexed</th>
        <th class="pb-2 px-3">Fast</th>
      </tr>
    </thead>
    <tbody class="divide-y dark:divide-gray-700">
      <tr v-for="field in schema.fields" :key="field.name" class="hover:bg-gray-50 dark:hover:bg-gray-800/50">
        <td class="py-2 px-3 text-sm font-medium text-gray-900 dark:text-gray-100">{{ field.name }}</td>
        <td class="py-2 px-3 text-sm text-gray-600 dark:text-gray-400">
          <span class="px-1.5 py-0.5 rounded bg-gray-100 dark:bg-gray-700 text-xs font-mono">{{ field.kind }}</span>
        </td>
        <td class="py-2 px-3 text-sm">{{ field.stored ? '✓' : '✗' }}</td>
        <td class="py-2 px-3 text-sm">{{ field.indexed ? '✓' : '✗' }}</td>
        <td class="py-2 px-3 text-sm">{{ field.fast ? '✓' : '✗' }}</td>
      </tr>
    </tbody>
  </table>
</template>
```

- [ ] **Step 2: Implement IndexDetailView with overview tab**

Replace `web/src/views/IndexDetailView.vue` with the initial version containing the header + overview tab. The search, documents, and operations tabs will be added in subsequent tasks:

```vue
<script setup>
import { ref, onMounted, watch } from 'vue'
import { useRouter } from 'vue-router'
import { TabGroup, TabList, Tab, TabPanels, TabPanel } from '@headlessui/vue'
import {
  ArrowLeftIcon, DocumentTextIcon, PuzzlePieceIcon,
  DocumentMagnifyingGlassIcon, DocumentPlusIcon, ListBulletIcon,
  ChartBarIcon, Cog6ToothyIcon as CogIcon, TrashIcon,
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
            <CogIcon class="w-4 h-4" /> Operations
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
```

- [ ] **Step 3: Verify build**

Run: `cd web && npm run build`
Expected: Build succeeds.

- [ ] **Step 4: Commit**

```bash
git add web/src/views/IndexDetailView.vue web/src/components/SchemaTable.vue
git commit -m "feat(ui): implement Index Detail page with overview tab and schema table"
```

---

### Task 8: Index Detail — Search Tab

**Files:**
- Create: `web/src/components/SearchResult.vue`
- Modify: `web/src/views/IndexDetailView.vue` — replace search tab placeholder

- [ ] **Step 1: Create SearchResult component**

Create `web/src/components/SearchResult.vue`:

```vue
<script setup>
import { Disclosure, DisclosureButton, DisclosurePanel } from '@headlessui/vue'
import { ChevronDownIcon } from '@heroicons/vue/24/outline'

defineProps({
  hit: { type: Object, required: true },
  index: { type: Number, required: true },
  rank: { type: Number, default: 0 },
})
</script>

<template>
  <div class="border border-gray-100 dark:border-gray-700 rounded-lg p-3 bg-gray-50/50 dark:bg-gray-800/50">
    <div class="flex items-center justify-between mb-2">
      <span class="text-xs font-semibold text-gray-500 dark:text-gray-400">#{{ rank }}</span>
      <span v-if="hit.score != null" class="text-xs text-gray-400 dark:text-gray-500">score {{ hit.score.toFixed(4) }}</span>
    </div>
    <!-- Parsed fields -->
    <div v-if="hit.doc" class="space-y-1">
      <div v-for="(value, key) in hit.doc" :key="key" class="text-sm">
        <span class="font-medium text-gray-600 dark:text-gray-400">{{ key }}:</span>
        <span class="text-gray-800 dark:text-gray-200 ml-1">{{ value }}</span>
      </div>
    </div>
    <!-- Snippets / highlights -->
    <div v-if="hit.snippets" class="mt-2 space-y-0.5">
      <div v-for="(text, key) in hit.snippets" :key="key" class="text-xs">
        <span class="font-medium text-gray-600 dark:text-gray-400">{{ key }}:</span>
        <span class="text-gray-700 dark:text-gray-300" v-html="Array.isArray(text) ? text.join(' ... ') : text" />
      </div>
    </div>
    <!-- Raw JSON disclosure -->
    <Disclosure class="mt-2" v-slot="{ open }">
      <DisclosureButton class="flex items-center gap-1 text-xs text-gray-400 dark:text-gray-500 hover:text-gray-600 dark:hover:text-gray-300">
        <ChevronDownIcon :class="['w-3 h-3 transition-transform', open && 'rotate-180']" />
        Raw JSON
      </DisclosureButton>
      <DisclosurePanel class="mt-1">
        <pre class="text-xs text-gray-600 dark:text-gray-400 overflow-x-auto bg-white dark:bg-gray-900 rounded p-2">{{ JSON.stringify(hit.doc, null, 2) }}</pre>
      </DisclosurePanel>
    </Disclosure>
  </div>
</template>
```

- [ ] **Step 2: Add search state and search tab to IndexDetailView**

Add these imports and reactive state at the top of the `<script setup>` block in `web/src/views/IndexDetailView.vue`, after the existing `import { getIndexStats } from '../api.js'` line:

```js
import SearchBar from '../components/SearchBar.vue'
import SearchResult from '../components/SearchResult.vue'
import { searchIndex } from '../api.js'

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
```

Replace the search tab placeholder (`<TabPanel>` with "Search tab") in `IndexDetailView.vue` with:

```vue
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
```

- [ ] **Step 3: Verify build**

Run: `cd web && npm run build`
Expected: Build succeeds.

- [ ] **Step 4: Commit**

```bash
git add web/src/views/IndexDetailView.vue web/src/components/SearchResult.vue web/src/components/SearchBar.vue
git commit -m "feat(ui): implement search tab with results display and highlighting"
```

---

### Task 9: Index Detail — Documents Tab

**Files:**
- Create: `web/src/components/DocTable.vue`
- Create: `web/src/components/DocDetail.vue`
- Modify: `web/src/views/IndexDetailView.vue` — replace documents tab placeholder

- [ ] **Step 1: Create DocTable component**

Create `web/src/components/DocTable.vue`:

```vue
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
```

- [ ] **Step 2: Create DocDetail slide-over component**

Create `web/src/components/DocDetail.vue`:

```vue
<script setup>
import { Dialog, DialogPanel, TransitionChild, TransitionRoot } from '@headlessui/vue'
import { XMarkIcon } from '@heroicons/vue/24/outline'

defineProps({
  show: Boolean,
  doc: { type: Object, default: null },
})
const emit = defineEmits(['update:show'])
</script>

<template>
  <TransitionRoot :show="show" as="template">
    <Dialog class="relative z-50" @close="emit('update:show', false)">
      <TransitionChild
        enter="duration-200 ease-out" enter-from="opacity-0" enter-to="opacity-100"
        leave="duration-150 ease-in" leave-from="opacity-100" leave-to="opacity-0"
      >
        <div class="fixed inset-0 bg-black/20" />
      </TransitionChild>
      <div class="fixed inset-0 flex justify-end">
        <TransitionChild
          enter="duration-200 ease-out" enter-from="translate-x-full" enter-to="translate-x-0"
          leave="duration-150 ease-in" leave-from="translate-x-0" leave-to="translate-x-full"
        >
          <DialogPanel class="w-full max-w-lg bg-white dark:bg-gray-800 flex flex-col h-full">
            <div class="flex items-center justify-between px-4 py-3 border-b dark:border-gray-700">
              <span class="text-sm font-semibold text-gray-900 dark:text-gray-100">Document Detail</span>
              <button @click="emit('update:show', false)" class="p-1 text-gray-400 hover:text-gray-600 dark:hover:text-gray-200">
                <XMarkIcon class="w-5 h-5" />
              </button>
            </div>
            <div v-if="doc" class="flex-1 overflow-auto p-4">
              <pre class="text-xs text-gray-700 dark:text-gray-300 whitespace-pre-wrap">{{ JSON.stringify(doc, null, 2) }}</pre>
            </div>
          </DialogPanel>
        </TransitionChild>
      </div>
    </Dialog>
  </TransitionRoot>
</template>
```

- [ ] **Step 3: Add document management state and replace documents tab in IndexDetailView**

Add these imports and reactive state in `IndexDetailView.vue`, after the search-related code:

```js
import DocTable from '../components/DocTable.vue'
import DocDetail from '../components/DocDetail.vue'
import Pagination from '../components/Pagination.vue'
import { listDocs, addDoc, bulkAddDocs } from '../api.js'

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
```

Replace the documents tab placeholder with:

```vue
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
                  ↑ Bulk Import
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

            <!-- Doc Detail Slide-over -->
            <DocDetail v-model:show="showDocDetail" :doc="selectedDoc" />
          </div>
        </TabPanel>
```

Also add the missing imports at the top of the file (after existing heroicon imports):

```js
import { DialogTitle } from '@headlessui/vue'
```

- [ ] **Step 4: Verify build**

Run: `cd web && npm run build`
Expected: Build succeeds.

- [ ] **Step 5: Commit**

```bash
git add web/src/views/IndexDetailView.vue web/src/components/DocTable.vue web/src/components/DocDetail.vue web/src/components/Pagination.vue
git commit -m "feat(ui): implement documents tab with table, add/bulk dialogs, and detail panel"
```

---

### Task 10: Index Detail — Operations Tab

**Files:**
- Modify: `web/src/views/IndexDetailView.vue` — replace operations tab placeholder

- [ ] **Step 1: Add operations state and replace operations tab**

Add these imports and reactive state in `IndexDetailView.vue`, after the document management code:

```js
import ConfirmDialog from '../components/ConfirmDialog.vue'
import { rebuildIndex, compressIndex, deleteIndex } from '../api.js'

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
```

Replace the operations tab placeholder with:

```vue
        <!-- Operations Tab -->
        <TabPanel>
          <div class="space-y-4 max-w-xl">
            <!-- Status message -->
            <div v-if="opsMessage" class="p-3 rounded-lg bg-emerald-50 dark:bg-emerald-900/20 text-sm text-emerald-700 dark:text-emerald-400">{{ opsMessage }}</div>
            <div v-if="opsError" class="p-3 rounded-lg bg-red-50 dark:bg-red-900/20 text-sm text-red-700 dark:text-red-400">{{ opsError }}</div>

            <!-- Maintenance -->
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

            <!-- Danger Zone -->
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
```

Also add `ArrowPathIcon` to the heroicons import line at the top:

```js
import {
  ArrowLeftIcon, ArrowPathIcon, DocumentTextIcon, PuzzlePieceIcon,
  DocumentMagnifyingGlassIcon, DocumentPlusIcon, ListBulletIcon,
  ChartBarIcon, Cog6ToothyIcon as CogIcon, TrashIcon,
} from '@heroicons/vue/24/outline'
```

- [ ] **Step 2: Verify build**

Run: `cd web && npm run build`
Expected: Build succeeds.

- [ ] **Step 3: Commit**

```bash
git add web/src/views/IndexDetailView.vue
git commit -m "feat(ui): implement operations tab with rebuild, compress, and delete"
```

---

### Task 11: Global Search Page

**Files:**
- Replace: `web/src/views/GlobalSearchView.vue`

- [ ] **Step 1: Implement GlobalSearchView**

Replace `web/src/views/GlobalSearchView.vue` with:

```vue
<script setup>
import { ref } from 'vue'
import { useRouter } from 'vue-router'
import { Listbox, ListboxButton, ListboxOption, ListboxOptions, Disclosure, DisclosureButton, DisclosurePanel } from '@headlessui/vue'
import { ChevronDownIcon, MagnifyingGlassIcon } from '@heroicons/vue/24/outline'
import SearchBar from '../components/SearchBar.vue'
import SearchResult from '../components/SearchResult.vue'
import { listIndexes, searchIndex } from '../api.js'

const router = useRouter()

const searchQ = ref('')
const searchLimit = ref(10)
const searchLoading = ref(false)
const selectedIndex = ref(null)
const indexOptions = ref([])
const results = ref([]) // [{ name, result }]

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
    <!-- Hero search -->
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

    <!-- Results grouped by index -->
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
```

- [ ] **Step 2: Verify build**

Run: `cd web && npm run build`
Expected: Build succeeds.

- [ ] **Step 3: Commit**

```bash
git add web/src/views/GlobalSearchView.vue
git commit -m "feat(ui): implement global search page with index filter and grouped results"
```

---

### Task 12: Index List Page

**Files:**
- Replace: `web/src/views/IndexListView.vue`

- [ ] **Step 1: Implement IndexListView**

Replace `web/src/views/IndexListView.vue` with:

```vue
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
      <button @click="showCreate = true"
        class="inline-flex items-center gap-1 bg-sky-600 hover:bg-sky-700 text-white text-xs font-medium px-3 py-1.5 rounded-md transition-colors">
        <PlusIcon class="w-3.5 h-3.5" /> New Index
      </button>
    </div>
    <div class="bg-white dark:bg-gray-800 rounded-xl border border-gray-200 dark:border-gray-700 p-5">
      <p v-if="error" class="text-sm text-red-600 dark:text-red-400 mb-3">{{ error }}</p>
      <IndexTable :indexes="indexes" :loading="loading" @delete="askDelete" />
    </div>

    <!-- Create dialog (same pattern as DashboardView) -->
    <ConfirmDialog v-model:show="showDelete" title="Delete Index"
      :message="`Permanently delete index &quot;${deleteName}&quot;?`" confirm-label="Delete" :danger="true" :loading="deleteLoading"
      @confirm="onConfirmDelete" />
  </div>
</template>
```

Note: The IndexListView reuses IndexTable and ConfirmDialog. For the create dialog, we'll inline a simple version since we want to keep the view self-contained. Add the create dialog HTML (same as DashboardView's create dialog) into the template before `</div>` closing tag.

- [ ] **Step 2: Verify build**

Run: `cd web && npm run build`
Expected: Build succeeds.

- [ ] **Step 3: Commit**

```bash
git add web/src/views/IndexListView.vue
git commit -m "feat(ui): implement index list page with create and delete"
```

---

### Task 13: Settings Page

**Files:**
- Replace: `web/src/views/SettingsView.vue`

- [ ] **Step 1: Implement SettingsView**

Replace `web/src/views/SettingsView.vue` with:

```vue
<script setup>
import { RadioGroup, RadioGroupLabel, RadioGroupOption } from '@headlessui/vue'
import { useTheme } from '../composables/useTheme.js'

const { theme, setTheme } = useTheme()

const themes = [
  { value: 'light', label: 'Light', desc: 'Always use light theme' },
  { value: 'dark', label: 'Dark', desc: 'Always use dark theme' },
  { value: 'system', label: 'System', desc: 'Follow your OS preference' },
]
</script>

<template>
  <div class="p-6 max-w-2xl mx-auto space-y-6">
    <!-- Appearance -->
    <div class="bg-white dark:bg-gray-800 rounded-xl border border-gray-200 dark:border-gray-700 p-5">
      <h3 class="text-sm font-semibold text-gray-900 dark:text-gray-100 mb-4">Appearance</h3>
      <RadioGroup :model-value="theme" @update:model-value="setTheme">
        <RadioGroupLabel class="text-xs font-medium text-gray-600 dark:text-gray-400 mb-2 block">Theme</RadioGroupLabel>
        <div class="space-y-2">
          <RadioGroupOption v-for="t in themes" :key="t.value" :value="t.value" v-slot="{ active, checked }">
            <div :class="[
              'flex items-center gap-3 px-4 py-3 rounded-lg border cursor-pointer transition-colors',
              checked ? 'border-sky-500 bg-sky-50 dark:bg-sky-900/20' : 'border-gray-200 dark:border-gray-600 hover:bg-gray-50 dark:hover:bg-gray-700'
            ]">
              <div :class="[
                'w-4 h-4 rounded-full border-2 flex items-center justify-center',
                checked ? 'border-sky-500' : 'border-gray-300 dark:border-gray-500'
              ]">
                <div v-if="checked" class="w-2 h-2 rounded-full bg-sky-500" />
              </div>
              <div>
                <div class="text-sm font-medium text-gray-900 dark:text-gray-100">{{ t.label }}</div>
                <div class="text-xs text-gray-500 dark:text-gray-400">{{ t.desc }}</div>
              </div>
            </div>
          </RadioGroupOption>
        </div>
      </RadioGroup>
    </div>

    <!-- Server Info -->
    <div class="bg-white dark:bg-gray-800 rounded-xl border border-gray-200 dark:border-gray-700 p-5">
      <h3 class="text-sm font-semibold text-gray-900 dark:text-gray-100 mb-3">Server</h3>
      <dl class="space-y-2 text-sm">
        <div class="flex justify-between">
          <dt class="text-gray-500 dark:text-gray-400">URL</dt>
          <dd class="text-gray-900 dark:text-gray-100 font-mono">{{ window.location.origin }}</dd>
        </div>
        <div class="flex justify-between">
          <dt class="text-gray-500 dark:text-gray-400">Version</dt>
          <dd class="text-gray-900 dark:text-gray-100">0.1.0</dd>
        </div>
      </dl>
    </div>
  </div>
</template>
```

- [ ] **Step 2: Verify build**

Run: `cd web && npm run build`
Expected: Build succeeds.

- [ ] **Step 3: Commit**

```bash
git add web/src/views/SettingsView.vue
git commit -m "feat(ui): implement settings page with theme selector and server info"
```

---

### Task 14: Cleanup Old Files + Final Verification

**Files:**
- Delete: `web/src/views/HomeView.vue`
- Delete: `web/src/views/IndexView.vue`

- [ ] **Step 1: Remove old view files**

```bash
rm web/src/views/HomeView.vue web/src/views/IndexView.vue
```

- [ ] **Step 2: Verify build**

Run: `cd web && npm run build`
Expected: Build succeeds with no errors.

- [ ] **Step 3: Verify no references to old files**

Run: `grep -r "HomeView\|IndexView" web/src/`
Expected: No matches (old imports were only in main.js which was rewritten).

- [ ] **Step 4: Commit**

```bash
git add -A web/src/
git commit -m "chore(ui): remove old HomeView and IndexView files"
```

---

## Self-Review

**1. Spec coverage:**

| Spec Section | Task |
|---|---|
| Navigation & Layout (top bar, sidebar, theme) | Task 1, 3 |
| Theme system (dark/light/system) | Task 2, 13 |
| Dashboard page (cards, table, auto-refresh) | Task 6 |
| Index Detail — Overview tab | Task 7 |
| Index Detail — Search tab | Task 8 |
| Index Detail — Documents tab | Task 9 |
| Index Detail — Operations tab | Task 10 |
| Global Search page | Task 11 |
| Settings page | Task 13 |
| Component architecture (all files) | Tasks 3, 4, 6-13 |
| API usage (all endpoints) | Task 5 |

**2. Placeholder scan:** No TBD, TODO, or "implement later" patterns found.

**3. Type consistency:** All imports and component names are consistent across tasks. API function names match between `api.js` (Task 5) and all view files.

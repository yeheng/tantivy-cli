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

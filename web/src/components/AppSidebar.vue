<script setup>
import { computed } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { Dialog, DialogPanel, TransitionChild, TransitionRoot } from '@headlessui/vue'
import {
  ChartBarIcon, FolderIcon, MagnifyingGlassIcon, Cog6ToothIcon,
  XMarkIcon,
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
  { icon: Cog6ToothIcon, label: 'Settings', to: '/settings' },
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

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

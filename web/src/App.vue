<template>
  <div class="h-full flex flex-col">
    <header class="bg-slate-900 text-white px-6 py-3 flex items-center justify-between shrink-0">
      <div class="flex items-center gap-3">
        <svg class="w-6 h-6 text-sky-400" fill="none" stroke="currentColor" stroke-width="2" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z"/></svg>
        <h1 class="text-lg font-semibold tracking-tight">Tantivy CLI</h1>
      </div>
      <span class="text-xs text-slate-400">Web UI</span>
    </header>
    <main class="flex-1 overflow-hidden">
      <router-view />
    </main>
  </div>
</template>

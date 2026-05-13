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

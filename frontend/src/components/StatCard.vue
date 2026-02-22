<template>
  <div class="card p-4 space-y-1">
    <div class="text-xs text-gray-500 dark:text-gray-400">{{ label }}</div>
    <div class="text-2xl font-bold">{{ value }}</div>
  </div>
</template>
<script setup>
defineProps({ label: String, value: [Number, String] })
</script>

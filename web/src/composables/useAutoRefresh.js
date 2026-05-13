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

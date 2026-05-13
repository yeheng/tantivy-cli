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

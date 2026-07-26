import { ref } from 'vue'
import { defineStore } from 'pinia'
import { startOauth, getLoginStatus, logout, type PixivUser } from '../api'

export const useAuthStore = defineStore('auth', () => {
  const user = ref<PixivUser | null>(null)
  const loading = ref(false)
  const error = ref<string | null>(null)

  async function checkLogin() {
    try {
      const u = await getLoginStatus()
      user.value = u
    } catch {
      user.value = null
    }
  }

  async function login() {
    loading.value = true
    error.value = null
    try {
      const u = await startOauth()
      user.value = u
    } catch (e) {
      error.value = String(e)
    } finally {
      loading.value = false
    }
  }

  function clearError() {
    error.value = null
  }

  async function doLogout() {
    try {
      await logout()
    } catch (e) {
      console.error('Logout failed:', e)
    }
    user.value = null
  }

  return { user, loading, error, checkLogin, login, clearError, doLogout }
})

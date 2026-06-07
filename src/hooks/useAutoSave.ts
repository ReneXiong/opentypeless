import { useEffect, useRef, useCallback } from 'react'
import { useAppStore } from '../stores/appStore'
import { updateConfig, setAutoStart } from '../lib/tauri'

const DEBOUNCE_MS = 800

/**
 * App-level auto-save hook. Must be mounted once in MainApp (lives for entire app lifetime).
 * Watches config vs savedConfig, debounces changes, and persists to disk.
 *
 * Save/error state is stored in Zustand (autoSaveStatus/autoSaveError) so it's
 * accessible from any component (e.g. AutoSaveIndicator in Settings).
 */
export function useAutoSave() {
  const config = useAppStore((s) => s.config)
  const savedConfig = useAppStore((s) => s.savedConfig)
  const setSavedConfig = useAppStore((s) => s.setSavedConfig)
  const setAutoSaveStatus = useAppStore((s) => s.setAutoSaveStatus)
  const retryCount = useAppStore((s) => s.autoSaveRetryCount)

  const timerRef = useRef<ReturnType<typeof setTimeout> | null>(null)
  const configRef = useRef(config)
  const savedConfigRef = useRef(savedConfig)

  // Keep refs in sync via useEffect (not during render)
  useEffect(() => {
    configRef.current = config
  }, [config])

  useEffect(() => {
    savedConfigRef.current = savedConfig
  }, [savedConfig])

  const performSave = useCallback(async () => {
    const latest = configRef.current
    const prev = savedConfigRef.current
    setAutoSaveStatus('saving')
    try {
      await updateConfig(latest)
      // Mark as saved immediately after config persistence succeeds.
      // setAutoStart is a best-effort system-level sync — failure here
      // should not block the saved baseline from advancing.
      setSavedConfig(latest)
      setAutoSaveStatus('idle')
      // Best-effort: sync system auto-start
      if (prev?.auto_start !== latest.auto_start) {
        try {
          await setAutoStart(latest.auto_start)
        } catch (e) {
          console.error('setAutoStart failed (config already saved):', e)
        }
      }
    } catch (e) {
      const msg = e instanceof Error ? e.message : 'Failed to save settings'
      setAutoSaveStatus('error', msg)
    }
  }, [setSavedConfig, setAutoSaveStatus])

  useEffect(() => {
    // No saved snapshot yet (initial load not done)
    if (savedConfig === null) return

    // No change — JSON.stringify works here because AppConfig key order is
    // deterministic (all fields are defined in a fixed interface, no dynamic keys).
    if (JSON.stringify(config) === JSON.stringify(savedConfig)) return

    // Clear previous debounce
    if (timerRef.current) clearTimeout(timerRef.current)

    // Debounce: wait for user to stop changing settings
    timerRef.current = setTimeout(() => {
      performSave()
    }, DEBOUNCE_MS)

    return () => {
      if (timerRef.current) clearTimeout(timerRef.current)
    }
  }, [config, savedConfig, retryCount, performSave])

  // Flush on visibility change (tab hidden) — best-effort immediate save
  useEffect(() => {
    const handleVisibilityChange = () => {
      if (document.visibilityState === 'hidden') {
        // Use getState() to avoid stale closure — this listener is stable
        // across renders, so reading refs or hook state would be outdated.
        const current = useAppStore.getState().config
        const saved = useAppStore.getState().savedConfig
        if (saved && JSON.stringify(current) !== JSON.stringify(saved)) {
          // Clear debounce timer and save immediately
          if (timerRef.current) clearTimeout(timerRef.current)
          performSave()
        }
      }
    }
    document.addEventListener('visibilitychange', handleVisibilityChange)
    return () => document.removeEventListener('visibilitychange', handleVisibilityChange)
  }, [performSave])
}

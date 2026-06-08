import { useEffect } from 'react'
import { listen } from '@tauri-apps/api/event'
import { useTranslation } from 'react-i18next'
import { useAppStore } from '../stores/appStore'
import type { PipelineState } from '../stores/appStore'
import { getHistory } from '../lib/tauri'
import { toast } from '../components/Toast'

export interface StructuredError {
  code: string
  summary?: string
  details?: string
  action?: string
  retryable?: boolean
  retry_count?: number
}

export function useTauriEvents() {
  const { t } = useTranslation()
  const {
    setAudioVolume,
    setPartialTranscript,
    setFinalTranscript,
    appendPolishedChunk,
    setPipelineState,
    setTargetApp,
    setPipelineError,
    setStructuredError,
    setAccessibilityTrusted,
    setHistory,
  } = useAppStore()

  useEffect(() => {
    let cancelled = false
    const unlisteners: Array<() => void> = []

    function addListener<T>(event: string, handler: (payload: T) => void) {
      listen<T>(event, (e) => handler(e.payload))
        .then((unlisten) => {
          if (cancelled) {
            unlisten()
          } else {
            unlisteners.push(unlisten)
          }
        })
        .catch((err) => {
          console.error(`Failed to register listener for "${event}":`, err)
        })
    }

    addListener<number>('audio:volume', setAudioVolume)
    addListener<string>('stt:partial', setPartialTranscript)
    addListener<string>('stt:final', setFinalTranscript)
    addListener<string>('llm:chunk', appendPolishedChunk)
    addListener<PipelineState>('pipeline:state', (state) => {
      setPipelineState(state)
      if (state === 'recording') {
        // Clear any previous error when starting a new pipeline run
        setPipelineError(null)
        setStructuredError(null)
      }
      if (state === 'idle') {
        // Don't clear pipelineError here — CapsuleError auto-resets after 2.5s.
        // Clearing here would swallow errors from failed start() calls that
        // transition Recording → Idle in rapid succession.
        getHistory(200, 0)
          .then(setHistory)
          .catch((err) => {
            console.error('Failed to refresh history:', err)
          })
      }
    })
    addListener<string>('pipeline:target_app', setTargetApp)
    addListener<string | StructuredError>(
      'pipeline:error',
      (payload) => {
        if (typeof payload === 'string') {
          // Legacy string error
          setPipelineError(payload)
          setStructuredError({
            code: 'unknown',
            summary: payload,
            details: undefined,
            action: undefined,
            retryable: false,
            retry_count: 0,
          })
        } else {
          // Structured error
          const structured = payload as StructuredError
          const message = t(`errors.${structured.code}.summary`, {
            defaultValue: structured.summary || structured.code,
            details: structured.details || '',
          })
          setPipelineError(message)
          setStructuredError(structured)
          if (structured.code === 'ACCESSIBILITY_REQUIRED') {
            setAccessibilityTrusted(false)
          }
        }
      },
    )
    addListener<{ code: string; details?: string }>('pipeline:warning', (payload) => {
      const message = t(`errors.${payload.code}.summary`, {
        defaultValue: payload.code,
        details: payload.details ?? '',
      })
      toast(message, 'info')
    })

    addListener<void>('tray:settings', () => {
      window.location.hash = '#/settings'
    })
    addListener<void>('tray:history', () => {
      window.location.hash = '#/history'
    })
    addListener<string>('navigate', (hash) => {
      window.location.hash = hash
    })
    addListener<void>('tray:about', () => {
      window.location.hash = '#/settings'
    })

    return () => {
      cancelled = true
      unlisteners.forEach((unlisten) => unlisten())
    }
  }, [
    setAudioVolume,
    setPartialTranscript,
    setFinalTranscript,
    appendPolishedChunk,
    setPipelineState,
    setTargetApp,
    setPipelineError,
    setStructuredError,
    setAccessibilityTrusted,
    setHistory,
    t,
  ])
}

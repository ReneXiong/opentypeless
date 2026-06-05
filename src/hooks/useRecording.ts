import { useCallback } from 'react'
import { useAppStore } from '../stores/appStore'
import { startRecording as tauriStartRecording, stopRecording as tauriStopRecording } from '../lib/tauri'

export function useRecording() {
  const { pipelineState, resetRecording } = useAppStore()

  const startRecording = useCallback(async () => {
    resetRecording()
    await tauriStartRecording()
  }, [resetRecording])

  const stopRecording = useCallback(async () => {
    await tauriStopRecording()
  }, [])

  const isRecording = pipelineState === 'recording'
  const isProcessing = pipelineState === 'transcribing' || pipelineState === 'polishing'
  const isIdle = pipelineState === 'idle'

  return {
    startRecording,
    stopRecording,
    isRecording,
    isProcessing,
    isIdle,
    pipelineState,
  }
}

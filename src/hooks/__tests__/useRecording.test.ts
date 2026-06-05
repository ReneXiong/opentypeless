import { describe, it, expect, vi, beforeEach } from 'vitest'
import { renderHook, act } from '@testing-library/react'
import { useRecording } from '../useRecording'
import { useAppStore } from '../../stores/appStore'

// Mock the tauri API wrapper module
vi.mock('../../lib/tauri', () => ({
  startRecording: vi.fn().mockResolvedValue(undefined),
  stopRecording: vi.fn().mockResolvedValue(undefined),
  abortRecording: vi.fn().mockResolvedValue(undefined),
}))

// Mock @tauri-apps/api/core to ensure direct invoke calls are NOT used
vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn().mockResolvedValue(undefined),
}))

describe('useRecording', () => {
  beforeEach(() => {
    useAppStore.setState(useAppStore.getInitialState())
    vi.clearAllMocks()
  })

  it('uses startRecording from lib/tauri instead of raw invoke', async () => {
    const { startRecording: tauriStartRecording } = await import('../../lib/tauri')
    const { invoke } = await import('@tauri-apps/api/core')

    const { result } = renderHook(() => useRecording())

    await act(async () => {
      await result.current.startRecording()
    })

    // Should call the wrapper function
    expect(tauriStartRecording).toHaveBeenCalledTimes(1)
    // Should NOT call raw invoke directly
    expect(invoke).not.toHaveBeenCalled()
  })

  it('uses stopRecording from lib/tauri instead of raw invoke', async () => {
    const { stopRecording: tauriStopRecording } = await import('../../lib/tauri')
    const { invoke } = await import('@tauri-apps/api/core')

    const { result } = renderHook(() => useRecording())

    await act(async () => {
      await result.current.stopRecording()
    })

    // Should call the wrapper function
    expect(tauriStopRecording).toHaveBeenCalledTimes(1)
    // Should NOT call raw invoke directly
    expect(invoke).not.toHaveBeenCalled()
  })

  it('returns correct derived state for idle', () => {
    const { result } = renderHook(() => useRecording())

    expect(result.current.isRecording).toBe(false)
    expect(result.current.isProcessing).toBe(false)
    expect(result.current.isIdle).toBe(true)
  })

  it('returns correct derived state for recording', () => {
    useAppStore.getState().setPipelineState('recording')
    const { result } = renderHook(() => useRecording())

    expect(result.current.isRecording).toBe(true)
    expect(result.current.isProcessing).toBe(false)
    expect(result.current.isIdle).toBe(false)
  })

  it('returns correct derived state for transcribing', () => {
    useAppStore.getState().setPipelineState('transcribing')
    const { result } = renderHook(() => useRecording())

    expect(result.current.isRecording).toBe(false)
    expect(result.current.isProcessing).toBe(true)
    expect(result.current.isIdle).toBe(false)
  })

  it('returns correct derived state for polishing', () => {
    useAppStore.getState().setPipelineState('polishing')
    const { result } = renderHook(() => useRecording())

    expect(result.current.isRecording).toBe(false)
    expect(result.current.isProcessing).toBe(true)
    expect(result.current.isIdle).toBe(false)
  })
})

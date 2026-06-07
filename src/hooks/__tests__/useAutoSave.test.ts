import { renderHook, act } from '@testing-library/react'
import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'
import { useAppStore } from '../../stores/appStore'

// Mock lib/tauri
vi.mock('../../lib/tauri', () => ({
  updateConfig: vi.fn().mockResolvedValue(undefined),
  setAutoStart: vi.fn().mockResolvedValue(undefined),
}))

import { useAutoSave } from '../useAutoSave'
import { updateConfig, setAutoStart } from '../../lib/tauri'

const mockUpdateConfig = vi.mocked(updateConfig)
const mockSetAutoStart = vi.mocked(setAutoStart)

describe('useAutoSave', () => {
  beforeEach(() => {
    vi.useFakeTimers()
    useAppStore.setState(useAppStore.getInitialState())
    mockUpdateConfig.mockClear()
    mockSetAutoStart.mockClear()
    mockUpdateConfig.mockResolvedValue(undefined)
    mockSetAutoStart.mockResolvedValue(undefined)
  })

  afterEach(() => {
    vi.useRealTimers()
  })

  it('should not save when config matches savedConfig', async () => {
    const base = { stt_provider: 'deepgram', auto_start: false }
    useAppStore.setState({ config: base as any, savedConfig: base as any })

    renderHook(() => useAutoSave())

    await act(async () => {
      await vi.advanceTimersByTimeAsync(2000)
    })

    expect(mockUpdateConfig).not.toHaveBeenCalled()
  })

  it('should debounce and save after 800ms', async () => {
    const base = { stt_provider: 'deepgram', auto_start: false }
    useAppStore.setState({ config: base as any, savedConfig: base as any })

    renderHook(() => useAutoSave())

    // Change config
    act(() => {
      useAppStore.setState({ config: { ...base, stt_provider: 'assemblyai' } as any })
    })

    // Should not save yet at 500ms
    await act(async () => {
      await vi.advanceTimersByTimeAsync(500)
    })
    expect(mockUpdateConfig).not.toHaveBeenCalled()

    // Should save after 800ms
    await act(async () => {
      await vi.advanceTimersByTimeAsync(400)
    })
    expect(mockUpdateConfig).toHaveBeenCalledWith(
      expect.objectContaining({ stt_provider: 'assemblyai' })
    )
  })

  it('should call setAutoStart when auto_start changes', async () => {
    const base = { stt_provider: 'deepgram', auto_start: false }
    useAppStore.setState({ config: base as any, savedConfig: base as any })

    renderHook(() => useAutoSave())

    act(() => {
      useAppStore.setState({ config: { ...base, auto_start: true } as any })
    })

    await act(async () => {
      await vi.advanceTimersByTimeAsync(1000)
    })

    expect(mockUpdateConfig).toHaveBeenCalled()
    expect(mockSetAutoStart).toHaveBeenCalledWith(true)
  })

  it('should not call setAutoStart when auto_start unchanged', async () => {
    const base = { stt_provider: 'deepgram', auto_start: false }
    useAppStore.setState({ config: base as any, savedConfig: base as any })

    renderHook(() => useAutoSave())

    act(() => {
      useAppStore.setState({ config: { ...base, stt_provider: 'assemblyai' } as any })
    })

    await act(async () => {
      await vi.advanceTimersByTimeAsync(1000)
    })

    expect(mockUpdateConfig).toHaveBeenCalled()
    expect(mockSetAutoStart).not.toHaveBeenCalled()
  })

  it('should set error status when save fails', async () => {
    mockUpdateConfig.mockRejectedValue(new Error('disk full'))

    const base = { stt_provider: 'deepgram' }
    useAppStore.setState({ config: base as any, savedConfig: base as any })

    renderHook(() => useAutoSave())

    act(() => {
      useAppStore.setState({ config: { ...base, stt_provider: 'assemblyai' } as any })
    })

    await act(async () => {
      await vi.advanceTimersByTimeAsync(1000)
    })

    const store = useAppStore.getState()
    expect(store.autoSaveStatus).toBe('error')
    expect(store.autoSaveError).toBe('disk full')
  })

  it('should update savedConfig after successful save', async () => {
    const base = { stt_provider: 'deepgram' }
    useAppStore.setState({ config: base as any, savedConfig: base as any })

    renderHook(() => useAutoSave())

    act(() => {
      useAppStore.setState({ config: { ...base, stt_provider: 'assemblyai' } as any })
    })

    await act(async () => {
      await vi.advanceTimersByTimeAsync(1000)
    })

    const store = useAppStore.getState()
    expect(store.savedConfig).toEqual(expect.objectContaining({ stt_provider: 'assemblyai' }))
    expect(store.autoSaveStatus).toBe('idle')
  })

  it('should reset debounce on rapid changes', async () => {
    const base = { stt_provider: 'deepgram' }
    useAppStore.setState({ config: base as any, savedConfig: base as any })

    renderHook(() => useAutoSave())

    // Rapid changes at 0ms, 200ms, 400ms, 600ms
    act(() => {
      useAppStore.setState({ config: { ...base, stt_provider: 'a' } as any })
    })
    await act(async () => {
      await vi.advanceTimersByTimeAsync(200)
    })
    act(() => {
      useAppStore.setState({ config: { ...base, stt_provider: 'b' } as any })
    })
    await act(async () => {
      await vi.advanceTimersByTimeAsync(200)
    })
    act(() => {
      useAppStore.setState({ config: { ...base, stt_provider: 'c' } as any })
    })
    await act(async () => {
      await vi.advanceTimersByTimeAsync(200)
    })
    act(() => {
      useAppStore.setState({ config: { ...base, stt_provider: 'd' } as any })
    })

    // Only 600ms since first change, should not have saved yet
    expect(mockUpdateConfig).not.toHaveBeenCalled()

    // After 800ms from last change, should save with final value
    await act(async () => {
      await vi.advanceTimersByTimeAsync(900)
    })

    expect(mockUpdateConfig).toHaveBeenCalledTimes(1)
    expect(mockUpdateConfig).toHaveBeenCalledWith(
      expect.objectContaining({ stt_provider: 'd' })
    )
  })

  it('should keep savedConfig unchanged when save fails', async () => {
    mockUpdateConfig.mockRejectedValue(new Error('network error'))

    const base = { stt_provider: 'deepgram' }
    useAppStore.setState({ config: base as any, savedConfig: base as any })

    renderHook(() => useAutoSave())

    act(() => {
      useAppStore.setState({ config: { ...base, stt_provider: 'assemblyai' } as any })
    })

    await act(async () => {
      await vi.advanceTimersByTimeAsync(1000)
    })

    // savedConfig should remain at the old value
    expect(useAppStore.getState().savedConfig).toEqual(base)
  })
})

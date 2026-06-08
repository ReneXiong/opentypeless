import { useEffect, useState, useCallback } from 'react'
import { motion } from 'framer-motion'
import { useTranslation } from 'react-i18next'
import { useAppStore } from '../../stores/appStore'
import { ErrorDetailPopup } from '../ErrorDetailPopup'

export function CapsuleError() {
  const { t } = useTranslation()
  const pipelineError = useAppStore((s) => s.pipelineError)
  const structuredError = useAppStore((s) => s.structuredError)
  const setPipelineError = useAppStore((s) => s.setPipelineError)
  const setStructuredError = useAppStore((s) => s.setStructuredError)
  const resetRecording = useAppStore((s) => s.resetRecording)
  const [expanded, setExpanded] = useState(false)
  const [dismissTimer, setDismissTimer] = useState<ReturnType<typeof setTimeout> | null>(null)

  const handleDismiss = useCallback(() => {
    setExpanded(false)
    setPipelineError(null)
    setStructuredError(null)
    const currentState = useAppStore.getState().pipelineState
    if (currentState === 'idle') {
      resetRecording()
    }
  }, [setPipelineError, setStructuredError, resetRecording])

  const handleClick = useCallback(() => {
    if (structuredError) {
      setExpanded((prev) => !prev)
      // Pause auto-dismiss when expanded
      if (dismissTimer) {
        clearTimeout(dismissTimer)
        setDismissTimer(null)
      }
    }
  }, [structuredError, dismissTimer])

  useEffect(() => {
    if (!expanded) {
      const timer = setTimeout(() => {
        handleDismiss()
      }, 2500)
      setDismissTimer(timer)
      return () => clearTimeout(timer)
    }
  }, [expanded, handleDismiss, pipelineError])

  const handleRetry = useCallback(() => {
    handleDismiss()
    // Trigger retry by emitting a custom event
    window.dispatchEvent(new CustomEvent('pipeline:retry'))
  }, [handleDismiss])

  const handleOpenSettings = useCallback(() => {
    handleDismiss()
    window.location.hash = '#/settings'
  }, [handleDismiss])

  return (
    <motion.div
      className="relative z-10 flex items-center gap-2 h-9 px-3 cursor-pointer"
      initial={{ opacity: 0, x: -4 }}
      animate={{ opacity: 1, x: 0 }}
      transition={{ duration: 0.3, ease: 'easeOut' }}
      onClick={handleClick}
    >
      {/* White dot */}
      <motion.div className="w-2 h-2 rounded-full bg-white/80 flex-shrink-0" />
      <p className="text-[11px] text-white truncate flex-1">
        {pipelineError === 'ACCESSIBILITY_REQUIRED'
          ? t('capsule.accessibilityRequired')
          : pipelineError || t('capsule.errorOccurred')}
      </p>

      {/* Error Detail Popup */}
      {expanded && structuredError && (
        <ErrorDetailPopup
          error={structuredError}
          onDismiss={handleDismiss}
          onRetry={structuredError.retryable ? handleRetry : undefined}
          onOpenSettings={handleOpenSettings}
        />
      )}
    </motion.div>
  )
}

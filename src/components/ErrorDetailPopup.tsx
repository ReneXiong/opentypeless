import { motion, AnimatePresence } from 'framer-motion'
import { useTranslation } from 'react-i18next'
import { X, Copy, RefreshCw, Settings, AlertCircle } from 'lucide-react'
import type { StructuredError } from '../stores/appStore'

interface ErrorDetailPopupProps {
  error: StructuredError
  onDismiss: () => void
  onRetry?: () => void
  onOpenSettings?: () => void
}

export function ErrorDetailPopup({ error, onDismiss, onRetry, onOpenSettings }: ErrorDetailPopupProps) {
  const { t } = useTranslation()

  const handleCopy = () => {
    const text = [
      `Error: ${error.code}`,
      error.summary && `Summary: ${error.summary}`,
      error.details && `Details: ${error.details}`,
      error.action && `Action: ${error.action}`,
    ]
      .filter(Boolean)
      .join('\n')
    navigator.clipboard.writeText(text)
  }

  return (
    <AnimatePresence>
      <motion.div
        className="absolute bottom-full left-0 right-0 mb-2 z-50"
        initial={{ opacity: 0, y: 10, scale: 0.95 }}
        animate={{ opacity: 1, y: 0, scale: 1 }}
        exit={{ opacity: 0, y: 10, scale: 0.95 }}
        transition={{ duration: 0.2, ease: 'easeOut' }}
      >
        <div className="bg-gray-900/95 backdrop-blur-xl rounded-2xl shadow-2xl border border-white/10 p-4 min-w-[280px] max-w-[360px]">
          {/* Header */}
          <div className="flex items-start justify-between gap-3 mb-3">
            <div className="flex items-center gap-2">
              <AlertCircle className="w-5 h-5 text-red-400 flex-shrink-0" />
              <h3 className="text-sm font-medium text-white">
                {error.summary || t('errors.default.summary', 'Error')}
              </h3>
            </div>
            <button
              onClick={onDismiss}
              className="p-1 rounded-lg hover:bg-white/10 transition-colors"
            >
              <X className="w-4 h-4 text-white/60" />
            </button>
          </div>

          {/* Details */}
          {error.details && (
            <div className="mb-3 p-2.5 bg-white/5 rounded-lg">
              <p className="text-xs text-white/70 font-mono break-all">{error.details}</p>
            </div>
          )}

          {/* Action */}
          {error.action && (
            <p className="text-xs text-white/80 mb-3 leading-relaxed">{error.action}</p>
          )}

          {/* Action Buttons */}
          <div className="flex items-center gap-2">
            {error.retryable && onRetry && (
              <button
                onClick={onRetry}
                className="flex items-center gap-1.5 px-3 py-1.5 bg-white/10 hover:bg-white/20 rounded-lg text-xs text-white transition-colors"
              >
                <RefreshCw className="w-3.5 h-3.5" />
                {t('errors.retry', 'Retry')}
              </button>
            )}
            {onOpenSettings && (
              <button
                onClick={onOpenSettings}
                className="flex items-center gap-1.5 px-3 py-1.5 bg-white/10 hover:bg-white/20 rounded-lg text-xs text-white transition-colors"
              >
                <Settings className="w-3.5 h-3.5" />
                {t('errors.openSettings', 'Settings')}
              </button>
            )}
            <button
              onClick={handleCopy}
              className="flex items-center gap-1.5 px-3 py-1.5 bg-white/10 hover:bg-white/20 rounded-lg text-xs text-white transition-colors ml-auto"
            >
              <Copy className="w-3.5 h-3.5" />
              {t('errors.copy', 'Copy')}
            </button>
          </div>
        </div>
      </motion.div>
    </AnimatePresence>
  )
}

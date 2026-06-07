import { motion, AnimatePresence } from 'framer-motion'
import { Loader2, AlertCircle, RefreshCw } from 'lucide-react'

interface Props {
  saving: boolean
  error: string | null
  onRetry: () => void
}

export function AutoSaveIndicator({ saving, error, onRetry }: Props) {
  const show = saving || error !== null

  return (
    <AnimatePresence>
      {show && (
        <motion.div
          className={`flex items-center justify-between px-5 py-2.5 ${
            error
              ? 'bg-error/10 border-t border-error/20'
              : 'bg-bg-tertiary/50 border-t border-border'
          }`}
          initial={{ y: 20, opacity: 0 }}
          animate={{ y: 0, opacity: 1 }}
          exit={{ y: 20, opacity: 0 }}
          transition={{ type: 'spring', stiffness: 400, damping: 30 }}
        >
          {saving && (
            <span className="flex items-center gap-1.5 text-text-secondary text-[12px]">
              <motion.div
                animate={{ rotate: 360 }}
                transition={{ repeat: Infinity, duration: 0.8, ease: 'linear' }}
              >
                <Loader2 size={12} />
              </motion.div>
              Saving...
            </span>
          )}
          {error && (
            <>
              <span className="flex items-center gap-1.5 text-error text-[12px] truncate mr-3">
                <AlertCircle size={12} />
                {error}
              </span>
              <button
                onClick={onRetry}
                className="flex items-center gap-1 px-2.5 py-1 text-[11px] text-text-secondary hover:text-text-primary bg-transparent border border-border cursor-pointer rounded-[8px] hover:bg-bg-tertiary transition-colors"
              >
                <RefreshCw size={10} />
                Retry
              </button>
            </>
          )}
        </motion.div>
      )}
    </AnimatePresence>
  )
}

import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { useAppStore } from '../../stores/appStore'
import { SettingsSidebar, type PaneId } from './SettingsSidebar'
import { GeneralPane } from './GeneralPane'
import { SttPane } from './SttPane'
import { LlmPane } from './LlmPane'
import { DictionaryPane } from './DictionaryPane'
import { ScenesPane } from './ScenesPane'
import { AboutPane } from './AboutPane'
import { AutoSaveIndicator } from './shared/AutoSaveIndicator'

const paneTitleKeys: Record<PaneId, string> = {
  general: 'settings.general',
  stt: 'settings.speechRecognition',
  llm: 'settings.aiPolish',
  dictionary: 'settings.dictionary',
  scenes: 'settings.scenes',
  about: 'settings.about',
}

export function Settings() {
  const [activePane, setActivePane] = useState<PaneId>('general')
  const autoSaveStatus = useAppStore((s) => s.autoSaveStatus)
  const autoSaveError = useAppStore((s) => s.autoSaveError)
  const retryAutoSave = useAppStore((s) => s.retryAutoSave)
  const { t } = useTranslation()

  return (
    <div className="w-full flex-1 bg-bg-primary text-text-primary flex flex-col min-h-0">
      <div className="flex-1 flex min-h-0">
        {/* Sidebar */}
        <SettingsSidebar activePane={activePane} onSelect={setActivePane} />

        {/* Content */}
        <div className="flex-1 flex flex-col min-w-0">
          {/* Title bar */}
          <div className="flex items-center justify-between px-6 pt-4 pb-3 border-b border-border bg-bg-primary/50">
            <h2 className="text-[15px] font-medium">{t(paneTitleKeys[activePane])}</h2>
          </div>

          {/* Pane content */}
          <div className="flex-1 overflow-y-auto px-6 py-5 min-h-0">
            {activePane === 'general' && <GeneralPane />}
            {activePane === 'stt' && <SttPane />}
            {activePane === 'llm' && <LlmPane />}
            {activePane === 'dictionary' && <DictionaryPane />}
            {activePane === 'scenes' && <ScenesPane />}
            {activePane === 'about' && <AboutPane />}
          </div>
        </div>
      </div>

      {/* Auto-save indicator */}
      <AutoSaveIndicator
        saving={autoSaveStatus === 'saving'}
        error={autoSaveError}
        onRetry={retryAutoSave}
      />
    </div>
  )
}

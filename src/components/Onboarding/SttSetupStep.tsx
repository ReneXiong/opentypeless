import { useAppStore } from '../../stores/appStore'
import { STT_PROVIDERS } from '../../lib/constants'
import { testSttConnection } from '../../lib/tauri'
import { CheckCircle2, XCircle, Loader2 } from 'lucide-react'

export function SttSetupStep() {
  const config = useAppStore((s) => s.config)
  const updateConfig = useAppStore((s) => s.updateConfig)
  const sttTestStatus = useAppStore((s) => s.sttTestStatus)
  const setSttTestStatus = useAppStore((s) => s.setSttTestStatus)

  const handleTest = async () => {
    setSttTestStatus('testing')
    try {
      const ok = await testSttConnection(config.stt_api_key, config.stt_provider)
      setSttTestStatus(ok ? 'success' : 'error')
    } catch {
      setSttTestStatus('error')
    }
  }

  return (
    <div className="space-y-5">
      <Field label="Speech Recognition Service">
        <select
          value={config.stt_provider}
          onChange={(e) => {
            const newProvider = e.target.value as typeof config.stt_provider
            const newKeys = { ...config.stt_api_keys }
            newKeys[config.stt_provider] = config.stt_api_key
            updateConfig({
              stt_provider: newProvider,
              stt_api_key: newKeys[newProvider] ?? '',
              stt_api_keys: newKeys,
            })
            setSttTestStatus('idle')
          }}
          className="w-full px-3 py-2.5 bg-bg-secondary border border-border rounded-[10px] text-[13px] text-text-primary outline-none focus:border-border-focus transition-colors"
        >
          {STT_PROVIDERS.map((p) => (
            <option key={p.value} value={p.value}>
              {p.label}
            </option>
          ))}
        </select>
      </Field>

      <Field label="API Key">
        <div className="flex gap-2">
          <input
            type="password"
            value={config.stt_api_key}
            onChange={(e) => {
              const newKeys = { ...config.stt_api_keys, [config.stt_provider]: e.target.value }
              updateConfig({ stt_api_key: e.target.value, stt_api_keys: newKeys })
              setSttTestStatus('idle')
            }}
            placeholder="Enter API Key..."
            className="flex-1 px-3 py-2.5 bg-bg-secondary border border-border rounded-[10px] text-[13px] text-text-primary outline-none focus:border-border-focus transition-colors"
          />
          <button
            onClick={handleTest}
            disabled={!config.stt_api_key || sttTestStatus === 'testing'}
            className="px-4 py-2.5 bg-accent text-white rounded-[10px] text-[13px] border-none cursor-pointer hover:bg-accent-hover disabled:opacity-40 disabled:cursor-not-allowed transition-colors flex items-center gap-1.5"
          >
            {sttTestStatus === 'testing' && <Loader2 size={14} className="animate-spin" />}
            Test
          </button>
        </div>
        <TestStatusHint status={sttTestStatus} />
      </Field>
    </div>
  )
}

function Field({ label, children }: { label: string; children: React.ReactNode }) {
  return (
    <div>
      <label className="block text-[13px] font-medium text-text-secondary mb-2">{label}</label>
      {children}
    </div>
  )
}

function TestStatusHint({ status }: { status: string }) {
  if (status === 'success') {
    return (
      <p className="flex items-center gap-1 text-[12px] text-success mt-2">
        <CheckCircle2 size={13} /> Connection successful
      </p>
    )
  }
  if (status === 'error') {
    return (
      <p className="flex items-center gap-1 text-[12px] text-error mt-2">
        <XCircle size={13} /> Connection failed, please check your API Key
      </p>
    )
  }
  return null
}

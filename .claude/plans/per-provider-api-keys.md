# Plan: Per-Provider API Key Storage

## Problem

`AppConfig` stores a single `stt_api_key` and `llm_api_key`. When switching providers, the key field doesn't update — you're left with the previous provider's key and must manually delete/replace it. This is especially painful for users who switch between providers frequently.

## Solution

Store API keys per-provider using a map (`Record<Provider, string>`). When switching providers, automatically load the saved key for the new provider. When editing a key, save it for the current provider.

## Files to Modify

| File | Change |
|------|--------|
| `src-tauri/src/storage/mod.rs` | Add `stt_api_keys` / `llm_api_keys` map fields to `AppConfig`, add migration from old fields |
| `src/stores/appStore.ts` | Add map fields to TypeScript `AppConfig` interface, update `defaultConfig` |
| `src/components/Settings/SttPane.tsx` | On provider switch: save current key to map, load new provider's key. On key edit: sync to map. |
| `src/components/Settings/LlmPane.tsx` | Same as SttPane — save/load key on switch, sync on edit. |
| `src-tauri/src/pipeline.rs` | Read key from map using provider name as key, fall back to flat field for migration. |
| `src/components/Onboarding/index.tsx` | Ensure onboarding writes keys to the map (not just flat field). |

## Implementation Details

### 1. Backend: `storage/mod.rs`

Add two new fields to `AppConfig`:

```rust
pub stt_api_keys: std::collections::HashMap<String, String>,
pub llm_api_keys: std::collections::HashMap<String, String>,
```

In `Default`, initialize both as empty maps.

**Migration**: In `ConfigManager::load()`, after deserializing, if `stt_api_keys` is empty but `stt_api_key` is non-empty, insert `stt_api_key` into `stt_api_keys` under the current `stt_provider`. Same for LLM. This ensures existing users' keys are preserved on first launch.

Keep the old `stt_api_key` / `llm_api_key` fields for backward compat (they won't be removed, just supplemented).

### 2. Frontend: `appStore.ts`

Add to `AppConfig` interface:

```typescript
stt_api_keys: Record<string, string>
llm_api_keys: Record<string, string>
```

Initialize as `{}` in `defaultConfig`.

### 3. Frontend: `SttPane.tsx` / `LlmPane.tsx` — Provider Switch Logic

On provider `<select>` change:

```typescript
// Save current provider's key to map, load new provider's key
const newKeys = { ...config.stt_api_keys }
newKeys[config.stt_provider] = config.stt_api_key  // save current
const newKey = newKeys[newProvider] ?? ''            // load new
updateConfig({
  stt_provider: newProvider,
  stt_api_key: newKey,
  stt_api_keys: newKeys,
})
```

On API key `<input>` change:

```typescript
const newKeys = { ...config.stt_api_keys, [config.stt_provider]: e.target.value }
updateConfig({ stt_api_key: e.target.value, stt_api_keys: newKeys })
```

Same pattern for LLM (with `llm_*` fields).

### 4. Backend: `pipeline.rs`

When reading the API key, look up from the map first, fall back to the flat field:

```rust
let stt_api_key = config_data.stt_api_keys
    .get(&config_data.stt_provider)
    .filter(|k| !k.is_empty())
    .cloned()
    .unwrap_or_else(|| config_data.stt_api_key.clone());
```

Same for LLM key lookup.

### 5. Onboarding: `index.tsx`

The onboarding flow writes keys via `updateConfig({ stt_api_key: ... })`. This still works because the flat field remains. But we should also write to the map:

```typescript
updateConfig({
  stt_api_key: key,
  stt_api_keys: { ...config.stt_api_keys, [provider]: key },
})
```

## Migration Strategy

- Existing users: On first load after update, `ConfigManager::load()` detects empty maps and migrates the flat `stt_api_key`/`llm_api_key` into the corresponding map entry.
- New users: Maps start empty, keys are added as they configure providers.
- The flat fields are kept (not removed) so rolling back to an older version doesn't lose keys.

## Testing

1. Fresh install: configure a provider, switch to another, enter key, switch back — key should be preserved.
2. Existing install: upgrade, keys should appear correctly for the previously configured provider.
3. Onboarding flow: keys entered during onboarding should be saved to the map.

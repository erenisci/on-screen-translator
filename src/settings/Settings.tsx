import { useCallback, useEffect, useState } from 'react';
import {
  getSettings,
  hasProviderKey,
  listOcrLanguages,
  saveSettings,
  setAutostart,
  setHotkey,
  setProviderKey,
  testProvider,
  toAppError,
} from '../lib/ipc';
import type { AppError, ProviderId, Settings } from '../lib/types';

/**
 * One small window, four groups (F9). Every field applies live — nothing here
 * requires a restart (FR-60), so there is no Save button by design.
 *
 * Settings are persisted on field change rather than on close, so nothing is
 * lost if the window is closed abruptly or the app quits.
 */
export function SettingsWindow() {
  const [settings, setSettings] = useState<Settings | null>(null);
  const [error, setError] = useState<AppError | null>(null);
  const [ocrLanguages, setOcrLanguages] = useState<string[]>([]);

  useEffect(() => {
    getSettings()
      .then(setSettings)
      .catch((thrown: unknown) => setError(toAppError(thrown)));
    listOcrLanguages()
      .then(setOcrLanguages)
      .catch(() => setOcrLanguages([]));
  }, []);

  /** Optimistic, then reconciled with what the core actually stored. */
  const patch = useCallback((changes: Partial<Settings>) => {
    setSettings((previous) => {
      if (!previous) return previous;
      const next = { ...previous, ...changes };
      saveSettings(next)
        .then(setSettings)
        .catch((thrown: unknown) => setError(toAppError(thrown)));
      return next;
    });
  }, []);

  if (!settings) {
    return (
      <div className="p-4 text-sm opacity-70">{error ? error.message : 'Loading settings…'}</div>
    );
  }

  return (
    <div className="flex flex-col gap-5 p-4 text-sm">
      {error ? (
        <div className="rounded bg-amber-500/15 px-2 py-1.5 text-xs">
          <p>{error.message}</p>
          {error.action ? <p className="mt-0.5 opacity-70">{error.action}</p> : null}
        </div>
      ) : null}

      <Group title="Language">
        <Field label="Translate into">
          <input
            type="text"
            value={settings.targetLanguage}
            onChange={(e) => patch({ targetLanguage: e.target.value })}
            className={inputClass}
            placeholder="tr"
          />
        </Field>
        <Field label="Read from">
          <input
            type="text"
            value={settings.sourceLanguage ?? ''}
            onChange={(e) => patch({ sourceLanguage: e.target.value.trim() || null })}
            className={inputClass}
            placeholder="Auto-detect"
          />
        </Field>
        {/* FR-34: surfaced here, before a capture fails. */}
        <p className="text-xs opacity-60">
          {ocrLanguages.length > 0
            ? `Windows can recognize: ${ocrLanguages.join(', ')}`
            : 'No OCR language packs detected. Add one in Windows language settings.'}
        </p>
      </Group>

      <Group title="Capture">
        <HotkeyField current={settings.hotkey} onChanged={(hotkey) => patch({ hotkey })} />
        <Field label="Show result as">
          <select
            value={settings.resultDisplayMode}
            onChange={(e) =>
              patch({ resultDisplayMode: e.target.value as Settings['resultDisplayMode'] })
            }
            className={inputClass}
          >
            <option value="both">Overlay and panel</option>
            <option value="overlay">Overlay only</option>
            <option value="panel">Panel only</option>
          </select>
        </Field>
      </Group>

      <Group title="Translation">
        <Field label="Provider">
          <select
            value={settings.provider}
            onChange={(e) => patch({ provider: e.target.value as ProviderId })}
            className={inputClass}
          >
            <option value="libretranslate">LibreTranslate (no key needed)</option>
            <option value="deepl">DeepL</option>
            <option value="google">Google Translate</option>
            <option value="llm">LLM endpoint</option>
          </select>
        </Field>
        <ProviderKeyField provider={settings.provider} />
        {settings.provider === 'llm' ? (
          <>
            <Field label="Endpoint">
              <input
                type="url"
                value={settings.llmEndpoint ?? ''}
                onChange={(e) => patch({ llmEndpoint: e.target.value.trim() || null })}
                className={inputClass}
                placeholder="https://…"
              />
            </Field>
            <Field label="Model">
              <input
                type="text"
                value={settings.llmModel ?? ''}
                onChange={(e) => patch({ llmModel: e.target.value.trim() || null })}
                className={inputClass}
              />
            </Field>
          </>
        ) : null}
      </Group>

      <Group title="General">
        <Toggle
          label="Start with Windows"
          checked={settings.startWithWindows}
          onChange={(enabled) => {
            // The core returns the state read back from the registry, not the
            // request — another tool may have removed the entry (F2).
            setAutostart(enabled)
              .then((actual) => setSettings((s) => (s ? { ...s, startWithWindows: actual } : s)))
              .catch((thrown: unknown) => setError(toAppError(thrown)));
          }}
        />
        <Field label="Theme">
          <select
            value={settings.theme}
            onChange={(e) => patch({ theme: e.target.value as Settings['theme'] })}
            className={inputClass}
          >
            <option value="system">Follow system</option>
            <option value="light">Light</option>
            <option value="dark">Dark</option>
          </select>
        </Field>
      </Group>
    </div>
  );
}

const inputClass =
  'w-52 rounded border border-black/15 bg-transparent px-2 py-1 dark:border-white/15';

function Group({ title, children }: { title: string; children: React.ReactNode }) {
  return (
    <section className="flex flex-col gap-2">
      <h2 className="text-xs font-semibold uppercase tracking-wide opacity-50">{title}</h2>
      {children}
    </section>
  );
}

function Field({ label, children }: { label: string; children: React.ReactNode }) {
  return (
    <label className="flex items-center justify-between gap-3">
      <span className="opacity-80">{label}</span>
      {children}
    </label>
  );
}

function Toggle({
  label,
  checked,
  onChange,
}: {
  label: string;
  checked: boolean;
  onChange: (checked: boolean) => void;
}) {
  return (
    <label className="flex items-center justify-between gap-3">
      <span className="opacity-80">{label}</span>
      <input type="checkbox" checked={checked} onChange={(e) => onChange(e.target.checked)} />
    </label>
  );
}

/**
 * FR-12: a conflicting binding must fail visibly and keep the previous one.
 * The core is the authority on what is actually registered, so we render what
 * it reports rather than what the user typed.
 */
function HotkeyField({
  current,
  onChanged,
}: {
  current: string;
  onChanged: (hotkey: string) => void;
}) {
  const [draft, setDraft] = useState(current);
  const [message, setMessage] = useState<string | null>(null);

  useEffect(() => setDraft(current), [current]);

  const commit = useCallback(() => {
    if (draft === current) return;
    setHotkey(draft)
      .then((result) => {
        setDraft(result.active);
        setMessage(result.ok ? null : result.message);
        if (result.ok) onChanged(result.active);
      })
      .catch((thrown: unknown) => setMessage(toAppError(thrown).message));
  }, [draft, current, onChanged]);

  return (
    <div className="flex flex-col gap-1">
      <Field label="Hotkey">
        <input
          type="text"
          value={draft}
          onChange={(e) => setDraft(e.target.value)}
          onBlur={commit}
          className={inputClass}
          placeholder="Ctrl+Shift+T"
        />
      </Field>
      {message ? <p className="text-xs text-amber-600">{message}</p> : null}
    </div>
  );
}

/**
 * The key never round-trips into the webview (invariant 7, FR-62): we ask the
 * core whether one is set, and only ever write.
 */
function ProviderKeyField({ provider }: { provider: ProviderId }) {
  const [isSet, setIsSet] = useState(false);
  const [draft, setDraft] = useState('');
  const [status, setStatus] = useState<string | null>(null);

  useEffect(() => {
    setDraft('');
    setStatus(null);
    hasProviderKey(provider)
      .then(setIsSet)
      .catch(() => setIsSet(false));
  }, [provider]);

  if (provider === 'libretranslate') return null;

  const save = () => {
    if (draft === '') return;
    setProviderKey(provider, draft)
      .then(() => {
        setIsSet(true);
        setDraft('');
        setStatus('Key saved.');
      })
      .catch((thrown: unknown) => setStatus(toAppError(thrown).message));
  };

  return (
    <div className="flex flex-col gap-1">
      <Field label="API key">
        <input
          type="password"
          value={draft}
          onChange={(e) => setDraft(e.target.value)}
          onBlur={save}
          className={inputClass}
          placeholder={isSet ? '•••••••• (saved)' : 'Paste your key'}
        />
      </Field>
      <div className="flex items-center gap-2 text-xs">
        <button
          type="button"
          onClick={() =>
            testProvider(provider)
              .then((health) => setStatus(health.message))
              .catch((thrown: unknown) => setStatus(toAppError(thrown).message))
          }
          className="rounded bg-black/10 px-2 py-0.5 hover:bg-black/20 dark:bg-white/10 dark:hover:bg-white/20"
        >
          Test connection
        </button>
        {status ? <span className="opacity-70">{status}</span> : null}
      </div>
    </div>
  );
}

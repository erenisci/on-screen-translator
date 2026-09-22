import { useCallback, useEffect, useState } from 'react';
import {
  closePanel,
  copyToClipboard,
  getPanelResult,
  onResult,
  retranslate,
  toAppError,
} from '../lib/ipc';
import type { AppError, TranslateResult } from '../lib/types';

/**
 * The copyable companion to the overlay (F8).
 *
 * Deliberately independent of the overlay: closing one does not close the other,
 * so a translation can stay on screen while the user re-selects.
 */
export function Panel() {
  const [result, setResult] = useState<TranslateResult | null>(null);
  const [error, setError] = useState<AppError | null>(null);
  const [busy, setBusy] = useState(false);

  useEffect(() => {
    getPanelResult()
      .then(setResult)
      .catch((thrown: unknown) => setError(toAppError(thrown)));

    const pending = onResult((next) => {
      setResult(next);
      setError(null);
    });
    return () => {
      void pending.then((unlisten) => unlisten());
    };
  }, []);

  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key === 'Escape') void closePanel().catch(() => undefined);
    };
    window.addEventListener('keydown', onKeyDown);
    return () => window.removeEventListener('keydown', onKeyDown);
  }, []);

  /** FR-54: reuses the cached OCR text — no re-capture, no re-OCR. */
  const handleRetranslate = useCallback((target: string) => {
    setBusy(true);
    retranslate(target)
      .then((next) => {
        setResult(next);
        setError(null);
      })
      // The existing translation stays on screen; the failure is inline (F8).
      .catch((thrown: unknown) => setError(toAppError(thrown)))
      .finally(() => setBusy(false));
  }, []);

  if (!result) {
    return (
      <div className="flex h-full items-center justify-center p-4 text-xs opacity-60">
        {error ? error.message : 'Waiting for a capture…'}
      </div>
    );
  }

  return (
    <div className="flex h-full flex-col gap-3 p-3 text-sm">
      <header className="flex items-center justify-between text-xs opacity-70">
        <span className="tabular-nums">
          {result.sourceLang} → {result.targetLang}
        </span>
        <button
          type="button"
          onClick={() => void closePanel().catch(() => undefined)}
          className="rounded px-1.5 py-0.5 hover:bg-black/10 dark:hover:bg-white/10"
          aria-label="Close"
        >
          ✕
        </button>
      </header>

      {error ? (
        <div className="rounded bg-amber-500/15 px-2 py-1.5 text-xs">
          <p>{error.message}</p>
          {error.action ? <p className="mt-0.5 opacity-70">{error.action}</p> : null}
        </div>
      ) : null}

      {/* FR-44: even when translation failed, the extracted source stays here, copyable. */}
      <TextBlock label="Translation" text={result.fullTranslation} emphasis />
      <TextBlock label="Source" text={result.fullSource} />

      <footer className="mt-auto flex items-center gap-2 text-xs">
        <RetranslateControl
          current={result.targetLang}
          disabled={busy}
          onRetranslate={handleRetranslate}
        />
        {!result.alignmentOk ? (
          <span
            className="ml-auto opacity-60"
            title="Translated segments could not be matched to the original lines"
          >
            in-place unavailable
          </span>
        ) : null}
      </footer>
    </div>
  );
}

function TextBlock({ label, text, emphasis }: { label: string; text: string; emphasis?: boolean }) {
  const [copied, setCopied] = useState(false);

  const handleCopy = useCallback(() => {
    copyToClipboard(text)
      .then(() => {
        setCopied(true);
        window.setTimeout(() => setCopied(false), 1200);
      })
      .catch(() => undefined);
  }, [text]);

  return (
    <section className="flex min-h-0 flex-1 flex-col gap-1">
      <div className="flex items-center justify-between text-xs opacity-60">
        <span>{label}</span>
        <button
          type="button"
          onClick={handleCopy}
          className="rounded px-1.5 py-0.5 hover:bg-black/10 dark:hover:bg-white/10"
        >
          {copied ? 'Copied' : 'Copy'}
        </button>
      </div>
      {/* Scrolls rather than growing the window past a sane height (F8). */}
      <div
        className={`otr-selectable min-h-0 flex-1 overflow-y-auto whitespace-pre-wrap rounded bg-black/5 p-2 dark:bg-white/5 ${
          emphasis ? '' : 'opacity-75'
        }`}
      >
        {text}
      </div>
    </section>
  );
}

/** Deliberately short: the full language list belongs in Settings, not here. */
const QUICK_TARGETS = ['tr', 'en', 'de', 'es', 'fr', 'ru', 'ar', 'ja', 'zh'];

function RetranslateControl({
  current,
  disabled,
  onRetranslate,
}: {
  current: string;
  disabled: boolean;
  onRetranslate: (target: string) => void;
}) {
  return (
    <label className="flex items-center gap-1.5">
      <span className="opacity-60">Translate to</span>
      <select
        value={QUICK_TARGETS.includes(current) ? current : ''}
        disabled={disabled}
        onChange={(event) => onRetranslate(event.target.value)}
        className="rounded border border-black/15 bg-transparent px-1 py-0.5 dark:border-white/15"
      >
        {!QUICK_TARGETS.includes(current) ? <option value="">{current}</option> : null}
        {QUICK_TARGETS.map((tag) => (
          <option key={tag} value={tag}>
            {tag}
          </option>
        ))}
      </select>
    </label>
  );
}

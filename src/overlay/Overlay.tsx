import { useCallback, useEffect, useMemo, useState } from 'react';
import {
  cancelCapture,
  closeOverlay,
  getCaptureFrame,
  onProgress,
  toAppError,
  translateRegion,
} from '../lib/ipc';
import { cssToPhysicalRect, isUsableSelection, type OverlayView } from '../lib/coords';
import { SelectionLayer } from './SelectionLayer';
import { ResultLayer } from './ResultLayer';
import type { AppError, CaptureFrameInfo, Rect, TranslateResult } from '../lib/types';

type Phase =
  | { kind: 'loading' }
  | { kind: 'selecting' }
  | { kind: 'working'; rect_css: Rect }
  | { kind: 'result'; rect_css: Rect; result: TranslateResult }
  | { kind: 'failed'; rect_css: Rect | null; error: AppError };

/** NFR-P2: a spinner on every sub-second capture is worse than none. */
const PROGRESS_DELAY_MS = 300;

export function Overlay() {
  const [frame, setFrame] = useState<CaptureFrameInfo | null>(null);
  const [phase, setPhase] = useState<Phase>({ kind: 'loading' });
  const [showProgress, setShowProgress] = useState(false);

  const view: OverlayView | null = useMemo(
    () => (frame ? { virtualBounds: frame.virtualBounds, scaleFactor: frame.scaleFactor } : null),
    [frame],
  );

  /* ------------------------------------------------------------ teardown -- */

  // NFR-R1: the user is never trapped behind a full-screen window. Dismissal
  // must work from every phase, including `failed`.
  const dismiss = useCallback(() => {
    void cancelCapture()
      .catch(() => undefined)
      .finally(() => void closeOverlay().catch(() => undefined));
  }, []);

  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key === 'Escape') dismiss();
    };
    window.addEventListener('keydown', onKeyDown);
    return () => window.removeEventListener('keydown', onKeyDown);
  }, [dismiss]);

  /* --------------------------------------------------------------- setup -- */

  useEffect(() => {
    let cancelled = false;
    getCaptureFrame()
      .then((info) => {
        if (cancelled) return;
        setFrame(info);
        setPhase({ kind: 'selecting' });
      })
      .catch((thrown: unknown) => {
        if (cancelled) return;
        setPhase({ kind: 'failed', rect_css: null, error: toAppError(thrown) });
      });
    return () => {
      cancelled = true;
    };
  }, []);

  useEffect(() => {
    const pending = onProgress(() => setShowProgress(true));
    return () => {
      void pending.then((unlisten) => unlisten());
    };
  }, []);

  /* ------------------------------------------------------------ pipeline -- */

  const handleSelectionStart = useCallback(() => {
    // FR-26: a new drag clears the previous result and returns to selecting.
    setShowProgress(false);
    setPhase({ kind: 'selecting' });
  }, []);

  const handleSelected = useCallback(
    (rect_css: Rect) => {
      if (!view) return;

      // THE conversion. Physical pixels from here down — nothing rescales again.
      const rect_physical = cssToPhysicalRect(rect_css, view);

      // FR-25: a mis-click is not a request. Stay in selecting, say nothing.
      if (!isUsableSelection(rect_physical)) {
        setPhase({ kind: 'selecting' });
        return;
      }

      setPhase({ kind: 'working', rect_css });

      const progressTimer = window.setTimeout(() => setShowProgress(true), PROGRESS_DELAY_MS);

      translateRegion(rect_physical)
        .then((result) => setPhase({ kind: 'result', rect_css, result }))
        .catch((thrown: unknown) =>
          setPhase({ kind: 'failed', rect_css, error: toAppError(thrown) }),
        )
        .finally(() => {
          window.clearTimeout(progressTimer);
          setShowProgress(false);
        });
    },
    [view],
  );

  /* ---------------------------------------------------------------- view -- */

  if (phase.kind === 'failed' && !frame) {
    return <FatalError error={phase.error} onDismiss={dismiss} />;
  }

  if (!frame || !view) return null;

  // The frozen frame is rendered at 1:1 physical pixels: its CSS size is the
  // virtual desktop divided by this window's scale factor (ADR-0004).
  const frameWidth_css = frame.virtualBounds.w / frame.scaleFactor;
  const frameHeight_css = frame.virtualBounds.h / frame.scaleFactor;

  const frozenSelection_css =
    phase.kind === 'working' || phase.kind === 'result'
      ? phase.rect_css
      : phase.kind === 'failed'
        ? phase.rect_css
        : null;

  // TD-06: when segments could not be matched back onto OCR lines, the panel is
  // the only honest surface. A translation on the wrong button is worse than none.
  const inPlaceLines =
    phase.kind === 'result' && phase.result.alignmentOk ? phase.result.lines : [];

  return (
    <div className="relative h-full w-full overflow-hidden">
      <img
        src={frame.frameUrl}
        alt=""
        draggable={false}
        className="absolute left-0 top-0 max-w-none select-none"
        style={{ width: frameWidth_css, height: frameHeight_css }}
      />

      <SelectionLayer
        enabled={phase.kind !== 'working'}
        frozenSelection_css={frozenSelection_css}
        onSelectionStart={handleSelectionStart}
        onSelected={handleSelected}
      />

      {inPlaceLines.length > 0 ? <ResultLayer lines={inPlaceLines} view={view} /> : null}

      {phase.kind === 'working' && showProgress ? <Working rect_css={phase.rect_css} /> : null}

      {phase.kind === 'failed' && phase.rect_css ? (
        <InlineError rect_css={phase.rect_css} error={phase.error} />
      ) : null}

      <Hint />
    </div>
  );
}

function Working({ rect_css }: { rect_css: Rect }) {
  return (
    <div
      className="pointer-events-none absolute flex items-center justify-center"
      style={{ left: rect_css.x, top: rect_css.y, width: rect_css.w, height: rect_css.h }}
    >
      <span className="rounded bg-black/75 px-3 py-1.5 text-xs text-white">Reading…</span>
    </div>
  );
}

/**
 * FR-35 / FR-44: an error keeps the overlay open so the user can re-select, and
 * always names an action when there is one.
 */
function InlineError({ rect_css, error }: { rect_css: Rect; error: AppError }) {
  return (
    <div
      className="absolute max-w-sm rounded-md bg-zinc-900/95 px-3 py-2 text-xs text-white shadow-lg"
      style={{ left: rect_css.x, top: rect_css.y + rect_css.h + 8 }}
    >
      <p>{error.message}</p>
      {error.action ? <p className="mt-1 text-zinc-300">{error.action}</p> : null}
    </div>
  );
}

/** The frame never arrived, so there is nothing to select over — offer the way out. */
function FatalError({ error, onDismiss }: { error: AppError; onDismiss: () => void }) {
  return (
    <div className="flex h-full w-full items-center justify-center bg-black/70">
      <div className="max-w-md rounded-lg bg-zinc-900 px-5 py-4 text-sm text-white shadow-xl">
        <p>{error.message}</p>
        {error.action ? <p className="mt-2 text-zinc-300">{error.action}</p> : null}
        <button
          type="button"
          onClick={onDismiss}
          className="mt-4 rounded bg-white/10 px-3 py-1.5 text-xs hover:bg-white/20"
        >
          Close (Esc)
        </button>
      </div>
    </div>
  );
}

function Hint() {
  return (
    <div className="pointer-events-none absolute bottom-4 left-1/2 -translate-x-1/2 rounded-full bg-black/60 px-3 py-1 text-xs text-white/90">
      Drag to select · Esc to cancel
    </div>
  );
}

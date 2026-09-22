# on-screen-translator

**Press a key, drag a box, read it in your language — without leaving what you're doing.**

A free, open-source Windows tool that lives in your system tray. Trigger it with a hotkey, select any region of your
screen exactly like Lightshot, and the text inside is recognized and translated — then drawn **back over the
original text**, where the words actually were, without leaving the capture overlay.

Built for anyone who reads a language they don't speak on screen: game UIs, installers, error dialogs, scanned PDFs,
subtitles, screenshots, remote desktop sessions — all the text you can't select and can't paste into a translator.

> ### ⚠️ Not usable yet
>
> **There is no release to download.** The architecture and documentation are complete and the frontend is built
> and tested, but the Rust core — tray, hotkey, screen capture, OCR, translation — is not implemented yet, so the
> app does not run end to end.
>
> Current state: [docs/progress.md](docs/progress.md) · Plan: [docs/project/roadmap.md](docs/project/roadmap.md)

## Planned features

What v1 is being built to do. None of it works yet — see the notice above.

- **One-keypress capture** — a global hotkey freezes the screen; drag to select, just like Lightshot
- **Read it where it was** — the translation is drawn over the original text blocks, not dumped into a text box
- **Keep going** — select another region without pressing the hotkey again
- **Your language** — the target language defaults to your Windows display language; nothing is hardcoded to English
- **Copyable** — an always-on-top panel with the source text and the translation, each with a copy button
- **Tray-only** — no taskbar entry, no window to manage. Optionally starts with Windows
- **Fast and light** — under a second for a paragraph, ~30 MB of RAM while idle, nothing running until you press the key
- **Multi-monitor and mixed-DPI correct** — a 150%-scaled laptop next to a 100% external monitor works properly

## Privacy

This app registers a global hotkey and captures your screen, so it should tell you exactly what it does with that.
These are design commitments, enforced in the architecture rather than promised in a policy:

- **Your screen never leaves your machine.** OCR runs locally using the OCR engine built into Windows. No image is
  ever uploaded.
- **Only the extracted text is sent**, only to the translation provider *you* chose, only when you ask for a
  translation.
- **No telemetry.** No analytics, no crash reporting, no update pings. The app makes no outbound request you didn't
  initiate.
- **Nothing captured is written to disk** — not images, not text, not in the logs.
- **Your API keys** are stored in Windows Credential Manager, never in a config file and never in a log.

All network access is confined to one module so you can verify this yourself rather than take our word for it.
Details: [docs/operations/security.md](docs/operations/security.md).

## How it will work

```
hotkey → capture the whole virtual desktop once (the screen freezes)
       → you drag a selection
       → crop from that frame (no re-capture)
       → pre-process → OCR locally (Windows.Media.Ocr)
       → one batched translation request
       → draw the result over the original text
```

**Tauri v2** with a **Rust core** and a **React + TypeScript + Tailwind** frontend. The Rust side owns the hotkey,
capture, OCR, translation, and secrets; the frontend only draws. Electron was ruled out because a resident Chromium
costs roughly 8× the idle memory budget for an app that sits idle 99% of the time.

Translation providers are pluggable — **LibreTranslate** (keyless), **DeepL**, **Google Translate**, or any
**OpenAI-compatible LLM endpoint** — always with your own key. No key ships with the app and there is no backend.

Architecture: [docs/architecture/overview.md](docs/architecture/overview.md) · The four load-bearing decisions and
their trade-offs: [docs/architecture/adr/](docs/architecture/adr/README.md)

## Building from source

### Requirements

| Tool                     | Notes                                                             |
| ------------------------ | ------------------------------------------------------------------- |
| Windows 10 (1809+) or 11 | `Windows.Media.Ocr` needs 1809 or newer                            |
| Node.js 20+              | Frontend build                                                     |
| Rust (stable, MSVC)      | `winget install Rustlang.Rustup`                                   |
| VS Build Tools 2022      | "Desktop development with C++" workload — provides the MSVC linker |
| WebView2 runtime         | Already present on Windows 11 and most Windows 10 installs         |

```powershell
winget install Rustlang.Rustup
rustup default stable-x86_64-pc-windows-msvc
winget install Microsoft.VisualStudio.2022.BuildTools `
  --override "--quiet --wait --add Microsoft.VisualStudio.Workload.VCTools --includeRecommended"
```

### Run

```bash
git clone https://github.com/erenisci/on-screen-translator
cd on-screen-translator
npm install
npm run tauri dev      # not available yet — src-tauri/ is not implemented
```

What works today:

```bash
npm test          # 66 unit tests (coordinate math, text fitting)
npm run typecheck
npm run lint
npm run build     # builds the three windows
```

Full setup and the things that will confuse you in the first hour:
[docs/onboarding.md](docs/onboarding.md).

## Project Structure

See [docs/engineering/project-structure.md](docs/engineering/project-structure.md).

## Documentation

This project is documentation-first: the docs are the source of truth and were written before the code.
Full index in [docs/](docs/README.md). Key entry points:

- Product & scope → [docs/product/prd.md](docs/product/prd.md)
- Architecture → [docs/architecture/overview.md](docs/architecture/overview.md)
- Setup & first hour → [docs/onboarding.md](docs/onboarding.md)
- Roadmap → [docs/project/roadmap.md](docs/project/roadmap.md)
- Current status → [docs/progress.md](docs/progress.md)

## Contributing

Issues and pull requests are welcome. Before opening a PR, please read
[docs/onboarding.md](docs/onboarding.md) — especially the "five things that will confuse you" section — and walk
[docs/engineering/self-review-checklist.md](docs/engineering/self-review-checklist.md).

One thing worth knowing up front: **anything touching screen capture, coordinates, or windows must be tested on a
multi-monitor setup with different scaling factors.** That is where this project's worst bugs live, and a
single-monitor test cannot find them.

## License

[MIT](LICENSE)

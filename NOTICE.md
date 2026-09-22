# Third-party notices

This CityWalk Meetings build is derived from two MIT-licensed projects. The
copyright notices and license terms of each must be retained in distributions.

## Meetily (Community Edition)

Copyright (c) 2024 Zackriya Solutions

See `LICENSE.md`.

## OpenWhispr

Copyright (c) 2024 OpenWhispr Team

Vendored as a git submodule at `vendor/openwhispr` (MIT, see
`vendor/openwhispr/LICENSE`).

CityWalk Meetings does **not** ship the OpenWhispr Electron shell, Neon cloud,
or dictation hotkeys. The following source is reused as the specification for
the Tauri cloud-STT providers under `frontend/src-tauri/src/audio/transcription/`:

- `vendor/openwhispr/src/helpers/transcriptionRoute.ts` — Groq default model
  `whisper-large-v3-turbo` and Deepgram `nova-*` model ids
- `vendor/openwhispr/src/helpers/audioManager.js` — multipart POST to
  `{base}/audio/transcriptions` (Groq / OpenAI-compatible)
- `vendor/openwhispr/src/helpers/deepgramStreaming.js` — Deepgram `Token`
  authorization for BYOK keys
- `vendor/openwhispr/src/config/constants.ts` — Groq base URL
  `https://api.groq.com/openai/v1`

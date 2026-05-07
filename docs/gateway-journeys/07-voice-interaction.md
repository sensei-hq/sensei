---
name: Voice Interaction
description: Speak a coding instruction → transcribe → reason → respond aloud — hands-free coding assistant
date: 2026-04-24
status: idea
---

# Journey: Voice Interaction

## Scenario

A developer is reading through a complex codebase. Their hands are on keyboard navigating code, but they want to ask questions verbally instead of typing.

## Pipeline

```
Microphone → STT chain → text prompt → Chat chain → text response → TTS chain → Speaker
```

Three gateway chains, composed sequentially.

## What happens

### 1. Voice captured

Developer speaks: "What does the refresh token handler do?"

Desktop app captures audio via microphone, streams to the gateway.

### 2. STT chain transcribes

```
gateway.execute(InferenceRequest {
  capability: VoiceStt,
  payload: SttPayload {
    audio: AudioInput::Stream(mic_stream),
    language: Some("en"),
  },
})

  → stt_chain: whisper:large-v3 (ollama)
  → POST http://localhost:11434/v1/audio/transcriptions
  → Transcription: "What does the refresh token handler do?"
  → Duration: 180ms (faster than real-time)
  → Cost: $0.00 (local)
```

### 3. Chat chain reasons

```
gateway.execute(InferenceRequest {
  capability: Chat,
  payload: ChatPayload {
    messages: [{ role: user, content: "What does the refresh token handler do?" }],
    system: "You are a code analysis assistant. Context: ...",
  },
})

  → chat_chain: claude-sonnet-4-6 (anthropic)
  → Streaming response: "The refresh token handler in auth/token_manager.ts
     manages the OAuth2 token refresh flow. When an access token expires..."
  → Duration: 1,200ms
  → Cost: $0.008
```

### 4. TTS chain speaks

```
gateway.execute(InferenceRequest {
  capability: VoiceTts,
  payload: TtsPayload {
    text: "The refresh token handler in auth/token_manager.ts...",
    voice: "nova",
    speed: 1.1,
  },
})

  → tts_chain: piper-en-us (local-tts)
  → Audio generated: 8.2 seconds of speech
  → Duration: 95ms generation time
  → Cost: $0.00 (local)
```

### 5. Developer hears the answer

Total round-trip: ~1.5 seconds from end-of-speech to start-of-audio.

Developer's hands never left the keyboard. Eyes stayed on code. Brain received the explanation through a different channel — audio — while visually processing the code.

## Streaming optimization

For lower latency, TTS can start before chat finishes:

```
Chat streaming:     "The refresh─ token handler─ in auth/─ ..."
                         │              │            │
TTS streaming:      [audio chunk 1] [chunk 2]  [chunk 3] ...
                         │              │            │
Speaker:            🔊──────────────────────────────────>
```

First audio plays ~500ms after first chat tokens arrive, not after the full response.

## Use cases

| Scenario | STT | Chat | TTS |
|----------|-----|------|-----|
| Ask about code | Microphone | Claude/local | Speaker |
| Dictate a commit message | Microphone | — (pass-through) | — |
| Listen to PR summary | — | Local model | Speaker |
| Voice-controlled navigation | Microphone | Command parser | — |
| Pair programming narration | — | — | Speaker reads code aloud |

## Hardware requirements

| Component | Minimum | Recommended |
|-----------|---------|-------------|
| STT (Whisper large-v3) | 2GB VRAM | Apple Silicon M1+ |
| TTS (Piper) | CPU only | Any modern CPU |
| Both running + Ollama | 8GB unified memory | 16GB+ |

## Open questions

| # | Question |
|---|----------|
| 1 | Should voice be push-to-talk or always-listening with VAD? |
| 2 | How to handle ambient noise in a coding environment? |
| 3 | Should voice responses be interruptible? ("stop, go back to the part about...") |
| 4 | Multi-language support: detect language automatically or require explicit setting? |

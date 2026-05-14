---
name: Voice Chain
description: Speech-to-text and text-to-speech chain — Whisper for transcription, TTS models for synthesis, streaming audio support
date: 2026-04-24
status: idea
related: 06-chain-chat.md, 02-adapter-system.md
---

# Voice Chain

## Problem

Voice is a natural interface for coding:
- **Dictate intent** while hands are on keyboard — "refactor this to use the builder pattern"
- **Listen to explanations** while reading code — eyes on screen, ears on context
- **Accessibility** — voice input/output for developers who need it
- **Ambient coding** — voice commands during whiteboard sessions, standups, pair programming

The gateway needs to route voice through the same chain/fallback infrastructure as text.

## Two sub-chains

Voice is bidirectional — two distinct pipelines:

### STT (Speech-to-Text) chain

```yaml
stt_chain:
  capability: voice_stt
  fallback_triggers: [timeout, model_unavailable, provider_error]
  models:
    - model: whisper:large-v3
      router: ollama                 # local whisper via Ollama
      priority: 1
    - model: whisper-1
      router: openai                 # OpenAI Whisper API
      priority: 2
```

### TTS (Text-to-Speech) chain

```yaml
tts_chain:
  capability: voice_tts
  fallback_triggers: [timeout, model_unavailable, provider_error]
  models:
    - model: piper-en-us
      router: local-tts              # local Piper TTS
      priority: 1
    - model: tts-1
      router: openai                 # OpenAI TTS API
      priority: 2
    - model: tts-1-hd
      router: openai                 # higher quality, slower
      priority: 3
```

## STT: Speech-to-Text

### Input formats
- **File upload** — wav, mp3, m4a, webm → transcribe whole file
- **Streaming** — real-time microphone input → incremental transcription

### Streaming transcription

```rust
pub struct SttRequest {
    pub audio: AudioInput,           // File or Stream
    pub language: Option<String>,    // "en", "ja", etc.
    pub prompt: Option<String>,      // context hint for better accuracy
}

pub enum AudioInput {
    File(Vec<u8>),
    Stream(Pin<Box<dyn Stream<Item = Vec<u8>> + Send>>),
}

pub struct SttResponse {
    pub text: String,
    pub segments: Vec<TranscriptSegment>,  // timestamped segments
    pub language: String,
    pub duration_secs: f32,
}

pub struct TranscriptSegment {
    pub start: f32,
    pub end: f32,
    pub text: String,
    pub confidence: f32,
}
```

### Use cases for STT
- Voice-to-prompt: speak a coding instruction → transcribe → send to chat chain
- Meeting transcription: record a standup → structured notes
- Code review narration: dictate review comments while reading code

## TTS: Text-to-Speech

### Output formats
- **Full synthesis** — text → complete audio buffer
- **Streaming** — text → incremental audio chunks for real-time playback

### API

```rust
pub struct TtsRequest {
    pub text: String,
    pub voice: Option<String>,       // "alloy", "nova", etc.
    pub speed: Option<f32>,          // 0.5 - 2.0
    pub format: AudioFormat,         // mp3, wav, opus
}

pub enum AudioFormat {
    Mp3,
    Wav,
    Opus,
    Pcm,
}

pub struct TtsResponse {
    pub audio: Vec<u8>,
    pub format: AudioFormat,
    pub duration_secs: f32,
}
```

### Use cases for TTS
- Read explanations aloud while user reads code
- Narrate code reviews
- Accessibility: audio output for visually impaired developers

## Local voice models

### Whisper (STT)
- Available via Ollama or whisper.cpp directly
- Models: tiny, base, small, medium, large-v3
- large-v3 is best quality but needs ~2GB VRAM
- Real-time factor: large-v3 on M4 Max ≈ 0.1x (10x faster than real-time)

### Piper (TTS)
- Lightweight local TTS engine
- Multiple voices per language
- Runs on CPU, no GPU needed
- Lower quality than OpenAI TTS but fully offline

### Future: Bark, XTTS
- Higher quality local TTS options as they mature
- Voice cloning potential (XTTS)

## Voice + Chat pipeline

The natural flow combines STT → Chat → TTS:

```
Microphone → STT chain → text prompt
    → Chat chain → text response
    → TTS chain → speaker
```

This creates a voice-first coding assistant. The gateway handles all three chains independently — they compose naturally.

## Latency budget

For real-time voice interaction, total round-trip must be < 3 seconds:

| Step | Target | Local | External |
|------|--------|-------|----------|
| STT | < 500ms | ~200ms (whisper large-v3) | ~800ms (OpenAI) |
| Chat | < 2000ms | ~1500ms (gemma3:27b) | ~500ms (claude-sonnet) |
| TTS | < 500ms | ~100ms (piper) | ~300ms (OpenAI) |
| **Total** | **< 3000ms** | **~1800ms** | **~1600ms** |

Local STT + External chat + Local TTS may be the optimal mix.

## Open questions

| # | Question |
|---|----------|
| 1 | Should voice be a Phase 1 feature or later? It's high-impact but adds significant complexity. |
| 2 | Should we support voice during streaming chat? (user speaks while model is still responding) |
| 3 | What's the minimum hardware for acceptable local STT? whisper-small on an M1 MacBook Air? |
| 4 | Should the gateway handle audio format conversion (e.g. webm → wav) or require the caller to normalize? |
| 5 | How do we handle background noise, coding-environment audio (keyboard clicks, fan noise)? |
| 6 | Should voice commands have a wake word ("hey sensei") or always-listen with VAD (voice activity detection)? |

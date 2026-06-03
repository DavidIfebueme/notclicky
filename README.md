# notclicky

clicky but for linux.

a voice-first desktop companion that sees your screen, points at things, talks back, and runs background agents — all native, all local-first, all rust + gtk4.

## what it does

- wake word or push-to-talk voice input → whisper transcription → llm streaming response → tts playback
- cursor overlay that points at stuff on your screen (`[POINT:x,y:label]` from any llm)
- screen capture on demand — screenshot goes straight to the llm context
- background agents that spawn opencode cli sessions and stream output live
- skill system — 37 linux-applicable skills loaded from bundled markdown
- memory that persists across sessions (wiki, conversation history, memory.md)
- external control bridge on port 32123
- system tray + settings ui + chat panel + mini chat

## the pipeline

```
push-to-talk → pipewire capture → whisper.cpp transcription
  → speculative pre-fire to llm (streaming)
  → [POINT:x,y] parsed mid-stream → overlay renders instantly
  → sentence-chunked streaming tts → audio plays before llm finishes
```

target: 500-800ms push-to-talk release → voice + overlay

## build

```bash
cargo build
cargo test
```

you'll need gtk4, libadwaita, pipewire, and x11 dev libraries. on arch:

```bash
sudo pacman -S gtk4 libadwaita pipewire libx11 xcb-util-keysyms
```

## config

- `~/.config/notclicky/config.toml` — provider selection, model, voice settings

## status

phases 0-11 done. packaging next.

see [AGENTS.md](AGENTS.md) for the full build checklist.

## shoutouts

- [jason kneen](https://github.com/jasonkneen) — openclicky, the original that inspired this
- [farza](https://x.com/FarzaTV) — for the energy around making ai companions that feel real

## license

mit

# notclicky

clicky but for linux.

a voice-first desktop companion that sees your screen, points at things, talks back, and runs background agents — all native, all local-first, all rust + gtk4.

not a fork. not a port. a ground-up reimplementation of [openclicky](https://github.com/jasonkneen/openclicky) for linux.

## what it does

- push-to-talk voice input → whisper transcription → llm streaming response → tts playback
- cursor overlay that points at stuff on your screen (`[POINT:x,y:label]` from any llm)
- screen capture on demand — screenshot goes straight to the llm context
- background agents that spawn opencode cli sessions and stream output live
- skill system — 37 linux-applicable skills loaded from bundled markdown
- memory that persists across sessions (wiki, conversation history, memory.md)
- external control bridge on port 32123 — same api as openclicky, so existing mcp tools work
- system tray + settings ui + chat panel + mini chat

## the pipeline

```
push-to-talk → pipewire capture → whisper.cpp transcription
  → speculative pre-fire to llm (streaming)
  → [POINT:x,y] parsed mid-stream → overlay renders instantly
  → sentence-chunked streaming tts → audio plays before llm finishes
```

target: 500-800ms push-to-talk release → voice + overlay

## tech

- **ui**: gtk4 + libadwaita
- **voice**: pipewire capture, whisper.cpp (whisper-rs), edge tts (default, free)
- **llm**: openai-compatible streaming, anthropic messages api, ollama local
- **overlay**: x11 transparent window (wayland degrades to notifications)
- **capture**: x11 xgetimage (wayland uses xdg-desktop-portal)
- **hotkeys**: x11 xkb (wayland uses portal globalshortcuts)
- **bridge**: axum http server, mcp protocol, inference proxy
- **agents**: opencode cli subprocess, json streaming

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
- `~/.config/notclicky/secrets.env` — api keys (chmod 600, never committed)

## status

phases 0-10 done. wayland support (degraded) in place. packaging next.

see [AGENTS.md](AGENTS.md) for the full build checklist.

## shoutouts

this project wouldn't exist without [jason kneen's openclicky](https://github.com/jasonkneen/openclicky) — we studied every feature and behavior, then rewrote it all for linux from scratch. the soul, the skills, the bridge api contract — all originally jason's work.

also shoutout to [farza](https://x.com/farazsth15) for the inspiration and energy around making ai companions that actually feel like companions.

## license

mit

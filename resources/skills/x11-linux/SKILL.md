---
name: x11-linux
description: "X11/Linux: window management, screen capture, hotkeys, desktop integration via xdotool, xclip, notify-send, xdg-open."
version: 1.0.0
author: NotClicky
license: MIT
prerequisites:
  commands: [xdotool, xclip, notify-send, xdg-open]
metadata:
  notclicky:
    tags: [X11, Linux, Desktop, Window Management, Clipboard, Notifications]
---

## NotClicky compatibility guardrails

- Verify required local commands are available before promising execution.
- Treat sends, publishes, deploys, deletes, moves, merges, and app-control clicks as external writes.
- Stop and report the exact missing setup step for unavailable tools; do not loop or silently switch to browser automation.

# X11/Linux — Desktop Integration

Manage windows, capture screens, handle clipboard, send notifications, and interact with the Linux desktop via standard X11 tools.

## Window Management

```bash
xdotool search --name "Firefox" windowactivate
xdotool getactivewindow getwindowname
xdotool getactivewindow windowmove 0 0 windowsize 800 600
xdotool search --onlyvisible --name "Terminal" windowactivate --sync
```

## Clipboard

```bash
echo "text" | xclip -selection clipboard
xclip -selection clipboard -o
```

## Notifications

```bash
notify-send "Title" "Body"
notify-send -u critical "Alert" "Something important"
```

## Default Applications

```bash
xdg-open https://example.com
xdg-open document.pdf
```

## Key Simulation

```bash
xdotool key ctrl+c
xdotool type --delay 50 "hello world"
xdotool mousemove 500 300 click 1
```

## Screen Information

```bash
xdpyinfo | grep dimensions
xrandr --query | grep " connected"
xprop -id $(xdotool getactivewindow) _NET_WM_NAME
```

## Active Window Detection

Use `xdotool getactivewindow getwindowname` to determine the focused application for skill suggestion matching. Map window titles to app identifiers for the suggestion engine.

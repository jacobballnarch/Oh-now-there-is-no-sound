# Oh, now there is no sound 🔇

![license](https://img.shields.io/badge/license-MIT-blue)
![rust](https://img.shields.io/badge/rust-2024-orange)

A tiny, native Wayland notification that slides in from the top-right corner whenever your default sink gets muted (or the audio server drops) — because sometimes you just need Jamiroquai to tell you the obvious.

![screenshot](./assets/Screenshot.png)

## Requirements

- A Wayland compositor that supports `wlr-layer-shell` (works on COSMIC, Sway, Hyprland, and most wlroots-based setups)
- `gtk4`
- `gtk4-layer-shell`
- PulseAudio or PipeWire with `pipewire-pulse`

## Building

```bash
git clone https://github.com/jacobballnarch/Oh-now-there-is-no-sound.git
cd Oh-now-there-is-no-sound

# Arch
sudo pacman -S gtk4 gtk4-layer-shell

cargo build --release
```

The binary will be at `target/release/Oh-now-there-is-no-sound`.

## Running

```bash
./target/release/Oh-now-there-is-no-sound
```

It sits quietly in the background and only shows itself when your default sink gets muted.

## Running it automatically (systemd user service)

A ready-made unit file is included: [`oh-no-sound.service`](oh-no-sound.service).

```bash

mkdir -p ~/.config/systemd/user
cp oh-no-sound.service ~/.config/systemd/user/
# edit ExecStart in ~/.config/systemd/user/oh-no-sound.service
systemctl --user daemon-reload
systemctl --user enable --now oh-no-sound.service
```

## Configuration

Right now the panel margin (`--top`), colors, and icon are hardcoded in `main.rs` — no config file yet. If you want to tweak the look, edit the CSS block in `load_css()` and rebuild.

## License

MIT — see [LICENSE](./LICENSE).

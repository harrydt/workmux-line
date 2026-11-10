# wmux-line

A fast, composable tmux statusline for [workmux](https://github.com/raine/workmux)
users. Think **lualine.nvim meets workmux**.

> Status: design phase. See [docs/research.md](docs/research.md),
> [docs/architecture.md](docs/architecture.md), [docs/mvp.md](docs/mvp.md).

## Why

When you run several workmux agents in parallel, the question you keep asking is
*"which one needs me?"* workmux already tracks this: each agent window gets a 💬
(waiting) or ✅ (done) icon that clears when you look at it. That icon only lives in
the window list of its own session, though. wmux-line gathers it from every session
into one indicator:

```
 main   auth-refactor                       💬 api-tests    14:32
```

```
 main   auth-refactor                    💬 2  ✅ 1    14:32
```

When nothing needs you, the indicator disappears.

## Principles

- **workmux-first.** It reads what workmux publishes, never re-tracks agents, and
  sends navigation to workmux (`last-done`, `last-agent`, `sidebar next`).
- **No runtime cost.** wmux-line compiles your config into native tmux formats once.
  tmux renders them itself and redraws the moment an agent changes state. There's no
  daemon and no per-refresh process for agent data.
- **Composable.** Left and right sections, reorderable components, themes,
  separators, nerd-font or Unicode icons, and components that hide on narrow
  terminals.
- **Good defaults.** Zero config gives you a polished line.

## Proposed usage

```sh
cargo install wmux-line
```

```tmux
# ~/.tmux.conf
run-shell 'wmux-line apply'
bind R run-shell 'wmux-line apply' \; display 'wmux-line reloaded'

# jump to agents using workmux's own navigation
bind l   run-shell 'workmux last-done'
bind Tab run-shell 'workmux last-agent'
```

Optional `~/.config/wmux-line/config.toml`:

```toml
preset   = "developer"   # minimal | developer | powerline
nerdfont = true          # false: plain separators, Unicode icons

[sections]
left  = ["session", "branch"]
right = ["agents", "clock"]

[colors]
accent = "#cba6f7"
```

The full schema is in [docs/mvp.md](docs/mvp.md#example-config-the-whole-schema-for-the-mvp).

Run `wmux-line print` to see the exact tmux options it would set.

## Requirements

- tmux 3.2 or newer
- workmux with status tracking set up (`workmux setup`). Without it, wmux-line still
  works, but the agent components stay empty.
- Linux or macOS

## License

MIT (proposed)

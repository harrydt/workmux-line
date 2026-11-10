# wmux-line

A fast, composable tmux statusline for [workmux](https://github.com/raine/workmux)
users. Think **lualine.nvim meets workmux**.

> Status: pre-release, MVP milestones 1–4 done. Design notes:
> [docs/research.md](docs/research.md), [docs/architecture.md](docs/architecture.md),
> [docs/mvp.md](docs/mvp.md).

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

## Install

From a checkout (no published release yet):

```sh
cargo install --path .
```

Try it in your running tmux. `apply` loads the status line straight away:

```sh
wmux-line apply
```

To keep it, add this to your tmux config (`~/.tmux.conf` or
`~/.config/tmux/tmux.conf`). Put it **after** your own `status-*` settings, but
**before** the TPM `run-shell` line:

```tmux
run-shell 'wmux-line apply'

# Jump to agents with workmux's own navigation
bind l   run-shell 'workmux last-done'    # next agent that needs you
bind Tab run-shell 'workmux last-agent'   # back to the previous agent

# TPM last, as usual
run-shell '~/.tmux/plugins/tpm/tpm'
```

Some plugins hook into `status-right` when TPM loads them. tmux-continuum, for
example, prepends its auto-save job there, so wmux-line has to set it first. For
the same reason, re-apply by **reloading your whole tmux config**
(`tmux source-file <your tmux.conf>`) after changing wmux-line's config. A bare
`wmux-line apply` would drop those hooks until the next reload.

wmux-line sets these tmux options: `status-left`, `status-right` (and their
`-length`), `status-style`, `window-status-style`, `window-status-current-style`,
and its own `@wmux_line_*`. To go back, remove the line and reload your tmux
config.

**Things to know**

- Plugins that inject into `status-right` lose their slot, because wmux-line owns
  that option. The built-in `mode` component replaces tmux-prefix-highlight
  (`PREFIX` / `COPY` badge), so that plugin can go. To keep your own status line
  instead, use [just the indicator](#use-just-the-indicator).
- workmux's per-window formats (the 🤖/💬/✅ in the window list) are left as they are.
- If you changed workmux's `status_icons`, set the same `waiting`/`done` icons in
  `[components.agents]`, or the indicator won't recognise them.
- The default `developer` preset needs a [Nerd Font](https://www.nerdfonts.com/). Set
  `nerdfont = false` otherwise.

## Configure

Everything is optional. With no file, you get the `developer` preset.
`~/.config/wmux-line/config.toml`:

```toml
preset   = "developer"   # minimal | developer | powerline
nerdfont = true          # false: plain separators, Unicode icons

[sections]
left  = ["session", "branch"]
right = ["agents", "mode", "clock"]

[colors]
accent = "#cba6f7"
```

The full schema is in [docs/mvp.md](docs/mvp.md#example-config-the-whole-schema-for-the-mvp).

Run `wmux-line print` to see the exact tmux commands `apply` would run. A config
error is reported (with the line and the valid choices), and the status line is
left unchanged.

## Use just the indicator

If you'd rather keep your own status line (hand-written, catppuccin/tmux,
oh-my-tmux, …), turn the full line off:

```toml
statusline = false
```

`apply` then sets only `@wmux_line_*` options and leaves `status-left`,
`status-right` and the styles alone. Embed the indicator wherever you like. It's
unstyled text, and empty when nothing needs you:

```tmux
set -g status-right '#{E:@wmux_line_agents} %H:%M'
# or with padding that disappears with it:
set -g status-right '#{?#{E:@wmux_line_agents},#[fg=#cba6f7] #{E:@wmux_line_agents} #[default],}%H:%M'
```

Run `wmux-line apply` before (or after) your status settings. The order doesn't
matter, because the option is only read when tmux draws the line.

## Requirements

- tmux 3.2 or newer
- workmux with status tracking set up (`workmux setup`). Without it, wmux-line still
  works, but the agent components stay empty.
- Linux or macOS

## License

MIT (proposed)

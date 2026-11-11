# workmux-line

A fast, composable tmux statusline for [workmux](https://github.com/raine/workmux)
users. Think **lualine.nvim meets workmux**.

> Status: pre-release.

## Why

When you run several workmux agents in parallel, the question you keep asking is
*"which one needs me?"* workmux already tracks this: each agent window gets a 💬
(waiting) or ✅ (done) icon that clears when you look at it. That icon only lives in
the window list of its own session, though. workmux-line gathers it from every session
into one indicator. One agent waiting in another session shows its worktree:

![Status line with one agent waiting: 💬 fix-rss](assets/statusline-one.png)

Several show as counts:

![Status line with two agents waiting and one done: 💬 2 ✅ 1](assets/statusline-many.png)

When nothing needs you, the indicator disappears.

## Principles

- **workmux-first.** It reads what workmux publishes, never re-tracks agents, and
  sends navigation to workmux (`last-done`, `last-agent`, `sidebar next`).
- **No runtime cost.** workmux-line compiles your config into native tmux formats once.
  tmux renders them itself and redraws the moment an agent changes state. There's no
  daemon and no per-refresh process for agent data.
- **Composable.** Left and right sections, reorderable components, themes,
  separators, nerd-font or Unicode icons, and components that hide on narrow
  terminals.
- **Good defaults.** Zero config gives you a polished line.

## Install

With Rust installed (no prebuilt binaries yet):

```sh
cargo install --git https://github.com/harrydt/workmux-line
```

Try it in your running tmux. `apply` loads the status line straight away:

```sh
workmux-line apply
```

To keep it, add this to your tmux config (`~/.tmux.conf` or
`~/.config/tmux/tmux.conf`). Put it **after** your own `status-*` settings, but
**before** the TPM `run-shell` line:

```tmux
run-shell 'workmux-line apply'

# Jump to agents with workmux's own navigation
bind l   run-shell 'workmux last-done'    # next agent that needs you
bind Tab run-shell 'workmux last-agent'   # back to the previous agent

# TPM last, as usual
run-shell '~/.tmux/plugins/tpm/tpm'
```

Some plugins hook into `status-right` when TPM loads them. tmux-continuum, for
example, prepends its auto-save job there, so workmux-line has to set it first. For
the same reason, re-apply by **reloading your whole tmux config**
(`tmux source-file <your tmux.conf>`) after changing workmux-line's config. A bare
`workmux-line apply` would drop those hooks until the next reload.

workmux-line sets these tmux options: `status-left`, `status-right` (and their
`-length`), `status-style`, `window-status-style`, `window-status-current-style`,
and its own `@workmux_line_*`. To go back, remove the line and reload your tmux
config.

**Things to know**

- Plugins that inject into `status-right` lose their slot, because workmux-line owns
  that option. The built-in `mode` component replaces tmux-prefix-highlight
  (a keyboard icon while the prefix is held, `COPY` in copy mode), so that plugin can go. To keep your own status line
  instead, use [just the indicator](#use-just-the-indicator).
- workmux's per-window formats (the 🤖/💬/✅ in the window list) are left as they are.
- If you changed workmux's `status_icons`, set the same `waiting`/`done` icons in
  `[components.agents]`, or the indicator won't recognise them.
- The default `developer` preset needs a [Nerd Font](https://www.nerdfonts.com/). Set
  `nerdfont = false` otherwise.

## Configure

Everything is optional. With no file, you get the `developer` preset. Every key in
`~/.config/workmux-line/config.toml`:

```toml
preset = "developer"          # minimal | developer (default) | powerline
nerdfont = true               # false: plain separators and Unicode icons
statusline = true             # false: only publish @workmux_line_agents to embed yourself
separators = "round"          # overrides the preset: powerline | round | plain

[sections]                    # each side overrides the preset's
left  = ["session", "branch"]
right = ["agents", "mode", "clock"]

[components.agents]
min_width = 0                 # hide below this client width (0 = never)
# icons default to workmux's defaults; override only if you changed status_icons
# waiting = "💬"
# done    = "✅"

[components.branch]
min_width = 90                # default

[components.mode]
prefix = ""                  # default: Nerd Font keyboard (⌨ without one)
copy = "COPY"                 # default

[components.clock]
min_width = 80                # default

[colors]                      # any role of the built-in default theme
accent = "#cba6f7"            # bar_bg bar_fg muted accent a_bg a_fg b_bg b_fg mode
```

Components:

- `session`: the tmux session name.
- `branch`: the git branch of the current pane; hidden outside a repository.
- `agents`: workmux agents waiting for you or done, across all sessions. Shows the
  worktree name for one, counts for more, nothing when there are none.
- `mode`: a keyboard icon while the prefix is held, `COPY` in copy mode; hidden
  otherwise. It has its own colour (`colors.mode`), so appearing never recolours
  its neighbours.
- `clock`: `HH:MM`.

Presets:

| Preset | Left | Right | Separators |
|---|---|---|---|
| `minimal` | session | agents, mode | plain |
| `developer` (default) | session, branch | agents, mode, clock | round |
| `powerline` | session, branch | agents, mode, clock | powerline |

Segment colours follow lualine: the outermost segment on each side uses `a_bg`/`a_fg`,
the next `b_bg`/`b_fg`, and the rest sit on the bar (`bar_bg`/`bar_fg`).

Run `workmux-line print` to see the exact tmux commands `apply` would run. A config
error is reported (with the line and the valid choices), and the status line is
left unchanged.

## Use just the indicator

If you'd rather keep your own status line (hand-written, catppuccin/tmux,
oh-my-tmux, …), turn the full line off:

```toml
statusline = false
```

`apply` then sets only `@workmux_line_*` options and leaves `status-left`,
`status-right` and the styles alone. Embed the indicator wherever you like. It's
unstyled text, and empty when nothing needs you:

```tmux
set -g status-right '#{E:@workmux_line_agents} %H:%M'
# or with padding that disappears with it:
set -g status-right '#{?#{E:@workmux_line_agents},#[fg=#cba6f7] #{E:@workmux_line_agents} #[default],}%H:%M'
```

Run `workmux-line apply` before (or after) your status settings. The order doesn't
matter, because the option is only read when tmux draws the line.

## Requirements

- tmux 3.2 or newer
- workmux with status tracking set up (`workmux setup`). Without it, workmux-line still
  works, but the agent components stay empty.
- Linux or macOS

## License

[MIT](LICENSE)

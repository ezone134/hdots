# *_main — quick guide for keybinds.lua

The shell's CLI is a **family of one-per-domain bash tools** (`scripts/*_main`,
symlinked into `~/bin` so they're on `$PATH`): `audio_main`, `brightness_main`,
`theme_main`, `power_main`, `caffeine_main`, `pp_main`, `shot_main`,
`media_main`, `network_main`, `ws_main`, `status_main`. You never call `wpctl`
/ `brightnessctl` / `playerctl` directly from keybinds; the `*_main` tools wrap
them and keep the shell's state files in sync, so the OSD/bar and your keys
always agree.

General shape of a command:

```
<domain>_main <action> [number]
```

The `[number]` is optional. For `up` / `down` it is a **step** in percent
(defaults to 5). For `set` it is the target percent.

## Volume (speakers) — `audio_main vol`

| You want…                 | Command                          |
|---------------------------|----------------------------------|
| Raise volume              | `audio_main vol up`              |
| Raise by 10%              | `audio_main vol up 10`           |
| Lower volume              | `audio_main vol down`            |
| Set to exactly 40%        | `audio_main vol set 40`          |
| Mute                      | `audio_main vol mute`            |
| Unmute                    | `audio_main vol unmute`          |
| Toggle mute               | `audio_main vol toggle`          |
| Read current value        | `audio_main vol get`             |

## Mic — `audio_main mic`

Same actions, same logic, just swap `vol` for `mic`:

| You want…                 | Command                          |
|---------------------------|----------------------------------|
| Raise mic gain            | `audio_main mic up`              |
| Lower mic gain            | `audio_main mic down`            |
| Set to exactly 70%        | `audio_main mic set 70`          |
| Toggle mic mute           | `audio_main mic toggle`          |
| Mute / unmute             | `audio_main mic mute` / `audio_main mic unmute` |

## Audio routing / streams — `audio_main audio`

| You want…                          | Command                                   |
|------------------------------------|-------------------------------------------|
| List sinks / sources               | `audio_main audio sinks` / `sources`      |
| Set default sink / source          | `audio_main audio set-sink NAME` / `set-source NAME` |
| List per-app streams               | `audio_main audio streams`                |
| Stream volume / mute               | `audio_main audio stream-vol ID PCT` / `stream-mute ID [on\|off\|toggle]` |

## Brightness — `brightness_main`

| You want…                 | Command                     |
|---------------------------|-----------------------------|
| Brightness up             | `brightness_main up`        |
| Brightness down           | `brightness_main down`      |
| Set to 50%                | `brightness_main set 50`    |
| Read current              | `brightness_main get`       |

## Theme — `theme_main`

| You want…                          | Command                          |
|------------------------------------|----------------------------------|
| Toggle dark / light                | `theme_main toggle`              |
| Force dark / light                 | `theme_main dark` / `theme_main light` |
| Restore (accent changed, reapply)  | `theme_main restore`             |
| Reapply current theme              | `theme_main reapply`             |

## Power / session — `power_main`

| You want…                 | Command                          |
|---------------------------|----------------------------------|
| Battery / AC status       | `power_main status`              |
| Lock screen               | `power_main session lock`        |
| Suspend / reboot / logout | `power_main session suspend` / `reboot` / `logout` |

## Other handy ones

| You want…                          | Command                                  |
|------------------------------------|------------------------------------------|
| Play / pause                       | `media_main play-pause`                  |
| Next / prev track                  | `media_main next` / `media_main prev`    |
| Stop                                | `media_main stop`                        |
| Full screenshot                     | `shot_main full`                         |
| Region screenshot                   | `shot_main region`                       |
| Caffeine (idle inhibit) toggle      | `caffeine_main toggle`                   |
| Power profile cycle                 | `pp_main cycle`                          |
| Workspace next / prev / goto        | `ws_main next` / `ws_main prev` / `ws_main goto 5` |
| See everything at once              | `status_main`                            |

## How to wire them into keybinds.lua

In `keybinds.lua` everything is `hl.bind(<keys>, hl.dsp.exec_cmd("<command>"))`.
Just drop the `*_main ...` string in there — that's the whole trick:

```lua
-- Volume
hl.bind("XF86AudioRaiseVolume", hl.dsp.exec_cmd("audio_main vol up 5"), { locked = true, repeating = true })
hl.bind("XF86AudioLowerVolume", hl.dsp.exec_cmd("audio_main vol down 5"), { locked = true, repeating = true })
hl.bind("XF86AudioMute",        hl.dsp.exec_cmd("audio_main vol toggle"), { locked = true, repeating = true })

-- Mic
hl.bind("XF86AudioMicMute",    hl.dsp.exec_cmd("audio_main mic toggle"), { locked = true, repeating = true })
hl.bind("SUPER + XF86AudioRaiseVolume", hl.dsp.exec_cmd("audio_main mic up"),   { locked = true, repeating = true })
hl.bind("SUPER + XF86AudioLowerVolume", hl.dsp.exec_cmd("audio_main mic down"), { locked = true, repeating = true })

-- Brightness
hl.bind("XF86MonBrightnessUp",   hl.dsp.exec_cmd("brightness_main up"),   { locked = true, repeating = true })
hl.bind("XF86MonBrightnessDown", hl.dsp.exec_cmd("brightness_main down"), { locked = true, repeating = true })

-- Theme toggle
hl.bind("SUPER + M", hl.dsp.exec_cmd("theme_main toggle"), { locked = true, repeating = true })
```

## Quick mental model

- **`up` / `down`** → step by a percent amount (5 default). Use for media keys.
- **`set N`** → jump straight to a percent. Use for sliders / exact values.
- **`toggle`** → flip mute (or theme / caffeine / profile). Use for the mute key.
- **No `get` needed in keybinds** — `get` is only for the terminal/shell.

That's it. Any `*_main ...` string can live inside `hl.dsp.exec_cmd(...)`.
Run any of them bare from a terminal to see the defaults (`theme_main` alone =
`toggle`, `brightness_main` alone = `get`, `power_main` alone = `status`).

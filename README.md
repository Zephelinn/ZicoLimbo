<div align="center">

# ZicoLimbo

**An ultra-lightweight, multi-version Minecraft queue server written in Rust**

*Forked from [PicoLimbo](https://github.com/Quozul/PicoLimbo) — supporting all Minecraft versions from 1.7.2 through 26.1.1*

[![License](https://img.shields.io/github/license/Zephelinn/ZicoLimbo)](LICENSE)

[⭐ Star this repo](https://github.com/Zephelinn/ZicoLimbo)

![PicoLimbo.png](docs/public/world.png)
*Schematic from [LOOHP/Limbo](https://github.com/LOOHP/Limbo)*

</div>

---

## Introduction

ZicoLimbo is a fork of [PicoLimbo](https://github.com/Quozul/PicoLimbo) extended to function as a **Minecraft queue server**. Players connect and wait in a queue with live position updates, then get pushed to the main server automatically. It retains PicoLimbo's core focus on efficiency — ultra-lightweight with no unnecessary overhead.

Supports **all Minecraft versions from 1.7.2 through 26.1.1**, excluding snapshots.

## Features

### 🔢 Queue System

Players are placed in an ordered queue on join. A configurable push loop periodically moves players forward, sending them to the main server via kick (proxy re-routes) or the native 1.20.5+ Transfer packet.

- **Live position updates** via title, action bar, boss bar, and tab list
- **Placeholders**: `{position}`, `{total}`, `{player}`, `{eta}` supported in all display text
- **MiniMessage** formatting supported everywhere
- **Configurable refresh interval** for display updates
- **Push methods**: `kick` (proxy handles routing) or `transfer` (1.20.5+ native, falls back to kick on older versions)

### 🎮 Wide Version Compatibility

Supports all Minecraft versions from **1.7.2 to 26.1.1** natively, no need for ViaVersion or additional compatibility layers.

### ⚡ Ultra-Lightweight & Highly Scalable

Uses **0% CPU while idle** and handles **hundreds of players** under 10 MB RAM.

### 👤 Skin Support

Player skins are supported.

### 🔀 Built-in Proxy Support

Integrates with all major Minecraft proxies:

- Velocity (Modern Forwarding)
- BungeeCord (Legacy Forwarding)
- BungeeGuard & BungeeGuardPlus authentication

### ⚙️ Highly Configurable

Customize your server using a simple TOML configuration file. All queue display settings, push behaviour, and intervals are configurable.

### 🌍 Schematic World (Experimental)

Load a custom world from a schematic file and customize spawn location (1.16+ only).

---

## Queue Configuration

Add the following to your `server.toml` to enable the queue:

```toml
[queue]
enabled = true
push_interval_seconds = 5
push_count = 1
refresh_interval_seconds = 3

[queue.push_method]
type = "kick"
kick_message = "You have been moved to the main server."
# Or for 1.20.5+ Transfer packet:
# type = "transfer"
# host = "play.example.com"
# port = 25565

[queue.title]
enabled = true
title = "<bold>Queue</bold>"
subtitle = "Position: {position} of {total}"
fade_in = 0
stay = 2147483647
fade_out = 0

[queue.action_bar]
enabled = true
text = "<yellow>Queue: {position}/{total} | ETA: {eta}s</yellow>"

[queue.boss_bar]
enabled = true
title = "Queue: {position}/{total}"
color = "blue"
division = 0

[queue.tab_list]
enabled = true
header = "<bold>Queue</bold>"
footer = "Position: {position}/{total}"
```

---

## Quick Start

### Pterodactyl

Eggs for Pterodactyl are provided in the [pterodactyl](./pterodactyl) directory.

### Binary / Standalone

Download from [GitHub Releases](https://github.com/Zephelinn/ZicoLimbo/releases)

---

## Similar Projects

- [PicoLimbo](https://github.com/Quozul/PicoLimbo): The upstream project this is forked from
- [Limbo](https://github.com/LOOHP/Limbo): Supports only one Minecraft version at a time
- [NanoLimbo](https://github.com/Nan1t/NanoLimbo): Actively maintained
  (see [BoomEaro's fork](https://github.com/BoomEaro/NanoLimbo))

---

## Contributing

Contributions are welcome! If you encounter any issues or have suggestions for improvement, please submit an issue or pull request on GitHub.

1. Fork the repository.
2. Create a new branch `git checkout -b <branch-name>`.
3. Make changes and commit `git commit -m 'Add some feature'`.
4. Push to your fork `git push origin <branch-name>`.
5. Submit a pull request.

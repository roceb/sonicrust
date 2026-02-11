# Sonicrust

A Terminal based music player for Subsonic-compatible servers, written in Rust.

## Features:
- **Subsonic API Integration**: Connect to any Subsonic-compatible server (Navidrome, Airsonic, etc.)
- **Terminal UI**: Clean, responsive interface built with [Ratatui](https://github.com/ratatui-org/ratatui)
- **MPRIS Support**: Full media player integration for Linux desktop environments
- **Multiple Browse Modes**: Navigate by Queue, Songs, Artists, Albums, or Search
- **Local & Remote Search**: Fuzzy search through your local library or query the server directly
- **Playback Controls**: Play, pause, seek, volume control, next/previous track with [Rodio](https://github.com/RustAudio/rodio)
- **Scrobbling**: Automatic scrobbling support via the Subsonic API

### Build from Source

```bash
git clone https://github.com/roceb/sonicrust.git
cd sonicrust
cargo build --release
```

## Configuration

Create a configuration file at `~/.config/sonicrust/config.toml`:

```toml
[server]
url = "https://your-subsonic-server.com"
username = "your-username"
password = "your-password"
salt = "random-salt"

[search]
mode = "Local"  # or "Remote"
fuzzy_threshold = 0.6  # 0.0 to 1.0, lower = more fuzzy
```

### Keybindings

#### Global

| Key | Action |
|-----|--------|
| `q` | Quit |
| `Tab` | Next tab |
| `Shift+Tab` | Previous tab |
| `1-5` | Switch to tab (Queue/Songs/Artists/Albums/Search) |
| `Space` | Toggle play/pause |
| `n` | Next track |
| `p` | Previous track |
| `←` | Seek backward 5s |
| `→` | Seek forward 5s |
| `+` | Volume up |
| `-` | Volume down |
| `r` | Refresh library |

#### Navigation

| Key | Action |
|-----|--------|
| `j` / `↓` | Move down |
| `k` / `↑` | Move up |
| `Enter` | Play selected item |
| `s` | Open search |

#### Search Mode

| Key | Action |
|-----|--------|
| `Esc` | Exit search mode |
| `Enter` | Play selected result / Perform search |
| `Ctrl+r` | Clear search |
| `Ctrl+a` | Add result to queue |
| Any character | Type search query |
| `Backspace` | Delete character |

## Roadmap

- [ ] Playlist management
- [ ] Shuffle mode implementation
- [ ] Album art display (sixel/kitty protocol)
- [ ] Lyrics support
- [ ] Offline caching
- [ ] Multiple server profiles
- [ ] Custom themes
- [ ] Better UI
- [ ] Radio feature
- [ ] Allow for other audio backends like mpv

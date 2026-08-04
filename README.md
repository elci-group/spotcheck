# spotcheck

```
  ██████╗ █████╗  ██████╗██╗  ██╗███████╗██████╗ 
 ██╔════╝██╔══██╗██╔════╝██║ ██╔╝██╔════╝██╔══██╗
 ██║     ███████║██║     █████╔╝ █████╗  ██████╔╝
 ██║     ██╔══██║██║     ██╔═██╗ ██╔══╝  ██╔══██╗
 ╚██████╗██║  ██║╚██████╗██║  ██╗███████╗██║  ██║
  ╚═════╝╚═╝  ╚═╝ ╚═════╝╚═╝  ╚═╝╚══════╝╚═╝  ╚═╝
                                                 
    Precision text extraction for the terminal
```

Select anything visible in your terminal without touching the mouse.

<!-- Demo GIF placeholder -->
<!-- 
![spotcheck demo](docs/demo.gif)
-->

## The Problem

Terminal efficiency breaks the moment you want to copy text. You have to migrate to mouse to select text, which is slow and ergonomic-hostile.

## The Solution

Spotcheck is a keyboard-driven semantic selection layer over terminal output. Instead of dragging a cursor between pixel coordinates, you define ranges by anchoring two semantic points:

```
"Copy from where it says nginx to where it says 443"
```

## How It Works

1. Press your key combo to enter spotcheck mode
2. Type to find the **start point** (e.g., "nginx")
3. Navigate matches with ↑↓, press Enter to confirm
4. Type to find the **end point** (e.g., "443")
5. Navigate matches with ↑↓, press Enter to confirm
6. Press Enter to copy the range to clipboard

## Example

```bash
$ journalctl -xe
...
Aug 03 19:42:11 server nginx[4421]: failed to bind port 443
Aug 03 19:42:12 server nginx[4421]: retrying
...
```

**User flow:**
1. Type `nginx` → spotcheck highlights all "nginx" matches
2. Press Enter → anchor start point at first "nginx"
3. Type `443` → spotcheck highlights all "443" matches
4. Press Enter → anchor end point at "443"
5. Press Enter → copy to clipboard

**Result:** `nginx[4421]: failed to bind port 443`

## Architecture

The current implementation is a TUI-based prototype that demonstrates the core concept:

- **Search engine**: Case-insensitive substring search
- **Two-point selection**: Start and end point anchoring
- **Multi-line support**: Extract text across multiple lines
- **Clipboard integration**: Copy selections to system clipboard
- **Interactive UI**: ratatui-based terminal interface

## Usage

### Interactive Mode

```bash
cargo run
```

Controls:
- Type to search
- ↑↓ Navigate matches
- Enter Confirm selection
- Esc Cancel

### Test Mode

```bash
cargo run -- --test nginx 443
```

Output:
```
Selection: 'nginx[4421]: failed to bind port 443'
```

## Current Limitations

This is a prototype. The demo buffer is hardcoded. For production use, spotcheck would need:

1. **Terminal emulator integration** (best UX)
   - Kitty plugin
   - WezTerm plugin
   - Alacritty plugin
   - Direct access to terminal buffer

2. **Shell wrapper** (less elegant)
   - `spotcheck ssh user@server`
   - `sc journalctl -xe`

3. **ANSI screen scraper** (universal but harder)
   - `/dev/tty` access
   - Terminal escape sequence parsing

## Future Enhancements

- **Fuzzy matching**: Instead of exact strings, use fuzzy search like fzf
- **Regex support**: `/0x[0-9a-f]+/` for structured patterns
- **Vim-like movement**: h/j/k/l for navigation
- **Structured extraction**: Understand column layouts (docker ps, etc.)
- **Multiple selection**: Select and copy multiple ranges

## Building

```bash
cargo build
cargo run
```

## Dependencies

- `crossterm`: Terminal handling
- `ratatui`: TUI framework
- `arboard`: Clipboard access

## License

MIT

## Inspiration

This tool combines ideas from:
- tmux copy mode
- Vim visual mode
- fzf selection
- Emacs region selection

The unique contribution is semantic endpoints instead of cursor coordinates.

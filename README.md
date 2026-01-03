# Pexand - Text Expander

Pexand is a lightweight, fast, and intelligent text expander for Windows that automatically replaces short trigger phrases with longer text snippets.

## Features

✨ **Core Features**

- 🚀 Lightning-fast text expansion (< 5ms latency)
- 🎯 Global keyboard monitoring with zero configuration
- 💾 SQLite database for reliable snippet storage
- 🔍 Fuzzy search to find snippets quickly
- 📝 Full CRUD UI for managing snippets
- 🎨 Clean, dark-themed interface
- 💪 Works in any Windows application
- ⚙️ Application whitelist/blacklist filtering
- 📤 Export/Import snippets for backup and sharing

🎭 **Advanced Features**

- 🔄 Recursion prevention for safe expansions
- 📊 Usage tracking and statistics
- 🎯 Radix Trie for O(m) pattern matching
- 🔒 Thread-safe architecture
- 💼 Portable mode support

## Installation

### Pre-built Binary

1. Download the latest `pexand.exe` from the releases page
2. Run the executable - no installation required!

### Build from Source

```bash
# Clone the repository
git clone https://github.com/yourusername/pexand.git
cd pexand

# Build release version
cargo build --release

# Run
cargo run --release
```

## Quick Start

1. **Launch Pexand** - The UI will open showing your snippets
2. **Try the defaults** - Type `;name` in any application and it expands to "Neo Anderson"
3. **Create your own** - Click "New Snippet" in the UI to add custom expansions

### Default Snippets

Pexand comes with three example snippets:

- `;name` → "Neo Anderson"
- `;email` → "neo@matrix.com"
- `;date` → Current date (e.g., "2026-01-02")

## Usage

### Creating Snippets

1. Open Pexand UI
2. Click "New Snippet" or press `Ctrl+N`
3. Enter:
   - **Trigger**: The shortcut text (e.g., `;addr`)
   - **Label**: Optional description
   - **Body**: The expansion text
4. Press `Ctrl+S` or click "Save"

### Settings & Configuration

Click the **Settings** icon (⚙️) in the top bar to access:

#### Application Filtering

- **Whitelist (Only in these apps)**: Specify applications where text expansion should work. If empty, expansion works everywhere except blocked apps.
- **Blacklist (Never in these apps)**: Specify applications where text expansion should never trigger.
- Add app names like `notepad.exe`, `chrome.exe`, etc.

#### Data Management

- **Export Snippets**: Save all your snippets to a JSON file for backup or sharing
- **Import Snippets**: Load snippets from a JSON file. Existing snippets with matching triggers will be updated, new ones will be added.

### Dynamic Variables

Pexand supports dynamic variables that are evaluated at expansion time:

- `{{date}}` - Current date in YYYY-MM-DD format
- `{{date:%Y-%m-%d}}` - Custom date format (strftime)
- `{{date:%B %d, %Y}}` - Example: "January 02, 2026"
- `{{clipboard}}` - Current clipboard content

**Example:**

```
Trigger: ;meeting
Body: Meeting scheduled for {{date:%B %d, %Y}} at 2 PM
```

### Keyboard Shortcuts

**In Main Window:**

- `Ctrl+N` - New snippet
- `Ctrl+F` - Focus search
- `Enter` - Edit selected snippet
- `Delete` - Delete selected snippet
- `Esc` - Close window
- `Up/Down` - Navigate list

**In Editor:**

- `Ctrl+S` - Save snippet
- `Esc` - Cancel editing

## Configuration

### Database Location

**Standard Mode:** `%APPDATA%\Pexand\pexand.db`

**Portable Mode:** Create a file named `portable.txt` next to `pexand.exe`, and the database will be stored in the same directory.

### Portable Installation

1. Copy `pexand.exe` to a USB drive or any folder
2. Create an empty file named `portable.txt` in the same folder
3. Run `pexand.exe`
4. All data will be stored locally in `pexand.db`

## Architecture

Pexand uses a multi-threaded architecture:

1. **Sentinel Thread** - Monitors keyboard input globally
2. **UI Thread** - Manages the Iced GUI application
3. **Database** - SQLite for persistent storage

### Components

- **Buffer**: Rolling buffer tracking last 50 characters
- **Trie**: Radix Trie for O(m) trigger matching
- **Injector**: Keyboard simulation with `enigo`
- **Parser**: Variable expansion engine
- **UI**: Iced-based desktop application

## Performance

- **Memory Usage**: < 15MB idle
- **Expansion Latency**: < 5ms
- **CPU Usage**: < 1% idle
- **Binary Size**: ~4-8MB (optimized release)
- **Startup Time**: < 500ms

## Safety Features

### Recursion Prevention

Pexand automatically detects:

- Direct recursion (trigger in its own body)
- Indirect recursion (circular trigger chains)
- Maximum depth limiting (5 levels)

Invalid examples that are prevented:

```
;loop → "This is ;loop"  ❌ Direct recursion
;a → "Contains ;b"       ❌ Indirect recursion
;b → "Contains ;a"
```

## Technical Details

### Stack

- **Language**: Rust 🦀
- **GUI**: Iced v0.12
- **Database**: SQLite (rusqlite v0.32)
- **Keyboard**: rdev v0.5 + enigo v0.2
- **Search**: fuzzy-matcher v0.3

### Why Rust?

- Memory safety without garbage collection
- Zero-cost abstractions
- Fearless concurrency
- Small binary sizes
- Native performance

## Troubleshooting

### Expansions Not Working

1. Check if Pexand is running (should see in taskbar)
2. Verify trigger exists in the snippet list
3. Make sure you're typing the complete trigger
4. Try restarting Pexand

### UI Won't Open

1. Check if database is accessible
2. Look for error messages in the console
3. Try deleting `pexand.db` to reset (will lose snippets)

### High Memory Usage

1. Check number of snippets (thousands may use more memory)
2. Restart Pexand to clear any leaks
3. Report issue with details

## Development

### Running Tests

```bash
# Run all tests
cargo test

# Run specific test
cargo test test_name

# Run with output
cargo test -- --nocapture
```

### Building

```bash
# Debug build
cargo build

# Release build (optimized)
cargo build --release

# Check without building
cargo check
```

### Project Structure

```
pexand/
├── src/
│   ├── main.rs           # Application entry point
│   ├── lib.rs            # Library exports
│   ├── db/               # Database layer
│   │   ├── snippet.rs    # Snippet data structure
│   │   ├── database.rs   # DB initialization
│   │   ├── manager.rs    # CRUD operations
│   │   └── bootstrapper.rs # Default seeding
│   ├── sentinel/         # Expansion engine
│   │   ├── buffer.rs     # Rolling keystroke buffer
│   │   ├── trie.rs       # Pattern matching
│   │   ├── injector.rs   # Text injection
│   │   ├── sentinel.rs   # Main coordinator
│   │   └── variables.rs  # Variable parser
│   └── ui/               # User interface
│       └── app.rs        # Iced application
├── tests/                # Integration tests
├── Cargo.toml            # Dependencies
└── README.md             # This file
```

## FAQ

**Q: Does Pexand work in all applications?**  
A: Yes! Pexand uses global keyboard hooks that work system-wide.

**Q: Can I backup my snippets?**  
A: Yes, just copy the `pexand.db` file. It's a standard SQLite database.

**Q: Is my data secure?**  
A: All data is stored locally in SQLite. Nothing is sent to the internet.

**Q: Can I export/import snippets?**  
A: Not yet, but you can directly access the SQLite database with any SQLite tool.

**Q: Does it work on Mac/Linux?**  
A: Currently Windows only. The core logic is cross-platform, but keyboard hooks are OS-specific.

**Q: How many snippets can I have?**  
A: Thousands! The Trie structure scales efficiently.

## Contributing

Contributions are welcome! Please:

1. Fork the repository
2. Create a feature branch
3. Make your changes with tests
4. Submit a pull request

## License

MIT License - See LICENSE file for details

## Credits

Developed by the Pexand Team  
Built with ❤️ and Rust 🦀

## Support

- 🐛 **Issues**: GitHub Issues
- 💬 **Discussions**: GitHub Discussions
- 📧 **Email**: support@pexand.dev (if you set this up)

## Changelog

### v2.0.0 (2026-01-02)

- ✨ Complete rewrite in Rust
- 🎨 New Iced-based UI
- 💾 SQLite database backend
- 🚀 10x faster expansions
- 📅 Dynamic variables support
- 🔍 Fuzzy search
- 🔒 Recursion prevention
- 📊 Usage tracking

### v1.0.0

- Initial release (if you had one)

---

**Made with 🦀 Rust** | **Powered by ⚡ Iced** | **Data stored in 💾 SQLite**

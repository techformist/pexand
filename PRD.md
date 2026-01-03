Here is the updated, definitive Technical Product Requirement Document (PRD) for **Pexand**.

This version removes the file-system config dependency entirely. The application is a self-contained, database-driven engine.

---

# Product Requirement Document: Pexand (v2.0)

**Type:** Desktop Application (Windows/macOS)
**Distribution:** Single Portable Executable (`.exe`)
**Core Technology:** Rust + Iced + SQLite

---

## 1. Product Philosophy

**"The Database is the Truth."**
Pexand does not rely on fragile text files. It is a robust, self-contained binary that manages its own high-performance embedded database. It ships with "Batteries Included" (defaults) and offers a spotlight-style interface for management.

---

## 2. System Architecture

The application is compiled into a single static binary. It manages:

1.  **The Sentinel (Background Thread):** Listens for text expansion triggers AND the "Summon UI" hotkey.
2.  **The Vault (Storage):** An embedded **SQLite** database.
3.  **The Dashboard (GUI):** An **Iced** window that is normally hidden, visible only on command.

### 2.1 Storage Layer (The Vault)

- **Engine:** `rusqlite` (SQLite embedded database, ACID compliant).
- **Location:**
  - _Standard:_ `%APPDATA%/Pexand/pexand.db`
  - _Portable Mode:_ `./pexand.db` (if running from a USB drive).
- **Schema:**
  - **Table:** `snippets`
  - **Columns:**
    - `trigger` TEXT PRIMARY KEY (e.g., `;email`)
    - `label` TEXT (Optional description)
    - `body` TEXT (The expansion content)
    - `usage_count` INTEGER DEFAULT 0
    - `created_at` INTEGER (Unix timestamp)
    - `updated_at` INTEGER (Unix timestamp)

### 2.2 Bootstrapping (First Run Experience)

If `pexand.db` does not exist on startup, the app creates it and seeds default "Demo" data so the user understands how it works immediately:

- `;name` $\rightarrow$ `Neo Anderson`
- `;email` $\rightarrow$ `neo@matrix.com`
- `;date` $\rightarrow$ `{{date:%Y-%m-%d}}`

---

## 3. User Interface (The Dashboard)

The UI is designed for speed. Mouse usage is optional.

### 3.1 Invocation

- **Default Hotkey:** `Ctrl + Alt + Shift + P` (Configurable in DB).
- **Behavior:** The window appears instantly in the center of the screen.
- **Focus:** The cursor is strictly locked to the **Search Bar**.

### 3.2 The Main View (Search & List)

- **Top Bar:** A large, clean Search Input.
  - _Placeholder:_ "Type to search triggers or content..."
- **Middle:** A virtualized list of snippets.
  - _List Item:_ Shows Trigger (Left, Bold) and truncated Content (Right, Gray).
  - _Selection:_ Arrow keys `Up`/`Down` navigate the list.
- **Bottom Bar:** Quick Hint Strip.
  - `Enter`: Edit
  - `Ctrl+N`: New
  - `Esc`: Hide
  - `Del`: Delete

### 3.3 Search Logic (Fuzzy)

As the user types in the search box, the list filters in real-time.

- **Scope:** Matches against the **Trigger** AND the **Expansion Body**.
- **Example:** Typing "Matrix" will find the `;email` snippet (because it contains "neo@matrix.com").

### 3.4 The Editor (Overlay/Modal)

When `Ctrl+N` or `Enter` is pressed:

- **Fields:**
  1.  **Trigger:** (e.g., `;sig`) - _Auto-validates for duplicates._
  2.  **Label/Description:** (Optional).
  3.  **Expansion:** Multi-line text area.
- **Action:** `Ctrl+S` (Save & Close), `Esc` (Cancel).

---

## 4. Technical Specifications

### 4.1 Global Hotkey Handling

The Sentinel must listen for the "Summon" chord even when other apps are focused.

- **Crate:** `global-hotkey` (Cross-platform).
- **Logic:**
  ```rust
  // Simplified Logic
  if event == Hotkey::Summon {
      app_window.set_visible(true);
      app_window.set_focus();
  }
  ```

### 4.2 Expansion Logic (The Sentinel)

1.  **Startup:** Load all Triggers from SQLite into a `RadixTrie` (Memory).
    - _Note:_ We only load the keys (triggers) into RAM for speed. The Body stays on disk until matched.
2.  **Detection:** User types trigger.
3.  **Lookup:** Trie Match $\rightarrow$ Fetch Body from SQLite.
4.  **Injection:** Simulate Backspaces $\rightarrow$ Inject Body.

### 4.3 Database Updates

When the user saves a new snippet in the UI:

1.  **Write:** Update SQLite `snippets` table.
2.  **Signal:** Send `Message::ReloadTrie` to the Sentinel thread.
3.  **Result:** The new trigger works immediately without restarting the app.

---

## 5. Keyboard Shortcuts (App Context)

These shortcuts only work when the Pexand window is open/focused:

| Key         | Action                          |
| :---------- | :------------------------------ |
| `Ctrl + N`  | Create New Snippet              |
| `Ctrl + F`  | Focus Search Bar                |
| `Down / Up` | Select Snippet                  |
| `Enter`     | Edit Selected Snippet           |
| `Delete`    | Delete Selected Snippet         |
| `Esc`       | Close Window (Minimize to Tray) |

---

## 6. Development Roadmap

### Phase 1: The Core & DB (No UI)

- Setup SQLite database with `rusqlite`.
- Write the `Bootstrapper` (Seed defaults).
- Implement `SnippetManager` struct (CRUD operations).
- Test: Hardcode input `name` and ensure SQLite returns `Neo Anderson`.

### Phase 2: The Sentinel

- Implement `rdev` or `enigo` hooks.
- Connect the Trie to the SQLite lookups.
- Implement text injection.

### Phase 3: The Iced Dashboard

- Build the Search + List view.
- Implement the "Global Hotkey" to toggle visibility.
- Implement `Ctrl+N` / Editor logic.
- **Crucial:** Ensure the window uses `mode: hidden` on startup, only showing the Tray icon.

### Phase 4: Polish

- Add visual feedback when a snippet is saved.
- Add logic to prevent "Recursive Expansion" (e.g., if trigger is `;a` and content is `;a`, don't loop forever).

---

## 7. Recommended Crates

```toml
[dependencies]
# UI
iced = { version = "0.12", features = ["system", "canvas"] }
rfd = "0.12" # For native dialogs if needed

# Logic
rusqlite = { version = "0.31", features = ["bundled"] } # The Database
chrono = "0.4" # Date/time for timestamps
global-hotkey = "0.5" # For Ctrl+Alt+Shift+P

# System Interaction
tray-icon = "0.14"
clipboard-win = "5.0" # Windows specific clipboard
enigo = "0.2" # Keyboard simulation
```

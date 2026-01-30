# libghostty Research

## Summary

Ghostty exposes a C embedding API (`libghostty`) that lets you embed terminal emulators into native apps. The API is well-documented in code, though officially "not yet general purpose."

**Source:** https://github.com/ghostty-org/ghostty

## Key Files

| File | Purpose |
|------|---------|
| `include/ghostty.h` | C header with all types and function declarations |
| `src/main_c.zig` | C API entry points |
| `src/apprt/embedded.zig` | Embedding runtime (macOS/iOS specific) |

## Architecture

```
┌─────────────────────────────────────────┐
│         Your App (Swift/ObjC)           │
│  ┌─────────────────────────────────┐    │
│  │     NSView / MTKView            │    │
│  └─────────────────────────────────┘    │
│              ▲                          │
│              │ ghostty_surface_*()      │
│              ▼                          │
│  ┌─────────────────────────────────┐    │
│  │        libghostty               │    │
│  │  - Terminal emulation (VT)      │    │
│  │  - GPU rendering (Metal)        │    │
│  │  - Font rendering               │    │
│  │  - Input handling               │    │
│  └─────────────────────────────────┘    │
│              ▲                          │
│              │ PTY                       │
│              ▼                          │
│  ┌─────────────────────────────────┐    │
│  │     Shell / Coding Agent        │    │
│  └─────────────────────────────────┘    │
└─────────────────────────────────────────┘
```

## Core Types

```c
typedef void* ghostty_app_t;      // Application instance
typedef void* ghostty_config_t;   // Configuration
typedef void* ghostty_surface_t;  // Terminal surface (one per terminal)
typedef void* ghostty_inspector_t; // Debug inspector
```

## Platform Config (macOS)

```c
typedef struct {
  ghostty_platform_e platform_tag;  // GHOSTTY_PLATFORM_MACOS
  ghostty_platform_u platform;      // { .macos = { .nsview = yourView } }
  void* userdata;
  double scale_factor;
  float font_size;
  const char* working_directory;
  const char* command;              // Shell or agent to spawn
  ghostty_env_var_s* env_vars;
  size_t env_var_count;
  const char* initial_input;        // Send text on startup
  bool wait_after_command;
  ghostty_surface_context_e context; // WINDOW, TAB, or SPLIT
} ghostty_surface_config_s;
```

## Key Functions

### Initialization
```c
int ghostty_init(size_t argc, char** argv);
ghostty_info_s ghostty_info(void);
```

### App Lifecycle
```c
ghostty_app_t ghostty_app_new(ghostty_runtime_config_s* config, ghostty_config_t cfg);
void ghostty_app_free(ghostty_app_t app);
```

### Surface (Terminal) Management
```c
ghostty_surface_t ghostty_surface_new(ghostty_app_t app, ghostty_surface_config_s* config);
void ghostty_surface_free(ghostty_surface_t surface);
```

### Input Handling
```c
// Key events
bool ghostty_surface_key(ghostty_surface_t surface, ghostty_input_key_s event);

// Text input (like paste)
void ghostty_surface_text(ghostty_surface_t surface, const char* ptr, size_t len);

// Mouse
bool ghostty_surface_mouse_button(ghostty_surface_t surface, ...);
void ghostty_surface_mouse_pos(ghostty_surface_t surface, double x, double y, int mods);
void ghostty_surface_mouse_scroll(ghostty_surface_t surface, double x, double y, int scroll_mods);

// Focus
void ghostty_surface_set_focus(ghostty_surface_t surface, bool focused);
```

### Callbacks (You Provide)

The `ghostty_runtime_config_s` struct has callbacks you implement:

```c
typedef struct {
  void* userdata;
  bool supports_selection_clipboard;
  
  // Called to wake up your event loop
  void (*wakeup)(void* userdata);
  
  // Called when ghostty wants to perform an action
  bool (*action)(ghostty_app_t app, ghostty_target_s target, ghostty_action_s action);
  
  // Clipboard operations
  void (*read_clipboard)(void* userdata, int clipboard, ghostty_clipboard_request_s* req);
  void (*write_clipboard)(void* userdata, int clipboard, ghostty_clipboard_content_s* content, size_t count, bool);
  
  // Close surface
  void (*close_surface)(void* userdata, bool);
} ghostty_runtime_config_s;
```

## How Supacode Likely Uses It

1. **Create main window** with grid of NSViews
2. **Initialize libghostty** once with `ghostty_init()`
3. **Create app** with `ghostty_app_new()` providing callbacks
4. **For each agent terminal:**
   - Create NSView
   - Call `ghostty_surface_new()` with that view
   - Set `command` to the coding agent (claude-code, codex, etc.)
   - Set `working_directory` to the git worktree
5. **Route input** to focused surface via `ghostty_surface_key()`
6. **Render** happens automatically via Metal to each NSView

## Building libghostty

```bash
git clone https://github.com/ghostty-org/ghostty
cd ghostty

# Build for embedding (creates libghostty)
zig build -Dapp-runtime=embedded

# Output: zig-out/lib/libghostty.a (or .dylib)
```

Requires: Zig 0.13+, Xcode 26 SDK

## Integration Options for Rembrandt

### Option A: Link libghostty directly
- Build Ghostty as static lib
- Link into Tauri app (Rust can call C)
- Replace xterm.js with Metal-rendered terminals

**Pros:** Native performance, full Ghostty features
**Cons:** macOS only, complex build, large binary

### Option B: Hybrid approach
- Use libghostty for macOS native app
- Keep xterm.js for cross-platform web UI
- Same Rust backend, different frontends

**Pros:** Best of both worlds
**Cons:** Two codepaths to maintain

### Option C: Study and adapt patterns
- Learn from Ghostty's PTY handling
- Apply their session management approach
- Keep your own rendering (xterm.js/Tauri)

**Pros:** Cross-platform, lighter weight
**Cons:** Don't get Ghostty's rendering quality

## Next Steps

1. [ ] Build libghostty locally
2. [ ] Create minimal Swift app that embeds one terminal
3. [ ] Add multiple terminals in grid
4. [ ] Wire up git worktree isolation
5. [ ] Add agent spawning

## References

- Ghostty source: https://github.com/ghostty-org/ghostty
- Ghostty docs: https://ghostty.org/docs
- libghostty header: `include/ghostty.h`
- Embedding API: `src/apprt/embedded.zig`

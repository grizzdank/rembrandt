# PTY Refactor Handoff

**Issue:** rembrandt-omt (P0)
**Date:** 2026-01-04

## Context

The TUI dashboard works (spawn, kill, nudge, cleanup, help). But **attach mode is broken** - users cannot connect to a running agent's terminal.

## The Problem

Current architecture in `src/daemon/session.rs`:
```
PtySession::spawn()
  → spawns background reader thread
  → thread reads PTY output forever
  → writes to RingBuffer for TUI display
```

When we try to attach:
```
attach_to_session()
  → try_clone_reader() to get PTY reader
  → but background thread already consuming all output!
  → attach sees nothing, keyboard doesn't work
```

## The Solution

**Remove background thread, implement on-demand reading:**

### Step 1: Modify `src/daemon/session.rs`

Remove from `PtySession::spawn()`:
- `_reader_handle: Option<JoinHandle<()>>` field
- The `thread::spawn(move || { Self::reader_loop(...) })` call
- The `reader_loop` function

Add new method:
```rust
/// Read available PTY output into the buffer (non-blocking)
pub fn read_available(&mut self) -> Result<usize> {
    // Set reader to non-blocking
    // Read whatever is available
    // Write to ring buffer
    // Return bytes read
}
```

### Step 2: Make PTY reader non-blocking

In `spawn()`, after getting the reader:
```rust
// Make reader non-blocking for polling
use std::os::unix::io::AsRawFd;
let fd = reader.as_raw_fd();
unsafe {
    let flags = libc::fcntl(fd, libc::F_GETFL);
    libc::fcntl(fd, libc::F_SETFL, flags | libc::O_NONBLOCK);
}
```

### Step 3: Update TUI to poll

In the main loop or render cycle, call `session.read_available()` periodically to update the buffer.

### Step 4: Rewrite `src/tui/attach.rs`

With no background thread, attach gets exclusive access:
```rust
pub fn attach_to_session(...) {
    // Leave alternate screen
    execute!(stdout, LeaveAlternateScreen);

    // Get exclusive PTY reader (no competition now!)
    let mut pty_reader = session.try_clone_reader()?;

    // Simple loop:
    loop {
        // Poll stdin and PTY reader (use select/poll)
        // stdin → PTY writer
        // PTY reader → stdout
        // Ctrl+] → break
    }

    // Re-enter alternate screen
    execute!(stdout, EnterAlternateScreen);
}
```

## Key Files

| File | What to do |
|------|------------|
| `src/daemon/session.rs` | Remove background thread, add `read_available()` |
| `src/daemon/buffer.rs` | May need to expose write method publicly |
| `src/tui/attach.rs` | Rewrite with exclusive PTY access |
| `src/tui/mod.rs` | Re-enable attach in event loop |
| `src/tui/events.rs` | Re-enable Enter → Attach |

## Testing

1. `cargo run --release -- dashboard`
2. Press `s`, select Claude Code, press Enter
3. Should see agent in list with status
4. Press Enter to attach
5. Claude Code should render correctly
6. Keyboard should work
7. Ctrl+] should detach back to dashboard

## Dependencies Already Added

- `libc = "0.2"` - for fcntl/non-blocking
- `strip-ansi-escapes = "0.2"` - for clean output display

## Estimate

~3-4 hours focused work

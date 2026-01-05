# Tauri + xterm.js Migration Plan

**Date:** 2026-01-04
**Goal:** Replace ratatui TUI with Tauri desktop app using Svelte + xterm.js
**Rationale:** Terminal-in-terminal attach is fundamentally broken; GUI sidesteps the problem entirely

---

## Pre-Migration

### Step 0: Preserve Current State
```bash
git checkout -b tui-ratatui-backup
git push origin tui-ratatui-backup
git checkout main
```
> Preserves TUI implementation for potential libghostty revisit later

---

## Phase 1: Tauri Scaffold

### Step 1.1: Initialize Tauri + Svelte
```bash
npm create tauri-app@latest rembrandt-gui -- --template svelte-ts
```
Or add Tauri to existing project:
```bash
cd rembrandt
npm create tauri-app@latest -- --template svelte-ts
# Move src-tauri into project, merge configs
```

### Step 1.2: Project Structure
```
rembrandt/
├── src-tauri/           # Rust backend (Tauri)
│   ├── src/
│   │   ├── main.rs      # Tauri entry point
│   │   ├── commands.rs  # Tauri command handlers
│   │   ├── session.rs   # Moved from daemon/session.rs
│   │   ├── manager.rs   # Moved from daemon/manager.rs
│   │   ├── buffer.rs    # Moved from daemon/buffer.rs
│   │   ├── worktree.rs  # Moved from worktree/mod.rs
│   │   └── agent.rs     # Moved from agent/mod.rs
│   └── Cargo.toml
├── src/                 # Svelte frontend
│   ├── lib/
│   │   ├── Terminal.svelte    # xterm.js wrapper
│   │   ├── AgentList.svelte   # Agent sidebar
│   │   ├── Dashboard.svelte   # Main layout
│   │   └── stores.ts          # Svelte stores for state
│   ├── App.svelte
│   └── main.ts
├── package.json
└── src-cli/             # Optional: keep CLI as separate binary
```

---

## Phase 2: Backend Migration

### Step 2.1: Move Reusable Code
These files move to `src-tauri/src/` with minimal changes:

| From | To | Changes |
|------|-----|---------|
| `src/daemon/session.rs` | `src-tauri/src/session.rs` | Remove TUI imports |
| `src/daemon/manager.rs` | `src-tauri/src/manager.rs` | Remove TUI imports |
| `src/daemon/buffer.rs` | `src-tauri/src/buffer.rs` | None |
| `src/worktree/mod.rs` | `src-tauri/src/worktree.rs` | None |
| `src/agent/mod.rs` | `src-tauri/src/agent.rs` | None |

### Step 2.2: Tauri Commands
Create `src-tauri/src/commands.rs`:

```rust
use tauri::State;
use std::sync::Mutex;

// State managed by Tauri
pub struct AppState {
    pub sessions: Mutex<SessionManager>,
    pub worktrees: Mutex<WorktreeManager>,
}

#[tauri::command]
pub fn spawn_agent(
    state: State<AppState>,
    agent_type: String,
    task: Option<String>,
) -> Result<String, String> {
    // Returns agent_id
}

#[tauri::command]
pub fn list_agents(state: State<AppState>) -> Vec<AgentInfo> { }

#[tauri::command]
pub fn kill_agent(state: State<AppState>, agent_id: String) -> Result<(), String> { }

#[tauri::command]
pub fn nudge_agent(state: State<AppState>, agent_id: String) -> Result<(), String> { }

#[tauri::command]
pub fn write_to_agent(
    state: State<AppState>,
    agent_id: String,
    data: Vec<u8>,
) -> Result<(), String> { }

#[tauri::command]
pub fn resize_agent(
    state: State<AppState>,
    agent_id: String,
    cols: u16,
    rows: u16,
) -> Result<(), String> { }

#[tauri::command]
pub fn get_history(
    state: State<AppState>,
    agent_id: String,
) -> Result<Vec<u8>, String> { }
```

### Step 2.3: PTY Output Streaming
Use Tauri events to push PTY output to frontend:

```rust
// In background task per session
loop {
    if let Some(data) = session.read_available()? {
        app_handle.emit_all("pty-output", PtyOutputEvent {
            agent_id: agent_id.clone(),
            data,
        })?;
    }
    tokio::time::sleep(Duration::from_millis(16)).await; // ~60fps
}
```

---

## Phase 3: Frontend Implementation

### Step 3.1: Dependencies
```bash
npm install xterm @xterm/addon-fit @xterm/addon-webgl
npm install -D @tauri-apps/api
```

### Step 3.2: Terminal Component
`src/lib/Terminal.svelte`:

```svelte
<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { Terminal } from 'xterm';
  import { FitAddon } from '@xterm/addon-fit';
  import { WebglAddon } from '@xterm/addon-webgl';
  import { invoke } from '@tauri-apps/api/tauri';
  import { listen } from '@tauri-apps/api/event';

  export let agentId: string;

  let terminalEl: HTMLDivElement;
  let term: Terminal;
  let unlistenOutput: () => void;

  onMount(async () => {
    term = new Terminal({
      cursorBlink: true,
      fontFamily: 'JetBrains Mono, monospace',
      fontSize: 14,
    });

    const fitAddon = new FitAddon();
    term.loadAddon(fitAddon);
    term.loadAddon(new WebglAddon());

    term.open(terminalEl);
    fitAddon.fit();

    // Get history for late-attach
    const history = await invoke<number[]>('get_history', { agentId });
    term.write(new Uint8Array(history));

    // Subscribe to PTY output
    unlistenOutput = await listen<{ agent_id: string; data: number[] }>(
      'pty-output',
      (event) => {
        if (event.payload.agent_id === agentId) {
          term.write(new Uint8Array(event.payload.data));
        }
      }
    );

    // Send input to PTY
    term.onData((data) => {
      invoke('write_to_agent', { agentId, data: [...new TextEncoder().encode(data)] });
    });

    // Handle resize
    term.onResize(({ cols, rows }) => {
      invoke('resize_agent', { agentId, cols, rows });
    });
  });

  onDestroy(() => {
    unlistenOutput?.();
    term?.dispose();
  });
</script>

<div bind:this={terminalEl} class="terminal-container"></div>

<style>
  .terminal-container {
    height: 100%;
    width: 100%;
  }
</style>
```

### Step 3.3: Dashboard Layout
`src/lib/Dashboard.svelte`:

```svelte
<script lang="ts">
  import { invoke } from '@tauri-apps/api/tauri';
  import { listen } from '@tauri-apps/api/event';
  import AgentList from './AgentList.svelte';
  import Terminal from './Terminal.svelte';
  import { agents, selectedAgent } from './stores';

  // Poll agents periodically or subscribe to status events
  async function refreshAgents() {
    $agents = await invoke('list_agents');
  }

  async function spawnAgent(type: string) {
    const agentId = await invoke('spawn_agent', { agentType: type });
    await refreshAgents();
    $selectedAgent = agentId;
  }
</script>

<div class="dashboard">
  <aside class="sidebar">
    <AgentList
      {agents}
      {selectedAgent}
      on:select={(e) => $selectedAgent = e.detail}
      on:spawn={(e) => spawnAgent(e.detail)}
    />
  </aside>

  <main class="terminal-pane">
    {#if $selectedAgent}
      <Terminal agentId={$selectedAgent} />
    {:else}
      <div class="empty-state">Select or spawn an agent</div>
    {/if}
  </main>
</div>
```

### Step 3.4: Agent Sidebar
`src/lib/AgentList.svelte`:

```svelte
<script lang="ts">
  import { createEventDispatcher } from 'svelte';
  import type { AgentInfo } from './types';

  export let agents: AgentInfo[];
  export let selectedAgent: string | null;

  const dispatch = createEventDispatcher();
</script>

<div class="agent-list">
  <button on:click={() => dispatch('spawn', 'claude')}>+ Spawn Agent</button>

  {#each agents as agent}
    <div
      class="agent-item"
      class:selected={agent.id === selectedAgent}
      on:click={() => dispatch('select', agent.id)}
    >
      <span class="status-dot" class:running={agent.status === 'running'}></span>
      <span class="agent-id">{agent.id}</span>
    </div>
  {/each}
</div>
```

---

## Phase 4: Polish & Integration

### Step 4.1: Keyboard Shortcuts
- `Cmd+N` / `Ctrl+N`: Spawn new agent
- `Cmd+W` / `Ctrl+W`: Kill focused agent
- `Cmd+1-9`: Switch to agent by index
- `Cmd+Tab`: Cycle agents

### Step 4.2: Status Bar
- Show agent count, worktree status
- Integration status (beads, porque)

### Step 4.3: CLI Compatibility (Optional)
Keep `src-cli/` as separate binary that:
- Uses same backend logic (links to library)
- Provides `rembrandt spawn`, `rembrandt list`, etc.
- For users who prefer CLI

---

## Implementation Order

1. **Preserve branch** - `git checkout -b tui-ratatui-backup`
2. **Scaffold Tauri** - Initialize project structure
3. **Migrate session.rs** - Core PTY logic
4. **Create spawn command** - End-to-end agent creation
5. **Build Terminal.svelte** - xterm.js integration
6. **Wire PTY streaming** - Tauri events for output
7. **Build Dashboard** - Layout with agent list
8. **Add controls** - Kill, nudge, resize
9. **Polish UI** - Keyboard shortcuts, styling
10. **Optional CLI** - Keep CLI frontend working

---

## Files to Modify/Create

### New Files
- `src-tauri/src/main.rs`
- `src-tauri/src/commands.rs`
- `src-tauri/Cargo.toml`
- `src-tauri/tauri.conf.json`
- `src/App.svelte`
- `src/lib/Terminal.svelte`
- `src/lib/Dashboard.svelte`
- `src/lib/AgentList.svelte`
- `src/lib/stores.ts`
- `package.json`

### Files to Move
- `src/daemon/session.rs` → `src-tauri/src/session.rs`
- `src/daemon/manager.rs` → `src-tauri/src/manager.rs`
- `src/daemon/buffer.rs` → `src-tauri/src/buffer.rs`
- `src/worktree/mod.rs` → `src-tauri/src/worktree.rs`
- `src/agent/mod.rs` → `src-tauri/src/agent.rs`

### Files to Delete (after migration)
- `src/tui/*` (replaced by Svelte frontend)
- `src/daemon/ipc.rs` (replaced by Tauri IPC)

---

## Decisions Made

1. **CLI support:** Structure backend as library for potential future CLI/TUI, but focus on GUI now
2. **Window model:** Single window with sidebar tabs (not multi-window)
3. **Backup branch:** Preserve current TUI as `tui-ratatui-backup` for libghostty revisit

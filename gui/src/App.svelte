<script lang="ts">
  import { onMount, onDestroy } from 'svelte'
  import { invoke } from '@tauri-apps/api/core'
  import Terminal from './lib/Terminal.svelte'
  import AgentCard from './lib/AgentCard.svelte'
  import KanbanBoard from './lib/KanbanBoard.svelte'

  type ViewMode = 'list' | 'kanban'

  interface SessionStatus {
    type: 'Running' | 'Exited' | 'Failed'
    value?: number | string
  }

  interface SessionInfo {
    id: string
    agent_id: string
    command: string
    workdir: string
    status: SessionStatus
    created_at: string
    branch: string | null
    isolated: boolean
  }

  let sessions: SessionInfo[] = $state([])
  let activeSessionId: string | null = $state(null)
  let refreshInterval: number | undefined
  let isSpawning = $state(false)
  let viewMode: ViewMode = $state('list')

  // Track sessions scheduled for auto-kill (session id -> timeout id)
  let exitedSessions: Map<string, number> = new Map()
  const AUTO_KILL_DELAY = 3000 // 3 seconds after exit

  // Spawn dialog state
  let showSpawnDialog = $state(false)
  let spawnAgentId = $state('')
  let spawnCommand = $state('claude')
  let spawnWorkdir = $state('')
  let spawnIsolated = $state(true)
  let spawnBaseBranch = $state('main')
  let agentIdInput: HTMLInputElement | undefined

  onMount(async () => {
    await refreshSessions()
    // Poll for session updates every second
    refreshInterval = setInterval(refreshSessions, 1000)

    // Get current working directory as default
    try {
      const cwd = await getCurrentDir()
      spawnWorkdir = cwd
    } catch (e) {
      console.warn('Could not get cwd:', e)
    }
  })

  onDestroy(() => {
    if (refreshInterval) clearInterval(refreshInterval)
    // Clear any pending auto-kill timers
    exitedSessions.forEach(timeoutId => clearTimeout(timeoutId))
  })

  async function getCurrentDir(): Promise<string> {
    // Use shell to get cwd (Tauri doesn't expose this directly)
    return '.'
  }

  async function refreshSessions() {
    try {
      sessions = await invoke('list_agents')

      // Auto-kill: schedule removal for exited sessions
      for (const session of sessions) {
        if (session.status.type !== 'Running' && !exitedSessions.has(session.id)) {
          // Schedule this session for removal
          const timeoutId = setTimeout(() => {
            killAgent(session.id)
            exitedSessions.delete(session.id)
          }, AUTO_KILL_DELAY)
          exitedSessions.set(session.id, timeoutId)
        }
      }

      // If active session no longer exists, clear selection
      if (activeSessionId && !sessions.find(s => s.id === activeSessionId)) {
        activeSessionId = null
      }
    } catch (e) {
      console.error('Failed to list agents:', e)
    }
  }

  // Generate a unique agent ID suffix (4 hex chars)
  function generateIdSuffix(): string {
    return Array.from({ length: 4 }, () =>
      Math.floor(Math.random() * 16).toString(16)
    ).join('')
  }

  async function spawnAgent() {
    console.log('spawnAgent called', { spawnAgentId, spawnCommand, spawnWorkdir, spawnIsolated, spawnBaseBranch })
    if (!spawnAgentId.trim()) {
      console.log('Agent ID is empty, returning')
      return
    }

    isSpawning = true
    try {
      // Generate unique agent ID like CLI does: {base}-{random}
      const uniqueAgentId = `${spawnAgentId.trim()}-${generateIdSuffix()}`
      console.log('Calling invoke spawn_agent with unique ID:', uniqueAgentId)

      const sessionId: string = await invoke('spawn_agent', {
        agentId: uniqueAgentId,
        command: spawnCommand || 'claude',
        workdir: spawnWorkdir || '.',
        rows: 24,
        cols: 80,
        isolated: spawnIsolated,
        baseBranch: spawnBaseBranch || 'main',
      })
      console.log('Spawn succeeded, sessionId:', sessionId)
      activeSessionId = sessionId
      showSpawnDialog = false
      // Don't reset spawnAgentId - user might want to spawn another with same base name
      await refreshSessions()
    } catch (e) {
      console.error('Failed to spawn agent:', e)
      alert(`Failed to spawn: ${e}`)
    } finally {
      isSpawning = false
    }
  }

  async function killAgent(sid: string) {
    try {
      await invoke('kill_agent', { sessionId: sid })
      await refreshSessions()
    } catch (e) {
      console.error('Failed to kill agent:', e)
    }
  }

  async function nudgeAgent(sid: string) {
    try {
      await invoke('nudge_agent', { sessionId: sid })
    } catch (e) {
      console.error('Failed to nudge agent:', e)
    }
  }

  function selectSession(sessionId: string) {
    activeSessionId = sessionId
  }

  $effect(() => {
    // Auto-select first session if none selected
    if (!activeSessionId && sessions.length > 0) {
      activeSessionId = sessions[0].id
    }
  })
</script>

<div class="app">
  <aside class="sidebar">
    <div class="sidebar-header">
      <h1>Rembrandt</h1>
      <div class="header-actions">
        <div class="view-toggle">
          <button
            class="toggle-btn"
            class:active={viewMode === 'list'}
            onclick={() => viewMode = 'list'}
            title="List view"
          >☰</button>
          <button
            class="toggle-btn"
            class:active={viewMode === 'kanban'}
            onclick={() => viewMode = 'kanban'}
            title="Symphony view (Kanban)"
          >▦</button>
        </div>
        <button class="spawn-btn" onclick={() => showSpawnDialog = true}>
          + New Agent
        </button>
      </div>
    </div>

    <div class="sessions-list">
      {#each sessions as session (session.id)}
        <AgentCard
          {session}
          isActive={session.id === activeSessionId}
          onSelect={() => selectSession(session.id)}
          onKill={() => killAgent(session.id)}
          onNudge={() => nudgeAgent(session.id)}
        />
      {/each}

      {#if sessions.length === 0}
        <div class="empty-state">
          <p>No agents running</p>
          <p class="hint">Click "+ New Agent" to spawn one</p>
        </div>
      {/if}
    </div>

    <div class="sidebar-footer">
      <span class="session-count">
        {sessions.filter(s => s.status.type === 'Running').length} active / {sessions.length} total
      </span>
    </div>
  </aside>

  <main class="main-content">
    {#if viewMode === 'kanban'}
      <KanbanBoard
        {sessions}
        {activeSessionId}
        onSelectSession={selectSession}
        onKillSession={killAgent}
        onNudgeSession={nudgeAgent}
      />
    {:else if activeSessionId}
      <div class="terminal-wrapper">
        {#key activeSessionId}
          <Terminal sessionId={activeSessionId} />
        {/key}
      </div>
    {:else}
      <div class="no-terminal">
        <p>Select an agent or spawn a new one</p>
      </div>
    {/if}
  </main>
</div>

{#if showSpawnDialog}
  <!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
  <div class="dialog-overlay" onclick={() => showSpawnDialog = false} role="presentation">
    <div class="dialog" onclick={(e) => e.stopPropagation()} role="dialog" aria-modal="true" tabindex="-1">
      <h2>Spawn New Agent</h2>

      <form class="form-fields" onsubmit={(e) => { e.preventDefault(); spawnAgent(); }}>
        <label>
          <span>Agent Name</span>
          <input
            type="text"
            bind:value={spawnAgentId}
            bind:this={agentIdInput}
            placeholder="e.g., backend, frontend, api"
            autofocus
          />
          <span class="field-hint">A unique suffix will be added (e.g., backend-a3f2)</span>
        </label>

        <label>
          <span>Command</span>
          <input
            type="text"
            bind:value={spawnCommand}
            placeholder="claude"
          />
        </label>

        <label>
          <span>Working Directory</span>
          <input
            type="text"
            bind:value={spawnWorkdir}
            placeholder="."
          />
        </label>

        <div class="isolation-row">
          <label class="checkbox-label">
            <input
              type="checkbox"
              bind:checked={spawnIsolated}
            />
            <span>Isolate in git worktree</span>
          </label>

          {#if spawnIsolated}
            <label class="branch-label">
              <span>Base branch</span>
              <input
                type="text"
                bind:value={spawnBaseBranch}
                placeholder="main"
              />
            </label>
          {/if}
        </div>

        <div class="dialog-actions">
          <button type="button" onclick={() => showSpawnDialog = false}>
            Cancel
          </button>
          <button type="submit" class="primary" disabled={isSpawning || !spawnAgentId.trim()}>
            {isSpawning ? 'Spawning...' : 'Spawn'}
          </button>
        </div>
      </form>
    </div>
  </div>
{/if}

<style>
  /* Rembrandt-inspired color palette
     Warm earth tones with golden ochre accents
     Deep chiaroscuro contrast */
  :global(html, body) {
    margin: 0;
    padding: 0;
    width: 100%;
    height: 100%;
    background: #1c1a17;
    color: #f5f0e6;
    font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
  }

  :global(#app) {
    width: 100%;
    height: 100%;
  }

  .app {
    display: flex;
    height: 100vh;
    overflow: hidden;
  }

  .sidebar {
    width: 280px;
    background: #2a2520;
    border-right: 1px solid #4a3f38;
    display: flex;
    flex-direction: column;
    flex-shrink: 0;
  }

  .sidebar-header {
    padding: 16px;
    border-bottom: 1px solid #4a3f38;
    display: flex;
    align-items: center;
    justify-content: space-between;
  }

  .sidebar-header h1 {
    margin: 0;
    font-size: 18px;
    font-weight: 600;
    color: #cc7722;
  }

  .header-actions {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .view-toggle {
    display: flex;
    background: #3d3632;
    border-radius: 4px;
    padding: 2px;
  }

  .toggle-btn {
    background: transparent;
    border: none;
    padding: 4px 8px;
    font-size: 14px;
    cursor: pointer;
    color: #7a6f62;
    border-radius: 3px;
    transition: all 0.15s ease;
  }

  .toggle-btn:hover {
    color: #a89a85;
  }

  .toggle-btn.active {
    background: #cc7722;
    color: #1c1a17;
  }

  .spawn-btn {
    background: #cc7722;
    color: #1c1a17;
    border: none;
    padding: 8px 12px;
    border-radius: 4px;
    font-size: 13px;
    font-weight: 500;
    cursor: pointer;
    transition: background 0.15s ease;
  }

  .spawn-btn:hover {
    background: #dd8833;
  }

  .sessions-list {
    flex: 1;
    overflow-y: auto;
    padding: 12px;
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .empty-state {
    text-align: center;
    color: #7a6f62;
    padding: 32px 16px;
  }

  .empty-state p {
    margin: 0;
  }

  .empty-state .hint {
    font-size: 12px;
    margin-top: 8px;
  }

  .sidebar-footer {
    padding: 12px 16px;
    border-top: 1px solid #4a3f38;
    font-size: 12px;
    color: #7a6f62;
  }

  .session-count {
    font-family: 'JetBrains Mono', monospace;
  }

  .main-content {
    flex: 1;
    display: flex;
    min-width: 0;
    padding: 12px;
  }

  .terminal-wrapper {
    flex: 1;
    display: flex;
    min-width: 0;
    min-height: 0;
  }

  .no-terminal {
    flex: 1;
    display: flex;
    align-items: center;
    justify-content: center;
    color: #7a6f62;
    font-size: 14px;
  }

  /* Dialog styles */
  .dialog-overlay {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.75);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 100;
  }

  .dialog {
    background: #2a2520;
    border: 1px solid #4a3f38;
    border-radius: 4px;
    padding: 24px;
    width: 400px;
    max-width: 90vw;
  }

  .dialog h2 {
    margin: 0 0 20px 0;
    font-size: 18px;
    font-weight: 600;
    color: #cc7722;
  }

  .dialog .form-fields {
    display: flex;
    flex-direction: column;
    gap: 16px;
  }

  .dialog label {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .dialog label span {
    font-size: 13px;
    color: #a89a85;
  }

  .field-hint {
    font-size: 11px !important;
    color: #6e5d52 !important;
    font-style: italic;
  }

  .dialog input {
    background: #1c1a17;
    border: 1px solid #4a3f38;
    border-radius: 4px;
    padding: 10px 12px;
    font-size: 14px;
    color: #f5f0e6;
  }

  .dialog input:focus {
    outline: none;
    border-color: #cc7722;
  }

  .dialog-actions {
    display: flex;
    gap: 12px;
    justify-content: flex-end;
    margin-top: 8px;
  }

  .dialog-actions button {
    padding: 10px 16px;
    border-radius: 4px;
    font-size: 14px;
    font-weight: 500;
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .dialog-actions button:not(.primary) {
    background: transparent;
    border: 1px solid #4a3f38;
    color: #a89a85;
  }

  .dialog-actions button:not(.primary):hover {
    background: #3d3632;
  }

  .dialog-actions button.primary {
    background: #cc7722;
    border: none;
    color: #1c1a17;
  }

  .dialog-actions button.primary:hover:not(:disabled) {
    background: #dd8833;
  }

  .dialog-actions button:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .isolation-row {
    display: flex;
    flex-direction: column;
    gap: 12px;
    padding: 12px;
    background: #1c1a17;
    border-radius: 4px;
    border: 1px solid #4a3f38;
  }

  .checkbox-label {
    display: flex;
    align-items: center;
    gap: 8px;
    cursor: pointer;
  }

  .checkbox-label input[type="checkbox"] {
    width: 16px;
    height: 16px;
    accent-color: #cc7722;
    cursor: pointer;
  }

  .checkbox-label span {
    font-size: 13px;
    color: #f5f0e6;
  }

  .branch-label {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-left: 24px;
  }

  .branch-label span {
    font-size: 12px;
    color: #a89a85;
    white-space: nowrap;
  }

  .branch-label input {
    flex: 1;
    background: #2a2520;
    border: 1px solid #4a3f38;
    border-radius: 4px;
    padding: 6px 10px;
    font-size: 13px;
    color: #f5f0e6;
  }

  .branch-label input:focus {
    outline: none;
    border-color: #cc7722;
  }
</style>

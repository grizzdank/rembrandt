<script lang="ts">
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

  interface Props {
    sessions: SessionInfo[]
    activeSessionId: string | null
    onSelectSession: (id: string) => void
    onKillSession: (id: string) => void
    onNudgeSession: (id: string) => void
  }

  let { sessions, activeSessionId, onSelectSession, onKillSession, onNudgeSession }: Props = $props()

  // Categorize sessions into kanban columns
  const columns = $derived(() => {
    const running: SessionInfo[] = []
    const blocked: SessionInfo[] = []  // TODO: wire up blocked detection
    const done: SessionInfo[] = []
    const failed: SessionInfo[] = []

    for (const session of sessions) {
      if (session.status.type === 'Running') {
        running.push(session)
      } else if (session.status.type === 'Exited') {
        if (session.status.value === 0) {
          done.push(session)
        } else {
          failed.push(session)
        }
      } else if (session.status.type === 'Failed') {
        failed.push(session)
      }
    }

    return { running, blocked, done, failed }
  })

  function getStatusIcon(status: SessionStatus): string {
    if (status.type === 'Running') return '●'
    if (status.type === 'Exited' && status.value === 0) return '✓'
    return '✕'
  }

  function formatTime(isoString: string): string {
    try {
      const date = new Date(isoString)
      return date.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' })
    } catch {
      return ''
    }
  }
</script>

<div class="kanban-board">
  <div class="column running">
    <div class="column-header">
      <span class="column-dot running"></span>
      <span class="column-title">Running</span>
      <span class="column-count">{columns().running.length}</span>
    </div>
    <div class="column-content">
      {#each columns().running as session (session.id)}
        <div
          class="kanban-card"
          class:active={session.id === activeSessionId}
          onclick={() => onSelectSession(session.id)}
          onkeydown={(e) => e.key === 'Enter' && onSelectSession(session.id)}
          role="button"
          tabindex="0"
        >
          <div class="card-header">
            <span class="card-icon">{getStatusIcon(session.status)}</span>
            <span class="card-title">{session.agent_id}</span>
          </div>
          <div class="card-meta">
            <span class="card-command">{session.command}</span>
            {#if session.branch}
              <span class="card-branch">⎇ {session.branch.replace('rembrandt/', '')}</span>
            {/if}
          </div>
          <div class="card-footer">
            <span class="card-time">{formatTime(session.created_at)}</span>
            <div class="card-actions">
              <button class="action-btn" onclick={(e) => { e.stopPropagation(); onNudgeSession(session.id); }} title="Nudge">↵</button>
              <button class="action-btn danger" onclick={(e) => { e.stopPropagation(); onKillSession(session.id); }} title="Kill">✕</button>
            </div>
          </div>
        </div>
      {/each}
      {#if columns().running.length === 0}
        <div class="empty-column">No running agents</div>
      {/if}
    </div>
  </div>

  <div class="column blocked">
    <div class="column-header">
      <span class="column-dot blocked"></span>
      <span class="column-title">Blocked</span>
      <span class="column-count">{columns().blocked.length}</span>
    </div>
    <div class="column-content">
      {#each columns().blocked as session (session.id)}
        <div
          class="kanban-card"
          class:active={session.id === activeSessionId}
          onclick={() => onSelectSession(session.id)}
          onkeydown={(e) => e.key === 'Enter' && onSelectSession(session.id)}
          role="button"
          tabindex="0"
        >
          <div class="card-header">
            <span class="card-icon blocked">⏸</span>
            <span class="card-title">{session.agent_id}</span>
          </div>
          <div class="card-meta">
            <span class="card-command">{session.command}</span>
            {#if session.branch}
              <span class="card-branch">⎇ {session.branch.replace('rembrandt/', '')}</span>
            {/if}
          </div>
          <div class="card-footer">
            <span class="card-time">{formatTime(session.created_at)}</span>
            <div class="card-actions">
              <button class="action-btn" onclick={(e) => { e.stopPropagation(); onNudgeSession(session.id); }} title="Nudge">↵</button>
            </div>
          </div>
        </div>
      {/each}
      {#if columns().blocked.length === 0}
        <div class="empty-column">No blocked agents</div>
      {/if}
    </div>
  </div>

  <div class="column done">
    <div class="column-header">
      <span class="column-dot done"></span>
      <span class="column-title">Done</span>
      <span class="column-count">{columns().done.length}</span>
    </div>
    <div class="column-content">
      {#each columns().done as session (session.id)}
        <div
          class="kanban-card"
          class:active={session.id === activeSessionId}
          onclick={() => onSelectSession(session.id)}
          onkeydown={(e) => e.key === 'Enter' && onSelectSession(session.id)}
          role="button"
          tabindex="0"
        >
          <div class="card-header">
            <span class="card-icon done">{getStatusIcon(session.status)}</span>
            <span class="card-title">{session.agent_id}</span>
          </div>
          <div class="card-meta">
            <span class="card-command">{session.command}</span>
            {#if session.branch}
              <span class="card-branch">⎇ {session.branch.replace('rembrandt/', '')}</span>
            {/if}
          </div>
          <div class="card-footer">
            <span class="card-time">{formatTime(session.created_at)}</span>
            <div class="card-actions">
              <button class="action-btn danger" onclick={(e) => { e.stopPropagation(); onKillSession(session.id); }} title="Remove">✕</button>
            </div>
          </div>
        </div>
      {/each}
      {#if columns().done.length === 0}
        <div class="empty-column">No completed agents</div>
      {/if}
    </div>
  </div>

  <div class="column failed">
    <div class="column-header">
      <span class="column-dot failed"></span>
      <span class="column-title">Failed</span>
      <span class="column-count">{columns().failed.length}</span>
    </div>
    <div class="column-content">
      {#each columns().failed as session (session.id)}
        <div
          class="kanban-card"
          class:active={session.id === activeSessionId}
          onclick={() => onSelectSession(session.id)}
          onkeydown={(e) => e.key === 'Enter' && onSelectSession(session.id)}
          role="button"
          tabindex="0"
        >
          <div class="card-header">
            <span class="card-icon failed">{getStatusIcon(session.status)}</span>
            <span class="card-title">{session.agent_id}</span>
          </div>
          <div class="card-meta">
            <span class="card-command">{session.command}</span>
            {#if session.branch}
              <span class="card-branch">⎇ {session.branch.replace('rembrandt/', '')}</span>
            {/if}
          </div>
          <div class="card-footer">
            <span class="card-time">{formatTime(session.created_at)}</span>
            <div class="card-actions">
              <button class="action-btn danger" onclick={(e) => { e.stopPropagation(); onKillSession(session.id); }} title="Remove">✕</button>
            </div>
          </div>
        </div>
      {/each}
      {#if columns().failed.length === 0}
        <div class="empty-column">No failed agents</div>
      {/if}
    </div>
  </div>
</div>

<style>
  .kanban-board {
    display: flex;
    gap: 16px;
    height: 100%;
    padding: 16px;
    overflow-x: auto;
  }

  .column {
    flex: 1;
    min-width: 220px;
    max-width: 300px;
    display: flex;
    flex-direction: column;
    background: #2a2520;
    border-radius: 8px;
    border: 1px solid #4a3f38;
  }

  .column-header {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 12px 16px;
    border-bottom: 1px solid #4a3f38;
  }

  .column-dot {
    width: 10px;
    height: 10px;
    border-radius: 50%;
  }

  .column-dot.running { background: #8a9a5b; }
  .column-dot.blocked { background: #cc7722; }
  .column-dot.done { background: #5d7a8c; }
  .column-dot.failed { background: #c94c4c; }

  .column-title {
    font-weight: 600;
    font-size: 14px;
    color: #f5f0e6;
    flex: 1;
  }

  .column-count {
    font-size: 12px;
    color: #7a6f62;
    background: #3d3632;
    padding: 2px 8px;
    border-radius: 10px;
    font-family: 'JetBrains Mono', monospace;
  }

  .column-content {
    flex: 1;
    overflow-y: auto;
    padding: 12px;
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .kanban-card {
    background: #3d3632;
    border: 1px solid #4a3f38;
    border-radius: 6px;
    padding: 12px;
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .kanban-card:hover {
    background: #4a3f38;
    border-color: #6e5d52;
  }

  .kanban-card.active {
    border-color: #cc7722;
    box-shadow: 0 0 0 1px #cc7722;
  }

  .card-header {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-bottom: 6px;
  }

  .card-icon {
    font-size: 10px;
    color: #8a9a5b;
  }

  .card-icon.blocked { color: #cc7722; }
  .card-icon.done { color: #5d7a8c; }
  .card-icon.failed { color: #c94c4c; }

  .card-title {
    font-weight: 600;
    font-size: 13px;
    color: #f5f0e6;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .card-meta {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 6px;
    margin-bottom: 8px;
  }

  .card-command {
    font-size: 11px;
    color: #a89a85;
    font-family: 'JetBrains Mono', monospace;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .card-branch {
    font-size: 10px;
    color: #cc7722;
    font-family: 'JetBrains Mono', monospace;
    background: #4a3f38;
    padding: 1px 6px;
    border-radius: 3px;
  }

  .card-footer {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }

  .card-time {
    font-size: 11px;
    color: #7a6f62;
  }

  .card-actions {
    display: flex;
    gap: 4px;
  }

  .action-btn {
    background: transparent;
    border: 1px solid #6e5d52;
    border-radius: 4px;
    padding: 2px 6px;
    font-size: 11px;
    cursor: pointer;
    color: #a89a85;
    transition: all 0.15s ease;
  }

  .action-btn:hover {
    background: #4a3f38;
    border-color: #8a9a5b;
    color: #8a9a5b;
  }

  .action-btn.danger:hover {
    border-color: #c94c4c;
    color: #c94c4c;
  }

  .empty-column {
    text-align: center;
    color: #6e5d52;
    font-size: 12px;
    padding: 24px 12px;
    font-style: italic;
  }
</style>

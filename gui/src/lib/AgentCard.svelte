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
  }

  interface Props {
    session: SessionInfo
    isActive: boolean
    onSelect: () => void
    onKill: () => void
    onNudge: () => void
  }

  let { session, isActive, onSelect, onKill, onNudge }: Props = $props()

  function getStatusColor(status: SessionStatus): string {
    if (status.type === 'Running') return '#8a9a5b'  // rembrandt muted green
    if (status.type === 'Exited' && status.value === 0) return '#5d7a8c'  // rembrandt muted blue
    return '#c94c4c'  // rembrandt vermillion
  }

  function getStatusLabel(status: SessionStatus): string {
    if (status.type === 'Running') return 'Running'
    if (status.type === 'Exited') {
      return status.value === 0 ? 'Done' : `Exit ${status.value}`
    }
    if (status.type === 'Failed') return 'Failed'
    return 'Unknown'
  }

  function isRunning(status: SessionStatus): boolean {
    return status.type === 'Running'
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

<div
  class="agent-card"
  class:active={isActive}
  onclick={onSelect}
  onkeydown={(e) => e.key === 'Enter' && onSelect()}
  role="button"
  tabindex="0"
>
  <div class="header">
    <span class="status-dot" style:background-color={getStatusColor(session.status)}></span>
    <span class="agent-id">{session.agent_id}</span>
    <span class="status-label">{getStatusLabel(session.status)}</span>
  </div>

  <div class="command">{session.command}</div>

  <div class="footer">
    <span class="time">{formatTime(session.created_at)}</span>
    <div class="actions">
      {#if isRunning(session.status)}
        <button class="action-btn nudge" onclick={(e) => { e.stopPropagation(); onNudge(); }} title="Nudge (send Enter)">
          ↵
        </button>
      {/if}
      <button class="action-btn kill" onclick={(e) => { e.stopPropagation(); onKill(); }} title="Kill session">
        ✕
      </button>
    </div>
  </div>
</div>

<style>
  /* Rembrandt-inspired warm earth tones */
  .agent-card {
    display: flex;
    flex-direction: column;
    gap: 6px;
    padding: 12px;
    background: #3d3632;
    border: 1px solid #4a3f38;
    border-radius: 4px;
    cursor: pointer;
    transition: all 0.15s ease;
    text-align: left;
    width: 100%;
  }

  .agent-card:hover {
    background: #4a3f38;
    border-color: #6e5d52;
  }

  .agent-card.active {
    background: #4a3f38;
    border-color: #c9a227;
    box-shadow: 0 0 0 1px #c9a227;
  }

  .header {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .status-dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    flex-shrink: 0;
  }

  .agent-id {
    font-weight: 600;
    font-size: 13px;
    color: #f5f0e6;
    flex-grow: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .status-label {
    font-size: 11px;
    color: #a89a85;
    flex-shrink: 0;
  }

  .command {
    font-size: 11px;
    color: #a89a85;
    font-family: 'JetBrains Mono', monospace;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .footer {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-top: 4px;
  }

  .time {
    font-size: 11px;
    color: #7a6f62;
  }

  .actions {
    display: flex;
    gap: 4px;
  }

  .action-btn {
    background: transparent;
    border: 1px solid #6e5d52;
    border-radius: 4px;
    padding: 2px 6px;
    font-size: 12px;
    cursor: pointer;
    color: #a89a85;
    transition: all 0.15s ease;
  }

  .action-btn:hover {
    background: #4a3f38;
  }

  .action-btn.kill:hover {
    border-color: #c94c4c;
    color: #c94c4c;
  }

  .action-btn.nudge:hover {
    border-color: #8a9a5b;
    color: #8a9a5b;
  }
</style>

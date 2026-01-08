<script lang="ts">
  import { onMount, onDestroy } from 'svelte'
  import { Terminal } from '@xterm/xterm'
  import { FitAddon } from '@xterm/addon-fit'
  import { WebglAddon } from '@xterm/addon-webgl'
  import '@xterm/xterm/css/xterm.css'
  import { invoke } from '@tauri-apps/api/core'

  interface Props {
    sessionId: string
    onData?: (data: string) => void
  }

  let { sessionId, onData }: Props = $props()

  let terminalElement: HTMLDivElement
  let terminal: Terminal
  let fitAddon: FitAddon
  let pollInterval: number | undefined
  let outputOffset = 0
  let resizeObserver: ResizeObserver | undefined
  let resizeTimeout: number | undefined

  onMount(async () => {
    terminal = new Terminal({
      // Nerd Fonts have excellent box-drawing and icon support
      fontFamily: '"JetBrainsMono Nerd Font", "Hack Nerd Font", "SF Mono", Monaco, Menlo, monospace',
      fontSize: 14,
      // Allow fallback for box-drawing characters
      allowProposedApi: true,
      // Rembrandt-inspired theme: warm earth tones with golden highlights
      theme: {
        background: '#1c1a17',
        foreground: '#f5f0e6',
        cursor: '#c9a227',
        cursorAccent: '#1c1a17',
        selectionBackground: '#4a3f38',
        selectionForeground: '#f5f0e6',
        black: '#1c1a17',
        red: '#c94c4c',
        green: '#8a9a5b',
        yellow: '#c9a227',
        blue: '#5d7a8c',
        magenta: '#a67c52',
        cyan: '#7a9a8a',
        white: '#f5f0e6',
        brightBlack: '#6e5d52',
        brightRed: '#d4605b',
        brightGreen: '#9aaa6b',
        brightYellow: '#dbb842',
        brightBlue: '#6d8a9c',
        brightMagenta: '#b68c62',
        brightCyan: '#8aaa9a',
        brightWhite: '#fffbf0',
      },
      cursorBlink: true,
      scrollback: 10000,
    })

    fitAddon = new FitAddon()
    terminal.loadAddon(fitAddon)

    terminal.open(terminalElement)

    // Try WebGL for better performance, fall back to canvas
    try {
      terminal.loadAddon(new WebglAddon())
    } catch (e) {
      console.warn('WebGL addon failed to load, using canvas renderer')
    }

    // Wait for next frame to ensure container has dimensions
    await new Promise(resolve => requestAnimationFrame(resolve))

    // Fit and verify we have valid dimensions
    fitAddon.fit()

    // If dimensions are invalid (container not sized yet), retry after a short delay
    if (terminal.cols < 2 || terminal.rows < 2) {
      console.warn('Terminal has invalid dimensions, retrying fit...')
      await new Promise(resolve => setTimeout(resolve, 100))
      fitAddon.fit()
    }

    console.log('Initial terminal size:', terminal.cols, 'x', terminal.rows)

    // Send initial size to backend
    await invoke('resize_agent', {
      sessionId: sessionId,
      cols: terminal.cols,
      rows: terminal.rows,
    }).catch(console.error)

    // Handle keyboard input - send to PTY
    // Use fire-and-forget to avoid blocking keystroke processing
    terminal.onData((data) => {
      const bytes = [...new TextEncoder().encode(data)]
      console.log('Sending to PTY:', bytes, 'chars:', data.split('').map(c => c.charCodeAt(0)))

      // Fire-and-forget - don't await to prevent keystroke loss
      invoke('write_to_agent', { sessionId: sessionId, data: bytes })
        .catch(e => console.error('Failed to write to agent:', e))

      onData?.(data)
    })

    // Load initial history
    await loadHistory()

    // Start polling for new output
    startPolling()

    // Handle window and container resize
    window.addEventListener('resize', handleResize)

    // Use ResizeObserver for container size changes
    resizeObserver = new ResizeObserver(() => {
      // Debounce resize
      if (resizeTimeout) clearTimeout(resizeTimeout)
      resizeTimeout = setTimeout(handleResize, 100)
    })
    resizeObserver.observe(terminalElement)
  })

  onDestroy(() => {
    if (pollInterval) clearInterval(pollInterval)
    if (resizeTimeout) clearTimeout(resizeTimeout)
    resizeObserver?.disconnect()
    window.removeEventListener('resize', handleResize)
    terminal?.dispose()
  })

  async function loadHistory() {
    try {
      console.log('Loading history for session:', sessionId)
      const history: number[] = await invoke('get_history', { sessionId: sessionId })
      console.log('Got history, length:', history.length)
      if (history.length > 0) {
        const text = new TextDecoder().decode(new Uint8Array(history))
        console.log('History text:', text.substring(0, 100))
        terminal.write(text)
        outputOffset = history.length
      }
    } catch (e) {
      console.error('Failed to load history:', e)
    }
  }

  function startPolling() {
    // Poll for new output every 50ms
    // TODO: Replace with Tauri event streaming for better performance
    pollInterval = setInterval(pollOutput, 50)
  }

  async function pollOutput() {
    try {
      const history: number[] = await invoke('get_history', { sessionId: sessionId })
      if (history.length > outputOffset) {
        // New data available - write the delta
        const newData = history.slice(outputOffset)
        const text = new TextDecoder().decode(new Uint8Array(newData))
        console.log('New output:', text.length, 'bytes')
        terminal.write(text)
        outputOffset = history.length
      }
    } catch (e) {
      console.error('Poll error:', e)
      // Session may have ended
      if (pollInterval) clearInterval(pollInterval)
    }
  }

  function handleResize() {
    if (fitAddon && terminal) {
      fitAddon.fit()
      const cols = terminal.cols
      const rows = terminal.rows

      // Only send resize if we have valid dimensions
      if (cols < 2 || rows < 2) {
        console.warn('Skipping resize - invalid dimensions:', cols, 'x', rows)
        return
      }

      console.log('Resizing terminal to', cols, 'x', rows)
      // Notify backend of new dimensions
      invoke('resize_agent', {
        sessionId: sessionId,
        cols,
        rows,
      }).catch(console.error)
    }
  }

  export function focus() {
    terminal?.focus()
  }

  export function clear() {
    terminal?.clear()
  }
</script>

<div class="terminal-container" bind:this={terminalElement}></div>

<style>
  .terminal-container {
    flex: 1;
    width: 100%;
    height: 100%;
    min-width: 0;
    min-height: 0;
    background: #1c1a17;
    border-radius: 4px;
    overflow: hidden;
  }

  :global(.xterm) {
    padding: 8px;
    height: 100%;
  }

  :global(.xterm-viewport) {
    overflow-y: auto !important;
  }
</style>

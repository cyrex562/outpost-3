import { describe, expect, it, vi, afterEach } from 'vitest'
import { logFrontendError } from '@/services/tauriBridge'

describe('logFrontendError (browser/non-Tauri mode)', () => {
  afterEach(() => {
    vi.restoreAllMocks()
  })

  it('POSTs the error report to /api/log', async () => {
    const fetchMock = vi.fn().mockResolvedValueOnce(new Response(null, { status: 204 }))
    vi.stubGlobal('fetch', fetchMock)

    await logFrontendError('vue-error-handler', 'boom', 'Error: boom\n  at foo')

    expect(fetchMock).toHaveBeenCalledWith(
      '/api/log',
      expect.objectContaining({
        method: 'POST',
        body: JSON.stringify({
          source: 'vue-error-handler',
          message: 'boom',
          stack: 'Error: boom\n  at foo',
        }),
      }),
    )
  })

  it('never throws when the backend is unreachable', async () => {
    vi.stubGlobal(
      'fetch',
      vi.fn().mockRejectedValueOnce(new Error('network down')),
    )

    await expect(logFrontendError('window-onerror', 'unreachable')).resolves.toBeUndefined()
  })
})

describe('logFrontendError (Tauri mode)', () => {
  afterEach(() => {
    vi.restoreAllMocks()
    vi.doUnmock('@tauri-apps/api/core')
    vi.unstubAllGlobals()
    delete (window as unknown as { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__
  })

  it('invokes the log_frontend_error Tauri command instead of fetching', async () => {
    ;(window as unknown as { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__ = {}
    const invokeMock = vi.fn().mockResolvedValueOnce(undefined)
    vi.doMock('@tauri-apps/api/core', () => ({ invoke: invokeMock }))
    vi.resetModules()

    const { logFrontendError: logFrontendErrorTauri } = await import('@/services/tauriBridge')
    await logFrontendErrorTauri('vue-error-handler', 'boom', 'stack trace')

    expect(invokeMock).toHaveBeenCalledWith('log_frontend_error', {
      source: 'vue-error-handler',
      message: 'boom',
      stack: 'stack trace',
    })
  })
})

/**
 * Thin wrapper around the backend REST command API.
 * All commands POST to /api/command/* and return { ok: bool }.
 */

export function useApi() {
  async function post(path, body = {}) {
    const res = await fetch(path, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(body),
    })
    if (!res.ok) {
      const err = await res.json().catch(() => ({}))
      throw new Error(err.detail ?? `Request failed: ${res.status}`)
    }
    return res.json()
  }

  function selectDestination(name) {
    return post('/api/command/select_destination', { name })
  }

  function surveyDecision(decision) {
    return post('/api/command/survey_decision', { decision })
  }

  function deliberationDecision(chosen) {
    return post('/api/command/deliberation_decision', { chosen })
  }

  return { selectDestination, surveyDecision, deliberationDecision }
}

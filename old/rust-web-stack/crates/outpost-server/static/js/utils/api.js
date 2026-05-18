/**
 * Outpost 3 API Client
 *
 * Handles all communication with the Rust backend.
 * Logs all requests and responses for debugging.
 */

import logger from './logger.js';

class ApiClient {
  constructor(baseUrl = '') {
    this.baseUrl = baseUrl;
    this.requestCount = 0;

    logger.info('ApiClient', 'Initialized', { baseUrl });
  }

  /**
   * Generic fetch wrapper with logging
   */
  async request(endpoint, options = {}) {
    const requestId = ++this.requestCount;
    const method = options.method || 'GET';
    const url = `${this.baseUrl}${endpoint}`;

    logger.logApiCall(endpoint, method, {
      requestId,
      body: options.body,
      headers: options.headers,
    });

    const startTime = performance.now();

    try {
      const response = await fetch(url, {
        ...options,
        headers: {
          'Content-Type': 'application/json',
          ...options.headers,
        },
      });

      const duration = performance.now() - startTime;
      const contentType = response.headers.get('content-type');
      let data;

      if (contentType && contentType.includes('application/json')) {
        data = await response.json();
      } else {
        data = await response.text();
      }

      logger.logApiResponse(endpoint, method, response.status, {
        requestId,
        duration: `${duration.toFixed(2)}ms`,
        contentType,
        dataSize: JSON.stringify(data).length,
      });

      if (!response.ok) {
        throw new Error(`HTTP ${response.status}: ${data.error || data}`);
      }

      return data;
    } catch (error) {
      const duration = performance.now() - startTime;

      logger.error('API', `${method} ${endpoint} FAILED`, {
        requestId,
        duration: `${duration.toFixed(2)}ms`,
        error: error.message,
      });

      throw error;
    }
  }

  // Convenience methods

  async get(endpoint) {
    return this.request(endpoint, { method: 'GET' });
  }

  async post(endpoint, data) {
    return this.request(endpoint, {
      method: 'POST',
      body: JSON.stringify(data),
    });
  }

  async put(endpoint, data) {
    return this.request(endpoint, {
      method: 'PUT',
      body: JSON.stringify(data),
    });
  }

  async delete(endpoint) {
    return this.request(endpoint, { method: 'DELETE' });
  }

  // Game-specific API methods

  /**
   * Get colony data
   */
  async getColony(colonyId) {
    logger.startPerformance(`getColony:${colonyId}`);
    const data = await this.get(`/colony/${colonyId}`);
    logger.endPerformance(`getColony:${colonyId}`);
    return data;
  }

  /**
   * Get colony history for charts
   */
  async getColonyHistory(colonyId, turns = 50) {
    logger.startPerformance(`getColonyHistory:${colonyId}`);
    const data = await this.get(`/api/colony/${colonyId}/history?turns=${turns}`);
    logger.endPerformance(`getColonyHistory:${colonyId}`);
    return data;
  }

  /**
   * Get colony map data
   */
  async getColonyMap(colonyId) {
    logger.startPerformance(`getColonyMap:${colonyId}`);
    const data = await this.get(`/api/colony/${colonyId}/map`);
    logger.endPerformance(`getColonyMap:${colonyId}`);
    return data;
  }

  /**
   * Get building details
   */
  async getBuilding(colonyId, buildingId) {
    return this.get(`/api/colony/${colonyId}/building/${buildingId}`);
  }

  /**
   * Construct a building
   */
  async constructBuilding(colonyId, buildingType, location = null) {
    return this.post(`/colony/${colonyId}/building`, {
      building_type: buildingType,
      location,
    });
  }

  /**
   * Place a building at hex coordinates
   */
  async placeBuilding(colonyId, buildingType, hexCoords) {
    return this.post(`/api/colony/${colonyId}/building/place`, {
      building_type: buildingType,
      hex_q: hexCoords.q,
      hex_r: hexCoords.r,
    });
  }

  /**
   * Advance turn
   */
  async advanceTurn(colonyId) {
    logger.info('API', 'Advancing turn', { colonyId });
    return this.post(`/colony/${colonyId}/turn`, {});
  }

  /**
   * Get resource stockpile
   */
  async getResources(colonyId) {
    return this.get(`/colony/${colonyId}/resources`);
  }

  /**
   * Get buildings list
   */
  async getBuildings(colonyId) {
    return this.get(`/colony/${colonyId}/buildings`);
  }

  /**
   * Get data layer information
   */
  async getDataLayer(colonyId, layerType) {
    return this.get(`/api/colony/${colonyId}/layers/${layerType}`);
  }
}

// Create singleton
const api = new ApiClient();

// Expose to window for debugging
if (typeof window !== 'undefined') {
  window.outpostApi = api;
  console.log('[ApiClient] Exposed as window.outpostApi');
}

export default api;

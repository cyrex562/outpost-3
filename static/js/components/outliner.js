/**
 * Outpost 3 - Outliner Component
 *
 * Displays quick lists of buildings, alerts, and other important items.
 * Updates dynamically and logs all changes for debugging.
 */

import logger from '../utils/logger.js';
import stateManager from '../utils/state.js';
import api from '../utils/api.js';

class Outliner {
  constructor() {
    this.buildingListElement = null;
    this.alertsListElement = null;
    this.colonyId = null;
    this.updateInterval = null;

    // Create reactive state
    this.state = stateManager.createStore('outliner', {
      buildings: [],
      alerts: [],
      lastUpdate: null,
      updating: false,
      autoRefresh: true,
      refreshInterval: 30000, // 30 seconds
    });

    logger.info('Outliner', 'Constructor called');
  }

  /**
   * Initialize the outliner component
   */
  init(colonyId) {
    logger.startPerformance('outliner-init');
    logger.info('Outliner', 'Initializing outliner', { colonyId });

    try {
      this.colonyId = colonyId;

      // Get DOM elements
      this.buildingListElement = document.getElementById('building-quick-list');
      this.alertsListElement = document.getElementById('alerts-list');

      if (!this.buildingListElement || !this.alertsListElement) {
        logger.error('Outliner', 'Required elements not found', {
          buildingList: !!this.buildingListElement,
          alertsList: !!this.alertsListElement,
        });
        return;
      }

      logger.debug('Outliner', 'DOM elements found');

      // Initial data fetch
      this.update();

      // Set up auto-refresh if enabled
      if (this.state.autoRefresh) {
        this.startAutoRefresh();
      }

      // Listen for events
      this.setupEventListeners();

      logger.endPerformance('outliner-init');
      logger.info('Outliner', 'Initialization complete');
    } catch (error) {
      logger.error('Outliner', 'Initialization failed', {
        error: error.message,
        stack: error.stack,
      });
    }
  }

  /**
   * Set up event listeners
   */
  setupEventListeners() {
    logger.debug('Outliner', 'Setting up event listeners');

    // Listen for building changes
    document.addEventListener('building:added', () => {
      logger.debug('Outliner', 'Event received: building:added');
      this.update();
    });

    document.addEventListener('building:updated', () => {
      logger.debug('Outliner', 'Event received: building:updated');
      this.update();
    });

    document.addEventListener('building:removed', () => {
      logger.debug('Outliner', 'Event received: building:removed');
      this.update();
    });

    // Listen for turn advancement
    document.addEventListener('turn:advanced', () => {
      logger.debug('Outliner', 'Event received: turn:advanced');
      this.update();
    });

    // Listen for manual refresh requests
    document.addEventListener('outliner:refresh', () => {
      logger.debug('Outliner', 'Event received: outliner:refresh');
      this.update();
    });
  }

  /**
   * Update outliner data from API
   */
  async update() {
    if (this.state.updating) {
      logger.warn('Outliner', 'Update already in progress, skipping');
      return;
    }

    logger.startPerformance('outliner-update');
    logger.info('Outliner', 'Updating outliner data');

    this.state.updating = true;

    try {
      // Fetch buildings data
      const buildings = await this.fetchBuildings();

      // Generate alerts from buildings data
      const alerts = this.generateAlerts(buildings);

      // Update state
      this.state.buildings = buildings;
      this.state.alerts = alerts;
      this.state.lastUpdate = new Date().toISOString();

      logger.info('Outliner', 'Data updated', {
        buildingCount: buildings.length,
        alertCount: alerts.length,
        timestamp: this.state.lastUpdate,
      });

      // Render
      this.render();

      logger.endPerformance('outliner-update');
    } catch (error) {
      logger.error('Outliner', 'Update failed', {
        error: error.message,
      });
    } finally {
      this.state.updating = false;
    }
  }

  /**
   * Fetch buildings from API
   */
  async fetchBuildings() {
    logger.debug('Outliner', 'Fetching buildings from API', {
      colonyId: this.colonyId,
    });

    try {
      // For now, return mock data until API endpoint exists
      // TODO: Replace with actual API call when backend is ready
      const buildings = this.getMockBuildings();

      logger.debug('Outliner', 'Buildings fetched', {
        count: buildings.length,
      });

      return buildings;
    } catch (error) {
      logger.error('Outliner', 'Failed to fetch buildings', {
        error: error.message,
      });
      return [];
    }
  }

  /**
   * Get mock buildings data (temporary)
   */
  getMockBuildings() {
    return [
      {
        id: 1,
        name: 'Mine Alpha',
        type: 'Mine',
        status: 'operational',
        output: 100,
        workers: 5,
      },
      {
        id: 2,
        name: 'Power Plant',
        type: 'PowerPlant',
        status: 'operational',
        output: 500,
        workers: 3,
      },
      {
        id: 3,
        name: 'Housing Complex',
        type: 'Housing',
        status: 'operational',
        capacity: 100,
        workers: 2,
      },
      {
        id: 4,
        name: 'Factory Beta',
        type: 'Factory',
        status: 'under_construction',
        progress: 65,
        workers: 10,
      },
    ];
  }

  /**
   * Generate alerts from buildings data
   */
  generateAlerts(buildings) {
    logger.debug('Outliner', 'Generating alerts from buildings');

    const alerts = [];

    // Check for buildings under construction
    const underConstruction = buildings.filter(
      (b) => b.status === 'under_construction'
    );
    if (underConstruction.length > 0) {
      alerts.push({
        type: 'info',
        message: `${underConstruction.length} building(s) under construction`,
        buildings: underConstruction.map((b) => b.name),
      });
    }

    // Check for damaged buildings
    const damaged = buildings.filter((b) => b.status === 'damaged');
    if (damaged.length > 0) {
      alerts.push({
        type: 'error',
        message: `${damaged.length} building(s) damaged`,
        buildings: damaged.map((b) => b.name),
      });
    }

    // Check for shutdown buildings
    const shutdown = buildings.filter((b) => b.status === 'shutdown');
    if (shutdown.length > 0) {
      alerts.push({
        type: 'warning',
        message: `${shutdown.length} building(s) offline`,
        buildings: shutdown.map((b) => b.name),
      });
    }

    logger.debug('Outliner', 'Alerts generated', {
      count: alerts.length,
      types: alerts.map((a) => a.type),
    });

    return alerts;
  }

  /**
   * Render the outliner UI
   */
  render() {
    logger.startPerformance('outliner-render');
    logger.info('Outliner', 'Rendering outliner UI');

    try {
      this.renderBuildings();
      this.renderAlerts();

      logger.logRender('Outliner', {
        buildingCount: this.state.buildings.length,
        alertCount: this.state.alerts.length,
        timestamp: this.state.lastUpdate,
      });

      logger.endPerformance('outliner-render');
    } catch (error) {
      logger.error('Outliner', 'Render failed', {
        error: error.message,
        stack: error.stack,
      });
    }
  }

  /**
   * Render buildings list
   */
  renderBuildings() {
    logger.debug('Outliner', 'Rendering buildings list', {
      count: this.state.buildings.length,
    });

    if (!this.buildingListElement) return;

    // Clear existing content
    this.buildingListElement.innerHTML = '';

    if (this.state.buildings.length === 0) {
      this.buildingListElement.innerHTML = `
        <div class="outliner-empty">
          <p>No buildings</p>
        </div>
      `;
      return;
    }

    // Create building items
    this.state.buildings.forEach((building) => {
      const item = this.createBuildingItem(building);
      this.buildingListElement.appendChild(item);
    });

    logger.debug('Outliner', 'Buildings rendered', {
      count: this.state.buildings.length,
    });
  }

  /**
   * Create a building item element
   */
  createBuildingItem(building) {
    const item = document.createElement('div');
    item.className = 'outliner-item';
    item.dataset.buildingId = building.id;

    // Get icon based on type
    const icon = this.getBuildingIcon(building.type);

    // Get status badge
    const badge = this.getStatusBadge(building.status);

    item.innerHTML = `
      <span class="outliner-item-icon">${icon}</span>
      <span class="outliner-item-text">${building.name}</span>
      ${badge}
    `;

    // Add click handler
    item.addEventListener('click', () => {
      logger.logInteraction('Outliner', 'Click', 'building-item', {
        buildingId: building.id,
        buildingName: building.name,
        buildingType: building.type,
      });

      this.onBuildingClick(building);
    });

    return item;
  }

  /**
   * Get icon for building type
   */
  getBuildingIcon(type) {
    const icons = {
      Mine: '⛏️',
      PowerPlant: '⚡',
      Housing: '🏠',
      Factory: '🏭',
      Farm: '🌾',
      Refinery: '🏗️',
      Warehouse: '📦',
      TrainStation: '🚉',
    };

    return icons[type] || '🏢';
  }

  /**
   * Get status badge HTML
   */
  getStatusBadge(status) {
    if (status === 'under_construction') {
      return '<span class="outliner-item-badge">🔨</span>';
    } else if (status === 'damaged') {
      return '<span class="outliner-item-badge">⚠️</span>';
    } else if (status === 'shutdown') {
      return '<span class="outliner-item-badge">🔴</span>';
    }
    return '';
  }

  /**
   * Render alerts list
   */
  renderAlerts() {
    logger.debug('Outliner', 'Rendering alerts list', {
      count: this.state.alerts.length,
    });

    if (!this.alertsListElement) return;

    // Clear existing content
    this.alertsListElement.innerHTML = '';

    if (this.state.alerts.length === 0) {
      this.alertsListElement.innerHTML = `
        <div class="outliner-empty">
          <p>No alerts</p>
        </div>
      `;
      return;
    }

    // Create alert items
    this.state.alerts.forEach((alert) => {
      const item = this.createAlertItem(alert);
      this.alertsListElement.appendChild(item);
    });

    logger.debug('Outliner', 'Alerts rendered', {
      count: this.state.alerts.length,
    });
  }

  /**
   * Create an alert item element
   */
  createAlertItem(alert) {
    const item = document.createElement('div');
    item.className = `alert-item alert-${alert.type}`;

    item.innerHTML = `
      <p>${alert.message}</p>
    `;

    // Add click handler if there are associated buildings
    if (alert.buildings && alert.buildings.length > 0) {
      item.style.cursor = 'pointer';
      item.addEventListener('click', () => {
        logger.logInteraction('Outliner', 'Click', 'alert-item', {
          type: alert.type,
          message: alert.message,
          buildingCount: alert.buildings.length,
        });

        this.onAlertClick(alert);
      });
    }

    return item;
  }

  /**
   * Handle building item click
   */
  onBuildingClick(building) {
    logger.info('Outliner', 'Building clicked', {
      buildingId: building.id,
      buildingName: building.name,
    });

    // Dispatch event for other components to handle
    document.dispatchEvent(new CustomEvent('outliner:building-selected', {
      detail: { building },
    }));

    // Open building detail modal
    document.dispatchEvent(new CustomEvent('modal:open', {
      detail: {
        modal: 'building-detail',
        data: { building },
        source: 'outliner',
      },
    }));
  }

  /**
   * Handle alert click
   */
  onAlertClick(alert) {
    logger.info('Outliner', 'Alert clicked', {
      type: alert.type,
      message: alert.message,
    });

    // Dispatch event
    document.dispatchEvent(new CustomEvent('outliner:alert-selected', {
      detail: { alert },
    }));
  }

  /**
   * Start auto-refresh timer
   */
  startAutoRefresh() {
    if (this.updateInterval) {
      logger.warn('Outliner', 'Auto-refresh already running');
      return;
    }

    logger.info('Outliner', 'Starting auto-refresh', {
      interval: this.state.refreshInterval,
    });

    this.updateInterval = setInterval(() => {
      logger.debug('Outliner', 'Auto-refresh triggered');
      this.update();
    }, this.state.refreshInterval);
  }

  /**
   * Stop auto-refresh timer
   */
  stopAutoRefresh() {
    if (!this.updateInterval) return;

    logger.info('Outliner', 'Stopping auto-refresh');

    clearInterval(this.updateInterval);
    this.updateInterval = null;
  }

  /**
   * Destroy the component
   */
  destroy() {
    logger.info('Outliner', 'Destroying outliner component');

    this.stopAutoRefresh();
  }
}

// Create singleton instance
const outliner = new Outliner();

// Expose to window for debugging
if (typeof window !== 'undefined') {
  window.outpostOutliner = outliner;
  logger.debug('Outliner', 'Exposed as window.outpostOutliner');
}

export default outliner;

/**
 * Outpost 3 - Construction Modal
 *
 * Allows selection and construction of new buildings with logging.
 */

import logger from '../../utils/logger.js';
import { BaseModal } from '../modal.js';

class ConstructionModal extends BaseModal {
  constructor(data = {}) {
    super(data);

    this.colonyId = data.colonyId;
    this.selectedBuilding = null;

    this.title = 'Construction Menu';
    this.icon = '🔨';
    this.size = 'modal-xl';

    logger.info('ConstructionModal', 'Created', {
      colonyId: this.colonyId,
    });

    this.footer = [
      {
        label: 'Build',
        class: 'modal-btn-primary',
        onClick: () => this.onBuild(),
        close: false,
      },
      {
        label: 'Cancel',
        class: '',
        close: true,
      },
    ];

    this.buildingTypes = this.getBuildingTypes();
  }

  /**
   * Get available building types
   */
  getBuildingTypes() {
    return [
      {
        id: 'mine',
        name: 'Mine',
        icon: '⛏️',
        cost: { credits: 500, iron: 100 },
        description: 'Extracts raw materials from deposits',
        category: 'production',
      },
      {
        id: 'power_plant',
        name: 'Power Plant',
        icon: '⚡',
        cost: { credits: 1000, iron: 200 },
        description: 'Generates electricity for the colony',
        category: 'infrastructure',
      },
      {
        id: 'solar_plant',
        name: 'Solar Plant',
        icon: '☀️',
        cost: { credits: 800, iron: 150 },
        description: 'Clean energy from solar panels',
        category: 'infrastructure',
      },
      {
        id: 'housing',
        name: 'Housing',
        icon: '🏠',
        cost: { credits: 600, iron: 120 },
        description: 'Provides living space for colonists',
        category: 'residential',
      },
      {
        id: 'factory',
        name: 'Factory',
        icon: '🏭',
        cost: { credits: 1500, iron: 300 },
        description: 'Processes raw materials into goods',
        category: 'production',
      },
      {
        id: 'farm',
        name: 'Farm',
        icon: '🌾',
        cost: { credits: 700, iron: 100 },
        description: 'Produces food for the colony',
        category: 'food',
      },
      {
        id: 'refinery',
        name: 'Refinery',
        icon: '🏗️',
        cost: { credits: 1200, iron: 250 },
        description: 'Refines raw materials',
        category: 'production',
      },
      {
        id: 'warehouse',
        name: 'Warehouse',
        icon: '📦',
        cost: { credits: 400, iron: 80 },
        description: 'Increases storage capacity',
        category: 'infrastructure',
      },
      {
        id: 'train_station',
        name: 'Train Station',
        icon: '🚉',
        cost: { credits: 2000, iron: 400 },
        description: 'Enables train transport',
        category: 'infrastructure',
      },
      {
        id: 'research',
        name: 'Research Lab',
        icon: '🔬',
        cost: { credits: 1800, iron: 200 },
        description: 'Unlocks new technologies',
        category: 'research',
      },
      {
        id: 'medical',
        name: 'Medical Center',
        icon: '🏥',
        cost: { credits: 1000, iron: 150 },
        description: 'Improves colonist health',
        category: 'residential',
      },
      {
        id: 'commercial',
        name: 'Commercial Zone',
        icon: '🏪',
        cost: { credits: 900, iron: 180 },
        description: 'Generates income from trade',
        category: 'commercial',
      },
    ];
  }

  /**
   * Render modal content
   */
  render() {
    logger.startPerformance('construction-modal-render');
    logger.logRender('ConstructionModal', {
      buildingTypeCount: this.buildingTypes.length,
    });

    const html = `
      <!-- Info Alert -->
      <div class="modal-alert alert-info">
        <span class="modal-alert-icon">ℹ️</span>
        <div class="modal-alert-content">
          <strong>Select a building to construct</strong>
          <p>Click on a building type to see details and costs</p>
        </div>
      </div>

      <!-- Building Grid -->
      <div class="modal-section">
        <h3 class="modal-section-title">Available Buildings</h3>
        <div class="modal-grid" id="building-grid">
          ${this.renderBuildingGrid()}
        </div>
      </div>

      <div class="modal-divider"></div>

      <!-- Selected Building Details -->
      <div class="modal-section">
        <h3 class="modal-section-title">Building Details</h3>
        <div id="building-details">
          ${this.renderBuildingDetails()}
        </div>
      </div>
    `;

    logger.endPerformance('construction-modal-render');

    // Set up event listeners after render
    setTimeout(() => this.setupGridListeners(), 0);

    return html;
  }

  /**
   * Render building grid
   */
  renderBuildingGrid() {
    return this.buildingTypes
      .map(
        (building) => `
      <div
        class="modal-grid-item"
        data-building-id="${building.id}"
        role="button"
        tabindex="0"
      >
        <span class="modal-grid-item-icon">${building.icon}</span>
        <span class="modal-grid-item-label">${building.name}</span>
        <span class="modal-grid-item-cost">
          ${this.formatCost(building.cost)}
        </span>
      </div>
    `
      )
      .join('');
  }

  /**
   * Format cost display
   */
  formatCost(cost) {
    return Object.entries(cost)
      .map(([resource, amount]) => `${amount} ${resource}`)
      .join(', ');
  }

  /**
   * Render building details
   */
  renderBuildingDetails() {
    if (!this.selectedBuilding) {
      return `
        <div class="modal-text-center modal-text-muted">
          <p>No building selected</p>
        </div>
      `;
    }

    const building = this.selectedBuilding;

    return `
      <div class="modal-stats">
        <div class="modal-stat">
          <div class="modal-stat-label">Name</div>
          <div class="modal-stat-value">${building.name} ${building.icon}</div>
        </div>
        <div class="modal-stat">
          <div class="modal-stat-label">Category</div>
          <div class="modal-stat-value">${building.category}</div>
        </div>
      </div>

      <div class="modal-spacer"></div>

      <p class="modal-text-muted">${building.description}</p>

      <div class="modal-spacer"></div>

      <div class="modal-section">
        <h4 class="modal-section-title">Construction Cost</h4>
        <div class="modal-text-muted">
          ${Object.entries(building.cost)
            .map(
              ([resource, amount]) =>
                `<p><strong>${resource}:</strong> ${amount}</p>`
            )
            .join('')}
        </div>
      </div>

      <div class="modal-alert alert-warning">
        <span class="modal-alert-icon">⚠️</span>
        <div class="modal-alert-content">
          <strong>Placement Required</strong>
          <p>After clicking Build, select a location on the map</p>
        </div>
      </div>
    `;
  }

  /**
   * Set up grid item click listeners
   */
  setupGridListeners() {
    const gridItems = document.querySelectorAll('.modal-grid-item');

    gridItems.forEach((item) => {
      item.addEventListener('click', () => {
        const buildingId = item.dataset.buildingId;

        logger.logInteraction('ConstructionModal', 'Click', 'building-grid-item', {
          buildingId,
        });

        this.selectBuilding(buildingId);
      });
    });

    logger.debug('ConstructionModal', 'Grid listeners set up', {
      itemCount: gridItems.length,
    });
  }

  /**
   * Select a building
   */
  selectBuilding(buildingId) {
    logger.info('ConstructionModal', 'Building selected', { buildingId });

    // Find building
    this.selectedBuilding = this.buildingTypes.find((b) => b.id === buildingId);

    if (!this.selectedBuilding) {
      logger.error('ConstructionModal', 'Building not found', { buildingId });
      return;
    }

    // Update UI
    const gridItems = document.querySelectorAll('.modal-grid-item');
    gridItems.forEach((item) => {
      item.classList.toggle('selected', item.dataset.buildingId === buildingId);
    });

    // Update details panel
    const detailsPanel = document.getElementById('building-details');
    if (detailsPanel) {
      detailsPanel.innerHTML = this.renderBuildingDetails();
      logger.logRender('ConstructionModal', {
        panel: 'details',
        buildingId,
      });
    }
  }

  /**
   * Handle build action
   */
  onBuild() {
    if (!this.selectedBuilding) {
      logger.warn('ConstructionModal', 'Build attempted with no selection');
      alert('Please select a building type first');
      return;
    }

    logger.logInteraction('ConstructionModal', 'Click', 'build', {
      buildingId: this.selectedBuilding.id,
      buildingName: this.selectedBuilding.name,
    });

    // Dispatch event for map to handle placement
    document.dispatchEvent(
      new CustomEvent('construction:start', {
        detail: {
          building: this.selectedBuilding,
          colonyId: this.colonyId,
        },
      })
    );

    logger.info('ConstructionModal', 'Construction started', {
      buildingId: this.selectedBuilding.id,
    });

    alert(
      `${this.selectedBuilding.name} selected for construction!\n\n` +
        `In the future, you'll click on the map to place it.\n` +
        `For now, this feature is coming soon.`
    );
  }

  /**
   * Called when modal closes
   */
  onClose() {
    logger.info('ConstructionModal', 'Modal closed', {
      selectedBuilding: this.selectedBuilding?.id || null,
    });
  }
}

export default ConstructionModal;

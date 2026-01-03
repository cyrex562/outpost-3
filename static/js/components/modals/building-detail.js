/**
 * Outpost 3 - Building Detail Modal
 *
 * Displays detailed information about a building with comprehensive logging.
 */

import logger from '../../utils/logger.js';
import { BaseModal } from '../modal.js';

class BuildingDetailModal extends BaseModal {
  constructor(data = {}) {
    super(data);

    this.building = data.building || {};
    this.title = this.building.name || 'Building Details';
    this.icon = this.getBuildingIcon(this.building.type);
    this.size = 'modal-lg';

    logger.info('BuildingDetailModal', 'Created', {
      buildingId: this.building.id,
      buildingName: this.building.name,
      buildingType: this.building.type,
    });

    this.footer = [
      {
        label: 'Upgrade',
        class: 'modal-btn-primary',
        onClick: () => this.onUpgrade(),
        close: false,
      },
      {
        label: 'Repair',
        class: 'modal-btn-success',
        onClick: () => this.onRepair(),
        close: false,
      },
      {
        label: 'Shutdown',
        class: 'modal-btn-danger',
        onClick: () => this.onShutdown(),
        close: false,
      },
      {
        label: 'Close',
        class: '',
        close: true,
      },
    ];
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
   * Render modal content
   */
  render() {
    logger.startPerformance('building-detail-render');
    logger.logRender('BuildingDetailModal', {
      buildingId: this.building.id,
    });

    const html = `
      <!-- Building Status Alert -->
      ${this.renderStatusAlert()}

      <!-- Main Stats -->
      <div class="modal-section">
        <h3 class="modal-section-title">Statistics</h3>
        <div class="modal-stats">
          ${this.renderStats()}
        </div>
      </div>

      <div class="modal-divider"></div>

      <!-- Production Info -->
      ${this.renderProduction()}

      <div class="modal-divider"></div>

      <!-- Workers -->
      ${this.renderWorkers()}

      <div class="modal-divider"></div>

      <!-- Actions History -->
      ${this.renderHistory()}
    `;

    logger.endPerformance('building-detail-render');

    return html;
  }

  /**
   * Render status alert if needed
   */
  renderStatusAlert() {
    const { status } = this.building;

    if (status === 'under_construction') {
      return `
        <div class="modal-alert alert-info">
          <span class="modal-alert-icon">🔨</span>
          <div class="modal-alert-content">
            <strong>Under Construction</strong>
            <p>Progress: ${this.building.progress || 0}%</p>
          </div>
        </div>
      `;
    } else if (status === 'damaged') {
      return `
        <div class="modal-alert alert-error">
          <span class="modal-alert-icon">⚠️</span>
          <div class="modal-alert-content">
            <strong>Damaged</strong>
            <p>This building requires repairs</p>
          </div>
        </div>
      `;
    } else if (status === 'shutdown') {
      return `
        <div class="modal-alert alert-warning">
          <span class="modal-alert-icon">🔴</span>
          <div class="modal-alert-content">
            <strong>Offline</strong>
            <p>This building is currently shut down</p>
          </div>
        </div>
      `;
    }

    return '';
  }

  /**
   * Render statistics
   */
  renderStats() {
    const { building } = this;

    return `
      <div class="modal-stat">
        <div class="modal-stat-label">Type</div>
        <div class="modal-stat-value">${building.type}</div>
      </div>
      <div class="modal-stat">
        <div class="modal-stat-label">Status</div>
        <div class="modal-stat-value">${this.formatStatus(building.status)}</div>
      </div>
      <div class="modal-stat">
        <div class="modal-stat-label">Workers</div>
        <div class="modal-stat-value">${building.workers || 0}</div>
      </div>
      <div class="modal-stat">
        <div class="modal-stat-label">Output</div>
        <div class="modal-stat-value positive">${building.output || 0}/turn</div>
      </div>
    `;
  }

  /**
   * Format status text
   */
  formatStatus(status) {
    const statusMap = {
      operational: 'Operational',
      under_construction: 'Under Construction',
      damaged: 'Damaged',
      shutdown: 'Shutdown',
    };
    return statusMap[status] || status;
  }

  /**
   * Render production information
   */
  renderProduction() {
    const { building } = this;

    if (building.type === 'Factory') {
      return `
        <div class="modal-section">
          <h3 class="modal-section-title">Production Chain</h3>
          <div class="modal-text-muted">
            <p><strong>Consumes:</strong> Iron Ore, Energy</p>
            <p><strong>Produces:</strong> Steel (${building.output || 0}/turn)</p>
            <p><strong>Efficiency:</strong> 85%</p>
          </div>
        </div>
      `;
    } else if (building.type === 'Mine') {
      return `
        <div class="modal-section">
          <h3 class="modal-section-title">Extraction</h3>
          <div class="modal-text-muted">
            <p><strong>Resource:</strong> Iron Ore</p>
            <p><strong>Rate:</strong> ${building.output || 0}/turn</p>
            <p><strong>Vein Quality:</strong> High</p>
          </div>
        </div>
      `;
    } else if (building.type === 'PowerPlant') {
      return `
        <div class="modal-section">
          <h3 class="modal-section-title">Power Generation</h3>
          <div class="modal-text-muted">
            <p><strong>Output:</strong> ${building.output || 0} MW</p>
            <p><strong>Fuel:</strong> Coal</p>
            <p><strong>Efficiency:</strong> 90%</p>
          </div>
        </div>
      `;
    }

    return '';
  }

  /**
   * Render worker allocation
   */
  renderWorkers() {
    const { building } = this;

    return `
      <div class="modal-section">
        <h3 class="modal-section-title">Labor</h3>
        <div class="modal-form-group">
          <label class="modal-form-label">
            Workers Assigned: ${building.workers || 0}
          </label>
          <input
            type="range"
            min="0"
            max="20"
            value="${building.workers || 0}"
            class="modal-form-input"
            data-action="adjust-workers"
          />
          <div class="modal-form-help">
            Drag to adjust worker allocation
          </div>
        </div>
      </div>
    `;
  }

  /**
   * Render history
   */
  renderHistory() {
    return `
      <div class="modal-section">
        <h3 class="modal-section-title">Recent Activity</h3>
        <div class="modal-text-muted">
          <ul style="margin: 0; padding-left: 1.5rem;">
            <li>Turn 95: Production started</li>
            <li>Turn 92: Construction completed</li>
            <li>Turn 90: Construction started</li>
          </ul>
        </div>
      </div>
    `;
  }

  /**
   * Handle upgrade action
   */
  onUpgrade() {
    logger.logInteraction('BuildingDetailModal', 'Click', 'upgrade', {
      buildingId: this.building.id,
      buildingName: this.building.name,
    });

    // Show confirmation
    alert(`Upgrade ${this.building.name}?\n\nThis feature is coming soon!`);
  }

  /**
   * Handle repair action
   */
  onRepair() {
    logger.logInteraction('BuildingDetailModal', 'Click', 'repair', {
      buildingId: this.building.id,
      buildingName: this.building.name,
    });

    if (this.building.status !== 'damaged') {
      logger.warn('BuildingDetailModal', 'Repair attempted on non-damaged building');
      alert('This building does not need repairs.');
      return;
    }

    alert(`Repair ${this.building.name}?\n\nThis feature is coming soon!`);
  }

  /**
   * Handle shutdown action
   */
  onShutdown() {
    logger.logInteraction('BuildingDetailModal', 'Click', 'shutdown', {
      buildingId: this.building.id,
      buildingName: this.building.name,
    });

    const action = this.building.status === 'shutdown' ? 'Restart' : 'Shutdown';
    alert(`${action} ${this.building.name}?\n\nThis feature is coming soon!`);
  }

  /**
   * Called when modal closes
   */
  onClose() {
    logger.info('BuildingDetailModal', 'Modal closed', {
      buildingId: this.building.id,
    });
  }
}

export default BuildingDetailModal;

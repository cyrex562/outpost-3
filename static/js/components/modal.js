/**
 * Outpost 3 - Modal Component
 *
 * Reusable modal framework with event logging.
 * Supports multiple modals, animations, and keyboard navigation.
 */

import logger from '../utils/logger.js';
import stateManager from '../utils/state.js';

class ModalManager {
  constructor() {
    this.container = null;
    this.activeModals = [];
    this.modalRegistry = new Map();

    // Create reactive state
    this.state = stateManager.createStore('modal', {
      activeCount: 0,
      currentModal: null,
      initialized: false,
    });

    logger.info('Modal', 'ModalManager created');
  }

  /**
   * Initialize the modal system
   */
  init() {
    logger.startPerformance('modal-init');
    logger.info('Modal', 'Initializing modal system');

    try {
      this.container = document.getElementById('modal-container');

      if (!this.container) {
        logger.error('Modal', 'Modal container not found');
        return;
      }

      logger.debug('Modal', 'Modal container found');

      // Set up event listeners
      this.setupEventListeners();

      this.state.initialized = true;

      logger.endPerformance('modal-init');
      logger.info('Modal', 'Modal system initialized');
    } catch (error) {
      logger.error('Modal', 'Initialization failed', {
        error: error.message,
        stack: error.stack,
      });
    }
  }

  /**
   * Set up global event listeners
   */
  setupEventListeners() {
    logger.debug('Modal', 'Setting up event listeners');

    // Listen for modal open requests
    document.addEventListener('modal:open', (event) => {
      const { modal, data, source } = event.detail || {};

      logger.info('Modal', 'Open request received', {
        modal,
        source,
        hasData: !!data,
      });

      this.open(modal, data);
    });

    // Listen for modal close requests
    document.addEventListener('modal:close', (event) => {
      const { modalId } = event.detail || {};

      logger.info('Modal', 'Close request received', {
        modalId: modalId || 'current',
      });

      if (modalId) {
        this.closeById(modalId);
      } else {
        this.closeCurrent();
      }
    });

    // Global escape key handler
    document.addEventListener('keydown', (event) => {
      if (event.key === 'Escape' && this.activeModals.length > 0) {
        logger.logInteraction('Modal', 'Keypress', 'Escape');
        this.closeCurrent();
      }
    });

    logger.debug('Modal', 'Event listeners set up');
  }

  /**
   * Register a modal type
   */
  register(modalId, modalClass) {
    logger.info('Modal', 'Registering modal type', { modalId });

    this.modalRegistry.set(modalId, modalClass);

    logger.debug('Modal', 'Modal registered', {
      modalId,
      totalRegistered: this.modalRegistry.size,
    });
  }

  /**
   * Open a modal
   */
  open(modalId, data = {}) {
    logger.startPerformance(`modal-open-${modalId}`);
    logger.info('Modal', 'Opening modal', { modalId, data });

    try {
      // Get modal class from registry
      const ModalClass = this.modalRegistry.get(modalId);

      if (!ModalClass) {
        logger.error('Modal', 'Modal not registered', { modalId });
        return null;
      }

      // Create modal instance
      const modal = new ModalClass(data);
      const modalElement = this.createModalElement(modalId, modal);

      // Add to active modals
      this.activeModals.push({
        id: modalId,
        instance: modal,
        element: modalElement,
      });

      // Update state
      this.state.activeCount = this.activeModals.length;
      this.state.currentModal = modalId;

      // Append to container
      this.container.appendChild(modalElement);
      this.container.classList.add('has-modal');

      logger.logRender('Modal', {
        modalId,
        activeCount: this.state.activeCount,
      });

      // Trigger animation
      requestAnimationFrame(() => {
        const overlay = modalElement.querySelector('.modal-overlay');
        const modalWindow = modalElement.querySelector('.modal');

        if (overlay) overlay.classList.add('visible');
        if (modalWindow) modalWindow.classList.add('visible');

        logger.debug('Modal', 'Animation triggered', { modalId });
      });

      logger.endPerformance(`modal-open-${modalId}`);
      logger.logModalEvent(modalId, 'OPENED', {
        activeCount: this.state.activeCount,
      });

      return modal;
    } catch (error) {
      logger.error('Modal', 'Failed to open modal', {
        modalId,
        error: error.message,
        stack: error.stack,
      });
      return null;
    }
  }

  /**
   * Create modal DOM element
   */
  createModalElement(modalId, modalInstance) {
    logger.debug('Modal', 'Creating modal element', { modalId });

    const wrapper = document.createElement('div');
    wrapper.className = 'modal-wrapper';
    wrapper.dataset.modalId = modalId;

    // Create overlay
    const overlay = document.createElement('div');
    overlay.className = 'modal-overlay';
    overlay.addEventListener('click', () => {
      logger.logInteraction('Modal', 'Click', 'overlay', { modalId });
      this.closeCurrent();
    });

    // Create modal window
    const modal = document.createElement('div');
    modal.className = `modal ${modalInstance.size || 'modal-md'}`;

    // Create header
    const header = this.createHeader(modalId, modalInstance);
    modal.appendChild(header);

    // Create body
    const body = document.createElement('div');
    body.className = 'modal-body';
    body.innerHTML = modalInstance.render();
    modal.appendChild(body);

    // Create footer if provided
    if (modalInstance.footer) {
      const footer = this.createFooter(modalId, modalInstance);
      modal.appendChild(footer);
    }

    // Assemble
    wrapper.appendChild(overlay);
    wrapper.appendChild(modal);

    // Store reference to instance
    wrapper._modalInstance = modalInstance;

    logger.logRender('Modal', {
      modalId,
      hasHeader: !!header,
      hasFooter: !!modalInstance.footer,
    });

    return wrapper;
  }

  /**
   * Create modal header
   */
  createHeader(modalId, modalInstance) {
    const header = document.createElement('div');
    header.className = 'modal-header';

    const title = document.createElement('h2');
    title.className = 'modal-title';
    if (modalInstance.icon) {
      title.innerHTML = `
        <span class="modal-title-icon">${modalInstance.icon}</span>
        <span>${modalInstance.title || 'Modal'}</span>
      `;
    } else {
      title.textContent = modalInstance.title || 'Modal';
    }

    const closeBtn = document.createElement('button');
    closeBtn.className = 'modal-close';
    closeBtn.innerHTML = '×';
    closeBtn.setAttribute('aria-label', 'Close');
    closeBtn.addEventListener('click', () => {
      logger.logInteraction('Modal', 'Click', 'close-button', { modalId });
      this.closeCurrent();
    });

    header.appendChild(title);
    header.appendChild(closeBtn);

    return header;
  }

  /**
   * Create modal footer
   */
  createFooter(modalId, modalInstance) {
    const footer = document.createElement('div');
    footer.className = 'modal-footer';

    modalInstance.footer.forEach((button) => {
      const btn = document.createElement('button');
      btn.className = `modal-btn ${button.class || ''}`;
      btn.textContent = button.label;

      btn.addEventListener('click', () => {
        logger.logInteraction('Modal', 'Click', 'footer-button', {
          modalId,
          buttonLabel: button.label,
        });

        if (button.onClick) {
          button.onClick(modalInstance);
        }

        if (button.close !== false) {
          this.closeCurrent();
        }
      });

      footer.appendChild(btn);
    });

    return footer;
  }

  /**
   * Close current (topmost) modal
   */
  closeCurrent() {
    if (this.activeModals.length === 0) {
      logger.warn('Modal', 'No active modals to close');
      return;
    }

    const current = this.activeModals[this.activeModals.length - 1];
    this.close(current);
  }

  /**
   * Close modal by ID
   */
  closeById(modalId) {
    const modal = this.activeModals.find((m) => m.id === modalId);

    if (!modal) {
      logger.warn('Modal', 'Modal not found', { modalId });
      return;
    }

    this.close(modal);
  }

  /**
   * Close a modal
   */
  close(modalData) {
    logger.startPerformance(`modal-close-${modalData.id}`);
    logger.info('Modal', 'Closing modal', { modalId: modalData.id });

    try {
      const { id, element, instance } = modalData;

      // Trigger exit animation
      const overlay = element.querySelector('.modal-overlay');
      const modalWindow = element.querySelector('.modal');

      if (overlay) overlay.classList.remove('visible');
      if (modalWindow) modalWindow.classList.remove('visible');

      logger.debug('Modal', 'Exit animation triggered', { modalId: id });

      // Remove after animation
      setTimeout(() => {
        element.remove();

        // Remove from active modals
        this.activeModals = this.activeModals.filter((m) => m.id !== id);

        // Update state
        this.state.activeCount = this.activeModals.length;
        this.state.currentModal =
          this.activeModals.length > 0
            ? this.activeModals[this.activeModals.length - 1].id
            : null;

        if (this.activeModals.length === 0) {
          this.container.classList.remove('has-modal');
        }

        logger.logRender('Modal', {
          modalId: id,
          activeCount: this.state.activeCount,
        });

        // Call instance cleanup if it exists
        if (instance.onClose) {
          instance.onClose();
        }

        logger.endPerformance(`modal-close-${id}`);
        logger.logModalEvent(id, 'CLOSED', {
          activeCount: this.state.activeCount,
        });
      }, 300); // Match CSS transition duration
    } catch (error) {
      logger.error('Modal', 'Failed to close modal', {
        modalId: modalData.id,
        error: error.message,
      });
    }
  }

  /**
   * Close all modals
   */
  closeAll() {
    logger.info('Modal', 'Closing all modals', {
      count: this.activeModals.length,
    });

    while (this.activeModals.length > 0) {
      this.closeCurrent();
    }
  }

  /**
   * Get active modal count
   */
  getActiveCount() {
    return this.activeModals.length;
  }

  /**
   * Check if a specific modal is open
   */
  isOpen(modalId) {
    return this.activeModals.some((m) => m.id === modalId);
  }
}

/**
 * Base Modal Class
 * All specific modals should extend this
 */
class BaseModal {
  constructor(data = {}) {
    this.data = data;
    this.title = 'Modal';
    this.icon = null;
    this.size = 'modal-md';
    this.footer = null;

    logger.debug('BaseModal', 'Modal instance created', {
      hasData: Object.keys(data).length > 0,
    });
  }

  /**
   * Render modal content (must be overridden)
   */
  render() {
    logger.warn('BaseModal', 'render() not implemented');
    return '<p>Modal content not implemented</p>';
  }

  /**
   * Called when modal is closed
   */
  onClose() {
    logger.debug('BaseModal', 'Modal closed');
  }
}

// Create singleton instance
const modalManager = new ModalManager();

// Expose to window for debugging
if (typeof window !== 'undefined') {
  window.outpostModal = modalManager;
  logger.debug('Modal', 'Exposed as window.outpostModal');
}

export { modalManager as default, BaseModal };

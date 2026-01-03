/**
 * Outpost 3 - Sidebar Component
 *
 * Collapsible sidebar with navigation, outliner, and quick actions.
 * All interactions and state changes are logged for debugging.
 */

import logger from '../utils/logger.js';
import stateManager from '../utils/state.js';

class Sidebar {
  constructor() {
    this.element = null;
    this.toggleButton = null;
    this.navItems = [];
    this.quickActionButtons = [];

    // Create reactive state for sidebar
    this.state = stateManager.createStore('sidebar', {
      collapsed: false,
      activeView: 'colony',
      initialized: false,
    });

    logger.info('Sidebar', 'Constructor called');
  }

  /**
   * Initialize the sidebar component
   */
  init() {
    logger.startPerformance('sidebar-init');
    logger.info('Sidebar', 'Initializing sidebar component');

    try {
      // Get DOM elements
      this.element = document.getElementById('app-sidebar');
      this.toggleButton = document.getElementById('sidebar-toggle');

      if (!this.element) {
        logger.error('Sidebar', 'Sidebar element not found');
        return;
      }

      logger.debug('Sidebar', 'DOM elements found', {
        sidebar: !!this.element,
        toggleButton: !!this.toggleButton,
      });

      // Get all navigation items
      this.navItems = Array.from(this.element.querySelectorAll('.nav-item'));
      logger.debug('Sidebar', 'Navigation items found', {
        count: this.navItems.length,
      });

      // Get quick action buttons
      this.quickActionButtons = [
        document.getElementById('btn-build'),
        document.getElementById('btn-advance-turn'),
      ].filter(Boolean);

      logger.debug('Sidebar', 'Quick action buttons found', {
        count: this.quickActionButtons.length,
      });

      // Load saved state
      this.loadState();

      // Set up event listeners
      this.setupEventListeners();

      // Mark as initialized
      this.state.initialized = true;

      logger.endPerformance('sidebar-init');
      logger.info('Sidebar', 'Initialization complete');
      logger.logRender('Sidebar', {
        collapsed: this.state.collapsed,
        activeView: this.state.activeView,
      });
    } catch (error) {
      logger.error('Sidebar', 'Initialization failed', {
        error: error.message,
        stack: error.stack,
      });
    }
  }

  /**
   * Load sidebar state from localStorage
   */
  loadState() {
    logger.debug('Sidebar', 'Loading saved state from localStorage');

    try {
      const savedCollapsed = localStorage.getItem('sidebar_collapsed');
      const savedView = localStorage.getItem('sidebar_active_view');

      if (savedCollapsed !== null) {
        const collapsed = savedCollapsed === 'true';
        this.state.collapsed = collapsed;

        if (collapsed) {
          document.body.classList.add('sidebar-collapsed');
        }

        logger.info('Sidebar', 'Restored collapsed state', { collapsed });
      }

      if (savedView) {
        this.state.activeView = savedView;
        this.setActiveNav(savedView);
        logger.info('Sidebar', 'Restored active view', { view: savedView });
      }
    } catch (error) {
      logger.error('Sidebar', 'Failed to load state', {
        error: error.message,
      });
    }
  }

  /**
   * Save sidebar state to localStorage
   */
  saveState() {
    logger.debug('Sidebar', 'Saving state to localStorage', {
      collapsed: this.state.collapsed,
      activeView: this.state.activeView,
    });

    try {
      localStorage.setItem('sidebar_collapsed', this.state.collapsed);
      localStorage.setItem('sidebar_active_view', this.state.activeView);
    } catch (error) {
      logger.error('Sidebar', 'Failed to save state', {
        error: error.message,
      });
    }
  }

  /**
   * Set up all event listeners with logging
   */
  setupEventListeners() {
    logger.debug('Sidebar', 'Setting up event listeners');

    // Toggle button
    if (this.toggleButton) {
      this.toggleButton.addEventListener('click', (e) => {
        logger.logInteraction('Sidebar', 'Click', 'toggle-button', {
          currentState: this.state.collapsed,
        });
        this.toggle();
      });
    }

    // Navigation items
    this.navItems.forEach((item, index) => {
      item.addEventListener('click', (e) => {
        const view = item.dataset.view;
        const href = item.getAttribute('href');

        logger.logInteraction('Sidebar', 'Click', 'nav-item', {
          view,
          href,
          index,
          preventDefault: e.defaultPrevented,
        });

        // Don't prevent default - let the link navigate
        // But update our state
        this.setActiveView(view);
      });

      // Hover logging for debugging
      item.addEventListener('mouseenter', () => {
        logger.debug('Sidebar', 'Nav item hover', {
          view: item.dataset.view,
        });
      });
    });

    // Quick action buttons
    this.quickActionButtons.forEach((button) => {
      if (!button) return;

      const action = button.dataset.action;

      button.addEventListener('click', (e) => {
        logger.logInteraction('Sidebar', 'Click', 'quick-action', {
          action,
          buttonId: button.id,
        });

        this.handleQuickAction(action, e);
      });
    });

    // Listen for custom events
    document.addEventListener('sidebar:toggle', () => {
      logger.debug('Sidebar', 'Custom event received: sidebar:toggle');
      this.toggle();
    });

    document.addEventListener('sidebar:collapse', () => {
      logger.debug('Sidebar', 'Custom event received: sidebar:collapse');
      this.collapse();
    });

    document.addEventListener('sidebar:expand', () => {
      logger.debug('Sidebar', 'Custom event received: sidebar:expand');
      this.expand();
    });

    logger.debug('Sidebar', 'Event listeners set up complete');
  }

  /**
   * Toggle sidebar collapsed state
   */
  toggle() {
    logger.startPerformance('sidebar-toggle');

    const newState = !this.state.collapsed;

    logger.info('Sidebar', 'Toggling sidebar', {
      from: this.state.collapsed,
      to: newState,
    });

    if (newState) {
      this.collapse();
    } else {
      this.expand();
    }

    logger.endPerformance('sidebar-toggle');
  }

  /**
   * Collapse the sidebar
   */
  collapse() {
    logger.info('Sidebar', 'Collapsing sidebar');

    this.state.collapsed = true;
    document.body.classList.add('sidebar-collapsed');
    this.saveState();

    logger.logRender('Sidebar', {
      collapsed: true,
      width: this.element.offsetWidth,
    });

    // Dispatch event for other components
    document.dispatchEvent(new CustomEvent('sidebar:collapsed', {
      detail: { collapsed: true },
    }));
  }

  /**
   * Expand the sidebar
   */
  expand() {
    logger.info('Sidebar', 'Expanding sidebar');

    this.state.collapsed = false;
    document.body.classList.remove('sidebar-collapsed');
    this.saveState();

    logger.logRender('Sidebar', {
      collapsed: false,
      width: this.element.offsetWidth,
    });

    // Dispatch event for other components
    document.dispatchEvent(new CustomEvent('sidebar:expanded', {
      detail: { collapsed: false },
    }));
  }

  /**
   * Set active view
   */
  setActiveView(view) {
    logger.info('Sidebar', 'Setting active view', {
      from: this.state.activeView,
      to: view,
    });

    this.state.activeView = view;
    this.setActiveNav(view);
    this.saveState();
  }

  /**
   * Update active navigation item
   */
  setActiveNav(view) {
    logger.debug('Sidebar', 'Updating active nav item', { view });

    this.navItems.forEach((item) => {
      const isActive = item.dataset.view === view;
      item.classList.toggle('active', isActive);
    });

    logger.logRender('Sidebar', {
      activeView: view,
      navItemsUpdated: this.navItems.length,
    });
  }

  /**
   * Handle quick action button clicks
   */
  handleQuickAction(action, event) {
    logger.info('Sidebar', 'Handling quick action', { action });

    switch (action) {
      case 'build':
        this.openBuildMenu();
        break;

      case 'advance-turn':
        this.advanceTurn();
        break;

      default:
        logger.warn('Sidebar', 'Unknown quick action', { action });
    }
  }

  /**
   * Open build menu modal
   */
  openBuildMenu() {
    logger.info('Sidebar', 'Opening build menu');

    // Dispatch custom event for modal system
    document.dispatchEvent(new CustomEvent('modal:open', {
      detail: {
        modal: 'build-menu',
        source: 'sidebar',
      },
    }));
  }

  /**
   * Advance turn
   */
  advanceTurn() {
    logger.info('Sidebar', 'Advance turn requested');

    // Dispatch custom event for app to handle
    document.dispatchEvent(new CustomEvent('game:advance-turn', {
      detail: {
        source: 'sidebar',
      },
    }));
  }

  /**
   * Get current state snapshot
   */
  getState() {
    return {
      collapsed: this.state.collapsed,
      activeView: this.state.activeView,
      initialized: this.state.initialized,
    };
  }

  /**
   * Destroy the component and clean up
   */
  destroy() {
    logger.info('Sidebar', 'Destroying sidebar component');

    // Remove event listeners would go here
    // For now, just log

    this.state.initialized = false;
  }
}

// Create singleton instance
const sidebar = new Sidebar();

// Expose to window for debugging
if (typeof window !== 'undefined') {
  window.outpostSidebar = sidebar;
  logger.debug('Sidebar', 'Exposed as window.outpostSidebar');
}

export default sidebar;

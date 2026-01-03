/**
 * Outpost 3 - Main Entry Point
 *
 * Initializes all UI systems with comprehensive logging.
 */

import logger from './utils/logger.js';
import stateManager from './utils/state.js';
import api from './utils/api.js';
import sidebar from './components/sidebar.js';
import outliner from './components/outliner.js';
import modalManager from './components/modal.js';
import BuildingDetailModal from './components/modals/building-detail.js';
import ConstructionModal from './components/modals/construction.js';

// Track initialization
logger.info('App', 'Starting Outpost 3 UI');
logger.startPerformance('app-init');

// Application state
const appState = stateManager.createStore('app', {
  initialized: false,
  currentView: 'colony',
  sidebarCollapsed: false,
  activeModal: null,
  colonyId: null,
  loading: false,
});

/**
 * Initialize the application
 */
async function init() {
  logger.info('App', 'Initializing application');

  try {
    // Get colony ID from URL or use default
    const colonyId = getColonyIdFromUrl() || 1;
    appState.colonyId = colonyId;

    logger.info('App', 'Colony ID determined', { colonyId });

    // Set up event listeners
    setupEventListeners();

    // Initialize components
    await initializeComponents();

    // Load sidebar state from localStorage
    loadSidebarState();

    // Mark as initialized
    appState.initialized = true;

    logger.endPerformance('app-init');
    logger.info('App', 'Application initialized successfully');

    // Show initialization stats
    logger.showStats();
  } catch (error) {
    logger.error('App', 'Initialization failed', {
      error: error.message,
      stack: error.stack,
    });

    showErrorMessage('Failed to initialize application. Please refresh the page.');
  }
}

/**
 * Get colony ID from URL path
 */
function getColonyIdFromUrl() {
  const match = window.location.pathname.match(/\/colony\/(\d+)/);
  return match ? parseInt(match[1], 10) : null;
}

/**
 * Set up global event listeners
 */
function setupEventListeners() {
  logger.debug('App', 'Setting up event listeners');

  // Keyboard shortcuts
  document.addEventListener('keydown', handleKeyPress);

  // Before unload - log session end
  window.addEventListener('beforeunload', () => {
    logger.info('App', 'Session ending', {
      duration: Date.now() - logger.startTime,
      renderCounts: Object.fromEntries(logger.renderCounts),
    });
  });

  // Log window resize
  window.addEventListener('resize', () => {
    logger.debug('App', 'Window resized', {
      width: window.innerWidth,
      height: window.innerHeight,
    });
  });

  // HTMX event logging (if HTMX is present)
  if (typeof htmx !== 'undefined') {
    setupHtmxLogging();
  }

  // Listen for game events from components
  document.addEventListener('game:advance-turn', (event) => {
    logger.debug('App', 'Custom event received: game:advance-turn', {
      source: event.detail?.source,
    });
    advanceTurn();
  });

  logger.debug('App', 'Event listeners set up');
}

/**
 * Set up HTMX event logging
 */
function setupHtmxLogging() {
  logger.debug('App', 'Setting up HTMX logging');

  document.body.addEventListener('htmx:configRequest', (event) => {
    logger.debug('HTMX', 'Request configured', {
      verb: event.detail.verb,
      path: event.detail.path,
    });
  });

  document.body.addEventListener('htmx:beforeSwap', (event) => {
    logger.debug('HTMX', 'Before swap', {
      target: event.detail.target.id || event.detail.target.className,
    });
  });

  document.body.addEventListener('htmx:afterSwap', (event) => {
    logger.debug('HTMX', 'After swap', {
      target: event.detail.target.id || event.detail.target.className,
    });
  });

  document.body.addEventListener('htmx:responseError', (event) => {
    logger.error('HTMX', 'Response error', {
      status: event.detail.xhr.status,
      path: event.detail.pathInfo.requestPath,
    });
  });
}

/**
 * Handle keyboard shortcuts
 */
function handleKeyPress(event) {
  // Don't trigger if user is typing in input
  if (event.target.matches('input, textarea, select')) {
    return;
  }

  const key = event.key.toLowerCase();

  logger.logInteraction('App', 'Keypress', key, {
    ctrl: event.ctrlKey,
    alt: event.altKey,
    shift: event.shiftKey,
  });

  switch (key) {
    case ' ':
      // Space: Advance turn
      event.preventDefault();
      advanceTurn();
      break;

    case 'escape':
      // Escape: Close modal
      event.preventDefault();
      closeActiveModal();
      break;

    case 'b':
      // B: Open build menu
      if (!event.ctrlKey) {
        event.preventDefault();
        openBuildMenu();
      }
      break;

    case 's':
      // S: Toggle sidebar
      if (!event.ctrlKey) {
        event.preventDefault();
        toggleSidebar();
      }
      break;

    // Number keys: Switch views
    case '1':
    case '2':
    case '3':
    case '4':
    case '5':
      if (!event.ctrlKey) {
        event.preventDefault();
        const views = ['colony', 'map', 'trains', 'economy', 'research'];
        switchView(views[parseInt(key) - 1]);
      }
      break;
  }
}

/**
 * Initialize UI components
 */
async function initializeComponents() {
  logger.info('App', 'Initializing components');

  try {
    // Initialize modal system
    modalManager.init();
    logger.debug('App', 'Modal system initialized');

    // Register modal types
    modalManager.register('building-detail', BuildingDetailModal);
    modalManager.register('build-menu', ConstructionModal);
    logger.debug('App', 'Modals registered');

    // Initialize sidebar
    sidebar.init();
    logger.debug('App', 'Sidebar initialized');

    // Initialize outliner if we have a colony ID
    if (appState.colonyId) {
      outliner.init(appState.colonyId);
      logger.debug('App', 'Outliner initialized', {
        colonyId: appState.colonyId,
      });
    }

    logger.info('App', 'Components initialized');
  } catch (error) {
    logger.error('App', 'Component initialization failed', {
      error: error.message,
      stack: error.stack,
    });
  }
}

/**
 * Load sidebar state from localStorage
 */
function loadSidebarState() {
  // Sidebar component handles its own state loading
  logger.debug('App', 'Sidebar state will be loaded by sidebar component');
}

/**
 * Toggle sidebar
 */
function toggleSidebar() {
  logger.debug('App', 'Toggle sidebar function called');
  sidebar.toggle();
}

/**
 * Switch view
 */
function switchView(viewName) {
  logger.logInteraction('App', 'Switch view', viewName);
  appState.currentView = viewName;

  // Update UI
  document.querySelectorAll('[data-view]').forEach(el => {
    el.classList.toggle('active', el.dataset.view === viewName);
  });
}

/**
 * Advance turn
 */
async function advanceTurn() {
  if (appState.loading || !appState.colonyId) {
    return;
  }

  logger.info('App', 'Advancing turn', { colonyId: appState.colonyId });
  appState.loading = true;

  try {
    await api.advanceTurn(appState.colonyId);
    logger.info('App', 'Turn advanced successfully');

    // Reload page or trigger HTMX refresh
    window.location.reload();
  } catch (error) {
    logger.error('App', 'Failed to advance turn', {
      error: error.message,
    });

    showErrorMessage('Failed to advance turn');
  } finally {
    appState.loading = false;
  }
}

/**
 * Close active modal
 */
function closeActiveModal() {
  if (appState.activeModal) {
    logger.logModalEvent(appState.activeModal, 'CLOSE_VIA_KEYBOARD');
    // Modal closing logic will be implemented in modal component
    const event = new CustomEvent('modal:close');
    document.dispatchEvent(event);
  }
}

/**
 * Open build menu
 */
function openBuildMenu() {
  logger.logInteraction('App', 'Open build menu', 'keyboard');
  // Build menu logic will be implemented in modal component
  const event = new CustomEvent('modal:open', { detail: { modal: 'build-menu' } });
  document.dispatchEvent(event);
}

/**
 * Show error message
 */
function showErrorMessage(message) {
  logger.error('App', 'Showing error to user', { message });

  // Simple alert for now - will be replaced with toast notification
  alert(message);
}

/**
 * Expose debug functions to window
 */
if (typeof window !== 'undefined') {
  window.outpost = {
    logger,
    stateManager,
    api,
    appState,
    sidebar,
    outliner,
    modalManager,
    toggleSidebar,
    switchView,
    advanceTurn,
    exportLogs: () => logger.downloadLogs(),
    showStats: () => logger.showStats(),
  };

  console.log('[App] Debug interface exposed as window.outpost');
  console.log('[App] Available: logger, stateManager, api, appState, sidebar, outliner, modalManager');
  console.log('[App] Functions: toggleSidebar, switchView, advanceTurn');
  console.log('[App] Commands: outpost.exportLogs(), outpost.showStats()');
}

// Initialize when DOM is ready
if (document.readyState === 'loading') {
  document.addEventListener('DOMContentLoaded', init);
  logger.debug('App', 'Waiting for DOMContentLoaded');
} else {
  init();
  logger.debug('App', 'DOM already loaded, initializing immediately');
}

export { appState, init };

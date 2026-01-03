/**
 * Outpost 3 State Management
 *
 * Simple reactive state management with automatic change logging.
 * Inspired by Alpine.js but with explicit logging for debugging.
 */

import logger from './logger.js';

class StateManager {
  constructor() {
    this.stores = new Map();
    this.subscribers = new Map();

    logger.info('StateManager', 'Initialized');
  }

  /**
   * Create a new reactive store
   */
  createStore(name, initialState = {}) {
    if (this.stores.has(name)) {
      logger.warn('StateManager', 'Store already exists', { name });
      return this.stores.get(name);
    }

    logger.info('StateManager', 'Creating store', { name, initialState });

    const proxy = this.createReactiveProxy(name, initialState);
    this.stores.set(name, proxy);
    this.subscribers.set(name, new Set());

    return proxy;
  }

  /**
   * Create reactive proxy that logs all state changes
   */
  createReactiveProxy(storeName, state) {
    const self = this;

    return new Proxy(state, {
      get(target, property) {
        const value = target[property];

        // If it's an object, wrap it in a proxy too
        if (typeof value === 'object' && value !== null && !Array.isArray(value)) {
          return self.createReactiveProxy(`${storeName}.${property}`, value);
        }

        return value;
      },

      set(target, property, value) {
        const oldValue = target[property];

        if (oldValue === value) {
          return true; // No change
        }

        logger.logStateChange(storeName, property, oldValue, value);

        target[property] = value;

        // Notify subscribers
        self.notifySubscribers(storeName, property, value, oldValue);

        return true;
      },
    });
  }

  /**
   * Get a store by name
   */
  getStore(name) {
    return this.stores.get(name);
  }

  /**
   * Subscribe to state changes
   */
  subscribe(storeName, callback) {
    if (!this.subscribers.has(storeName)) {
      this.subscribers.set(storeName, new Set());
    }

    this.subscribers.get(storeName).add(callback);

    logger.debug('StateManager', 'Subscriber added', { storeName });

    // Return unsubscribe function
    return () => {
      this.subscribers.get(storeName).delete(callback);
      logger.debug('StateManager', 'Subscriber removed', { storeName });
    };
  }

  /**
   * Notify all subscribers of a change
   */
  notifySubscribers(storeName, property, newValue, oldValue) {
    const subscribers = this.subscribers.get(storeName);
    if (!subscribers) return;

    logger.debug('StateManager', 'Notifying subscribers', {
      storeName,
      property,
      subscriberCount: subscribers.size,
    });

    subscribers.forEach(callback => {
      try {
        callback(property, newValue, oldValue);
      } catch (error) {
        logger.error('StateManager', 'Subscriber callback error', {
          storeName,
          property,
          error: error.message,
        });
      }
    });
  }

  /**
   * Get current state snapshot (non-reactive)
   */
  getSnapshot(storeName) {
    const store = this.stores.get(storeName);
    if (!store) return null;

    return JSON.parse(JSON.stringify(store));
  }

  /**
   * Reset store to initial state
   */
  resetStore(storeName, newState = {}) {
    logger.info('StateManager', 'Resetting store', { storeName });

    const proxy = this.createReactiveProxy(storeName, newState);
    this.stores.set(storeName, proxy);

    return proxy;
  }

  /**
   * Debug: show all stores
   */
  showStores() {
    const snapshot = {};
    this.stores.forEach((store, name) => {
      snapshot[name] = this.getSnapshot(name);
    });

    console.table(snapshot);
    return snapshot;
  }
}

// Create singleton
const stateManager = new StateManager();

// Expose to window for debugging
if (typeof window !== 'undefined') {
  window.outpostState = stateManager;
  console.log('[StateManager] Exposed as window.outpostState');
}

export default stateManager;

/**
 * Outpost 3 UI Logger
 *
 * Comprehensive logging system for UI events, render cycles, and state changes.
 * Helps with debugging and testing by tracking when and how UI components update.
 */

const LOG_LEVELS = {
  DEBUG: 0,
  INFO: 1,
  WARN: 2,
  ERROR: 3,
  NONE: 4,
};

class Logger {
  constructor() {
    this.level = LOG_LEVELS.DEBUG; // Default to DEBUG for development
    this.eventLog = [];
    this.maxLogSize = 1000;
    this.startTime = Date.now();
    this.enableConsole = true;
    this.enableStorage = true;

    // Performance tracking
    this.performanceMarks = new Map();
    this.renderCounts = new Map();

    console.log('[Logger] UI Logger initialized', {
      timestamp: new Date().toISOString(),
      level: this.getLevelName(this.level),
    });
  }

  getLevelName(level) {
    return Object.keys(LOG_LEVELS).find(key => LOG_LEVELS[key] === level) || 'UNKNOWN';
  }

  setLevel(levelName) {
    if (LOG_LEVELS[levelName] !== undefined) {
      this.level = LOG_LEVELS[levelName];
      this.info('Logger', 'Log level changed', { newLevel: levelName });
    }
  }

  shouldLog(level) {
    return level >= this.level;
  }

  formatMessage(component, event, data = {}) {
    const elapsed = Date.now() - this.startTime;
    return {
      timestamp: new Date().toISOString(),
      elapsed: `${elapsed}ms`,
      component,
      event,
      data,
    };
  }

  log(level, component, event, data = {}) {
    if (!this.shouldLog(level)) return;

    const message = this.formatMessage(component, event, data);
    const levelName = this.getLevelName(level);

    // Store in memory
    this.eventLog.push({ level: levelName, ...message });
    if (this.eventLog.length > this.maxLogSize) {
      this.eventLog.shift();
    }

    // Console output with colors
    if (this.enableConsole) {
      const color = this.getColorForLevel(level);
      const prefix = `[${levelName}][${component}]`;
      console.log(
        `%c${prefix}%c ${event}`,
        `color: ${color}; font-weight: bold`,
        'color: inherit',
        data
      );
    }

    // LocalStorage persistence for critical events
    if (this.enableStorage && level >= LOG_LEVELS.WARN) {
      this.persistToStorage(levelName, component, event, data);
    }
  }

  getColorForLevel(level) {
    switch (level) {
      case LOG_LEVELS.DEBUG: return '#6c757d';
      case LOG_LEVELS.INFO: return '#0dcaf0';
      case LOG_LEVELS.WARN: return '#ffc107';
      case LOG_LEVELS.ERROR: return '#dc3545';
      default: return '#ffffff';
    }
  }

  persistToStorage(level, component, event, data) {
    try {
      const key = `outpost3_log_${Date.now()}`;
      const entry = JSON.stringify({ level, component, event, data, timestamp: new Date().toISOString() });
      localStorage.setItem(key, entry);

      // Clean old entries (keep last 50)
      const keys = Object.keys(localStorage).filter(k => k.startsWith('outpost3_log_'));
      if (keys.length > 50) {
        keys.sort().slice(0, keys.length - 50).forEach(k => localStorage.removeItem(k));
      }
    } catch (e) {
      console.error('Failed to persist log to storage', e);
    }
  }

  debug(component, event, data) {
    this.log(LOG_LEVELS.DEBUG, component, event, data);
  }

  info(component, event, data) {
    this.log(LOG_LEVELS.INFO, component, event, data);
  }

  warn(component, event, data) {
    this.log(LOG_LEVELS.WARN, component, event, data);
  }

  error(component, event, data) {
    this.log(LOG_LEVELS.ERROR, component, event, data);
  }

  // UI-specific logging methods

  /**
   * Log a render event
   */
  logRender(component, data = {}) {
    const count = (this.renderCounts.get(component) || 0) + 1;
    this.renderCounts.set(component, count);

    this.debug(component, 'RENDER', {
      ...data,
      renderCount: count,
    });
  }

  /**
   * Log a state change
   */
  logStateChange(component, stateName, oldValue, newValue) {
    this.info(component, 'STATE_CHANGE', {
      state: stateName,
      from: oldValue,
      to: newValue,
    });
  }

  /**
   * Log a user interaction
   */
  logInteraction(component, action, target, data = {}) {
    this.info(component, 'USER_INTERACTION', {
      action,
      target,
      ...data,
    });
  }

  /**
   * Log API call
   */
  logApiCall(endpoint, method, data = {}) {
    this.debug('API', `${method} ${endpoint}`, data);
  }

  /**
   * Log API response
   */
  logApiResponse(endpoint, method, status, data = {}) {
    const level = status >= 400 ? LOG_LEVELS.ERROR : LOG_LEVELS.DEBUG;
    this.log(level, 'API', `${method} ${endpoint} → ${status}`, data);
  }

  /**
   * Performance monitoring
   */
  startPerformance(markName) {
    this.performanceMarks.set(markName, performance.now());
    this.debug('Performance', `START: ${markName}`);
  }

  endPerformance(markName, data = {}) {
    const start = this.performanceMarks.get(markName);
    if (start !== undefined) {
      const duration = performance.now() - start;
      this.performanceMarks.delete(markName);

      const level = duration > 100 ? LOG_LEVELS.WARN : LOG_LEVELS.DEBUG;
      this.log(level, 'Performance', `END: ${markName}`, {
        duration: `${duration.toFixed(2)}ms`,
        ...data,
      });
    }
  }

  /**
   * Canvas/rendering specific logging
   */
  logCanvasEvent(eventType, data = {}) {
    this.debug('Canvas', eventType, data);
  }

  /**
   * Chart rendering logging
   */
  logChartEvent(chartType, eventType, data = {}) {
    this.debug(`Chart:${chartType}`, eventType, data);
  }

  /**
   * Modal state logging
   */
  logModalEvent(modalId, eventType, data = {}) {
    this.info('Modal', `${modalId}: ${eventType}`, data);
  }

  /**
   * Get event log for debugging
   */
  getEventLog(filter = {}) {
    let logs = this.eventLog;

    if (filter.component) {
      logs = logs.filter(log => log.component === filter.component);
    }

    if (filter.level) {
      logs = logs.filter(log => log.level === filter.level);
    }

    if (filter.limit) {
      logs = logs.slice(-filter.limit);
    }

    return logs;
  }

  /**
   * Export logs as JSON
   */
  exportLogs() {
    const data = {
      exportedAt: new Date().toISOString(),
      logLevel: this.getLevelName(this.level),
      events: this.eventLog,
      renderCounts: Object.fromEntries(this.renderCounts),
    };

    return JSON.stringify(data, null, 2);
  }

  /**
   * Download logs as file
   */
  downloadLogs() {
    const json = this.exportLogs();
    const blob = new Blob([json], { type: 'application/json' });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = `outpost3-logs-${Date.now()}.json`;
    a.click();
    URL.revokeObjectURL(url);

    this.info('Logger', 'Logs downloaded');
  }

  /**
   * Clear all logs
   */
  clearLogs() {
    const count = this.eventLog.length;
    this.eventLog = [];
    this.renderCounts.clear();
    this.performanceMarks.clear();

    console.log(`[Logger] Cleared ${count} log entries`);
  }

  /**
   * Display summary statistics
   */
  showStats() {
    const stats = {
      totalEvents: this.eventLog.length,
      eventsByLevel: {},
      eventsByComponent: {},
      renderCounts: Object.fromEntries(this.renderCounts),
      uptime: Date.now() - this.startTime,
    };

    // Count by level
    this.eventLog.forEach(log => {
      stats.eventsByLevel[log.level] = (stats.eventsByLevel[log.level] || 0) + 1;
      stats.eventsByComponent[log.component] = (stats.eventsByComponent[log.component] || 0) + 1;
    });

    console.table(stats);
    return stats;
  }
}

// Create singleton instance
const logger = new Logger();

// Expose to window for debugging
if (typeof window !== 'undefined') {
  window.outpostLogger = logger;
  console.log('[Logger] Logger exposed as window.outpostLogger');
  console.log('[Logger] Available commands: .exportLogs(), .downloadLogs(), .showStats(), .clearLogs()');
}

export default logger;

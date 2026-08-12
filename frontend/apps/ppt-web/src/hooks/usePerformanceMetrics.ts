/**
 * Performance Metrics Hook
 * Epic 130: Performance Optimization
 *
 * Monitors Core Web Vitals and navigation performance metrics.
 * Uses the web-vitals library pattern for measuring LCP, FID, CLS.
 */

import { useEffect, useRef } from 'react';

export interface PerformanceMetrics {
  /** Largest Contentful Paint - measures loading performance */
  lcp?: number;
  /** First Input Delay - measures interactivity */
  fid?: number;
  /** Cumulative Layout Shift - measures visual stability */
  cls?: number;
  /** Time to First Byte */
  ttfb?: number;
  /** First Contentful Paint */
  fcp?: number;
  /** Navigation timing */
  navigationTiming?: {
    dnsLookup: number;
    tcpConnection: number;
    serverResponse: number;
    domContentLoaded: number;
    loadComplete: number;
  };
}

type MetricsCallback = (metrics: PerformanceMetrics) => void;

/**
 * Hook to collect and report Core Web Vitals and performance metrics.
 *
 * @param onReport - Callback function invoked with collected metrics
 * @param options - Configuration options
 */
export function usePerformanceMetrics(
  onReport?: MetricsCallback,
  options: { reportOnUnload?: boolean } = {}
) {
  const metricsRef = useRef<PerformanceMetrics>({});
  const { reportOnUnload = true } = options;

  // Keep the latest onReport in a ref so the effect does not need it as a
  // dependency. Passing onReport inline (a fresh function each render) would
  // otherwise re-run the effect on every render, tearing down and re-adding
  // observers and event listeners repeatedly.
  const onReportRef = useRef<MetricsCallback | undefined>(onReport);
  onReportRef.current = onReport;

  useEffect(() => {
    // Skip if PerformanceObserver is not available
    if (typeof PerformanceObserver === 'undefined') {
      return;
    }

    const observers: PerformanceObserver[] = [];

    // Observe Largest Contentful Paint (LCP)
    try {
      const lcpObserver = new PerformanceObserver((entryList) => {
        const entries = entryList.getEntries();
        const lastEntry = entries[entries.length - 1] as PerformanceEntry & { startTime: number };
        if (lastEntry) {
          metricsRef.current.lcp = lastEntry.startTime;
        }
      });
      lcpObserver.observe({ type: 'largest-contentful-paint', buffered: true });
      observers.push(lcpObserver);
    } catch {
      // LCP observation not supported
    }

    // Observe First Input Delay (FID)
    try {
      const fidObserver = new PerformanceObserver((entryList) => {
        const entries = entryList.getEntries();
        const firstEntry = entries[0] as PerformanceEventTiming | undefined;
        if (firstEntry) {
          metricsRef.current.fid = firstEntry.processingStart - firstEntry.startTime;
        }
      });
      fidObserver.observe({ type: 'first-input', buffered: true });
      observers.push(fidObserver);
    } catch {
      // FID observation not supported
    }

    // Observe Cumulative Layout Shift (CLS)
    try {
      let clsValue = 0;
      const clsObserver = new PerformanceObserver((entryList) => {
        for (const entry of entryList.getEntries()) {
          const layoutShift = entry as PerformanceEntry & {
            hadRecentInput: boolean;
            value: number;
          };
          if (!layoutShift.hadRecentInput) {
            clsValue += layoutShift.value;
            metricsRef.current.cls = clsValue;
          }
        }
      });
      clsObserver.observe({ type: 'layout-shift', buffered: true });
      observers.push(clsObserver);
    } catch {
      // CLS observation not supported
    }

    // Observe First Contentful Paint (FCP)
    try {
      const fcpObserver = new PerformanceObserver((entryList) => {
        const entries = entryList.getEntries();
        const fcpEntry = entries.find((e) => e.name === 'first-contentful-paint');
        if (fcpEntry) {
          metricsRef.current.fcp = fcpEntry.startTime;
        }
      });
      fcpObserver.observe({ type: 'paint', buffered: true });
      observers.push(fcpObserver);
    } catch {
      // FCP observation not supported
    }

    // Collect navigation timing metrics
    const collectNavigationTiming = () => {
      const navEntry = performance.getEntriesByType('navigation')[0] as
        | PerformanceNavigationTiming
        | undefined;
      if (navEntry) {
        metricsRef.current.ttfb = navEntry.responseStart - navEntry.requestStart;
        metricsRef.current.navigationTiming = {
          dnsLookup: navEntry.domainLookupEnd - navEntry.domainLookupStart,
          tcpConnection: navEntry.connectEnd - navEntry.connectStart,
          serverResponse: navEntry.responseEnd - navEntry.requestStart,
          domContentLoaded: navEntry.domContentLoadedEventEnd - navEntry.fetchStart,
          loadComplete: navEntry.loadEventEnd - navEntry.fetchStart,
        };
      }
    };

    // Collect after page load. Track whether we registered the listener so
    // cleanup can remove exactly what it added.
    let loadListenerAdded = false;
    if (document.readyState === 'complete') {
      collectNavigationTiming();
    } else {
      window.addEventListener('load', collectNavigationTiming);
      loadListenerAdded = true;
    }

    // Report metrics on page unload
    const reportMetrics = () => {
      const onReport = onReportRef.current;
      if (onReport && Object.keys(metricsRef.current).length > 0) {
        onReport(metricsRef.current);
      }
    };

    // Named handler (not an inline closure) so removeEventListener can target
    // the exact same reference on cleanup.
    const handleVisibilityChange = () => {
      if (document.visibilityState === 'hidden') {
        reportMetrics();
      }
    };

    if (reportOnUnload) {
      window.addEventListener('visibilitychange', handleVisibilityChange);
    }

    return () => {
      for (const observer of observers) {
        observer.disconnect();
      }
      if (loadListenerAdded) {
        window.removeEventListener('load', collectNavigationTiming);
      }
      if (reportOnUnload) {
        window.removeEventListener('visibilitychange', handleVisibilityChange);
      }
    };
  }, [reportOnUnload]);

  return metricsRef.current;
}

/**
 * Utility to log performance metrics to console in development.
 */
export function logPerformanceMetrics(metrics: PerformanceMetrics) {
  if (!import.meta.env.DEV) return;

  const formatMs = (ms?: number) => (ms !== undefined ? `${ms.toFixed(2)}ms` : 'N/A');

  console.group('📊 Performance Metrics');
  console.log(`LCP (Largest Contentful Paint): ${formatMs(metrics.lcp)}`);
  console.log(`FID (First Input Delay): ${formatMs(metrics.fid)}`);
  console.log(`CLS (Cumulative Layout Shift): ${metrics.cls?.toFixed(4) ?? 'N/A'}`);
  console.log(`TTFB (Time to First Byte): ${formatMs(metrics.ttfb)}`);
  console.log(`FCP (First Contentful Paint): ${formatMs(metrics.fcp)}`);

  if (metrics.navigationTiming) {
    console.group('Navigation Timing');
    console.log(`DNS Lookup: ${formatMs(metrics.navigationTiming.dnsLookup)}`);
    console.log(`TCP Connection: ${formatMs(metrics.navigationTiming.tcpConnection)}`);
    console.log(`Server Response: ${formatMs(metrics.navigationTiming.serverResponse)}`);
    console.log(`DOM Content Loaded: ${formatMs(metrics.navigationTiming.domContentLoaded)}`);
    console.log(`Load Complete: ${formatMs(metrics.navigationTiming.loadComplete)}`);
    console.groupEnd();
  }

  console.groupEnd();
}

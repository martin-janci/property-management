/**
 * Real-time IoT sensor reading WebSocket hook (Epic 14, FR72 — BIT-145 backend channel).
 *
 * Connects to `GET /api/v1/iot/sensors/ws?token=<jwt>&organization_id=<org>`,
 * subscribes to the `sensors:{org_id}` Redis pub/sub channel, and streams
 * `{ event: "sensor.reading.created", payload: SensorReading }` frames.
 *
 * The backend re-checks active membership of `organization_id` before the WS
 * upgrade, so the channel can never leak another tenant's readings (issue
 * #1668 converged on this DB-checked handler and removed the JWT-trusting
 * `/api/v1/iot/ws` channel).
 *
 * The REST polling hook (`useSensorReadings`) remains the primary data source.
 * This hook prepends fresh readings so the telemetry chart updates in real-time
 * without waiting for the next poll interval.
 *
 * Falls back gracefully when the WS server is unreachable or Redis is not
 * configured (the backend returns heartbeat-only frames in that case).
 *
 * Reconnection mirrors the shared notification stack (`WebSocketService` in
 * `lib/websocket.ts`): exponential backoff with jitter via the shared
 * {@link computeReconnectDelay} helper, a bounded attempt budget
 * ({@link DEFAULT_MAX_RECONNECT_ATTEMPTS}), and a terminal give-up so a dead
 * endpoint can no longer drive a fixed-interval reconnect storm. The budget
 * resets on every successful open, so a transient drop still gets a full set
 * of attempts. The two sockets run on different transports/paths, so only the
 * backoff math is shared — not the whole service.
 */

import type { SensorReading } from '@ppt/api-client';
import { getToken } from '@ppt/api-client';
import { useEffect, useMemo, useRef, useState } from 'react';
import { computeReconnectDelay, DEFAULT_MAX_RECONNECT_ATTEMPTS } from '../../../lib/websocket';

const WS_MAX_BUFFER = 200;

/** Event name emitted by the backend for a single newly-ingested reading. */
const EVENT_READING_CREATED = 'sensor.reading.created';

function buildWsUrl(token: string, organizationId: string): string {
  const win = typeof window !== 'undefined' ? (window as unknown as Record<string, unknown>) : {};
  const httpBase = win.__API_BASE_URL__ ? String(win.__API_BASE_URL__) : '';
  const wsBase = httpBase.replace(/^https/, 'wss').replace(/^http/, 'ws');
  const qs = new URLSearchParams({ token, organization_id: organizationId });
  return `${wsBase}/api/v1/iot/sensors/ws?${qs.toString()}`;
}

export function useIotWebSocket(
  sensorId: string | null,
  organizationId: string | null
): {
  liveReadings: SensorReading[];
  isConnected: boolean;
  /**
   * True once the reconnect budget is exhausted and the hook has stopped
   * retrying. Mirrors the shared stack's `connection:max-retries-exceeded`
   * signal so the UI can surface an "offline" state instead of assuming a
   * silent-but-live socket.
   */
  hasGivenUp: boolean;
} {
  const [allReadings, setAllReadings] = useState<SensorReading[]>([]);
  const [isConnected, setIsConnected] = useState(false);
  const [hasGivenUp, setHasGivenUp] = useState(false);
  const reconnectTimer = useRef<ReturnType<typeof setTimeout> | null>(null);
  const reconnectAttempts = useRef(0);
  const wsRef = useRef<WebSocket | null>(null);
  const cancelled = useRef(false);

  useEffect(() => {
    cancelled.current = false;
    reconnectAttempts.current = 0;
    setHasGivenUp(false);

    // Schedule a reconnect with exponential backoff, or give up once the
    // attempt budget is exhausted — same policy as WebSocketService. Only an
    // onopen (success) resets the budget, so a continuous outage still reaches
    // the cap instead of retrying forever.
    function scheduleReconnect() {
      if (cancelled.current) return;

      if (reconnectAttempts.current >= DEFAULT_MAX_RECONNECT_ATTEMPTS) {
        setHasGivenUp(true);
        return;
      }

      const delay = computeReconnectDelay(reconnectAttempts.current);
      reconnectAttempts.current += 1;
      reconnectTimer.current = setTimeout(() => {
        if (!cancelled.current) connect();
      }, delay);
    }

    function connect() {
      const token = getToken();
      if (!token || !organizationId || cancelled.current) return;

      const ws = new WebSocket(buildWsUrl(token, organizationId));
      wsRef.current = ws;

      ws.onopen = () => {
        if (cancelled.current) return;
        // Successful open — refill the reconnect budget.
        reconnectAttempts.current = 0;
        setHasGivenUp(false);
        setIsConnected(true);
      };

      ws.onmessage = (event) => {
        if (cancelled.current) return;
        try {
          const msg = JSON.parse(String(event.data)) as {
            event: string;
            payload: SensorReading;
          };
          if (msg.event === EVENT_READING_CREATED) {
            setAllReadings((prev: SensorReading[]) => {
              const next = [msg.payload, ...prev];
              return next.length > WS_MAX_BUFFER ? next.slice(0, WS_MAX_BUFFER) : next;
            });
          }
        } catch {
          // ignore malformed frames
        }
      };

      ws.onerror = () => {
        ws.close();
      };

      ws.onclose = () => {
        if (!cancelled.current) {
          setIsConnected(false);
          scheduleReconnect();
        }
      };
    }

    connect();

    return () => {
      cancelled.current = true;
      if (reconnectTimer.current) clearTimeout(reconnectTimer.current);
      wsRef.current?.close();
      wsRef.current = null;
    };
  }, [organizationId]);

  const liveReadings = useMemo(
    () => (sensorId ? allReadings.filter((r: SensorReading) => r.sensor_id === sensorId) : []),
    [allReadings, sensorId]
  );

  return { liveReadings, isConnected, hasGivenUp };
}

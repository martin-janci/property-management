/**
 * WebSocket Hook for ppt-web (Story 79.4)
 *
 * Provides a convenient hook for subscribing to WebSocket events
 * with automatic cleanup on unmount.
 */

import { useCallback, useEffect, useRef } from 'react';
import { useWebSocketContext } from '../contexts/WebSocketContext';
import type { MessageHandler, WebSocketMessage } from '../lib/websocket';

/**
 * Return value from useWebSocket hook.
 */
export interface UseWebSocketResult {
  /**
   * Whether the WebSocket is currently connected.
   */
  isConnected: boolean;

  /**
   * Whether the WebSocket is currently connecting.
   */
  isConnecting: boolean;

  /**
   * The last connection error, if any.
   */
  error: Error | null;

  /**
   * Send a message through the WebSocket.
   *
   * @param message - The message to send.
   * @returns true if the message was sent successfully.
   */
  send: (message: WebSocketMessage) => boolean;

  /**
   * Manually reconnect to the WebSocket server.
   */
  reconnect: () => void;
}

/**
 * Hook for subscribing to WebSocket events.
 *
 * Automatically cleans up subscriptions on unmount.
 *
 * @param eventType - The event type to subscribe to, or '*' for all events. Pass null to not subscribe.
 * @param handler - The handler function to call when a message is received.
 * @returns Connection state and send method.
 *
 * @example
 * ```tsx
 * function MessageList() {
 *   const [messages, setMessages] = useState<Message[]>([]);
 *
 *   const { isConnected } = useWebSocket('message:new', (message) => {
 *     setMessages((prev) => [...prev, message.payload as Message]);
 *   });
 *
 *   return (
 *     <div>
 *       {isConnected ? 'Connected' : 'Disconnected'}
 *       {messages.map((msg) => <MessageItem key={msg.id} message={msg} />)}
 *     </div>
 *   );
 * }
 * ```
 */
export function useWebSocket(
  eventType: string | null,
  handler?: MessageHandler
): UseWebSocketResult {
  const { isConnected, isConnecting, error, subscribe, send, reconnect } = useWebSocketContext();

  // Store handler in ref to avoid re-subscribing on handler changes
  const handlerRef = useRef<MessageHandler | undefined>(handler);
  handlerRef.current = handler;

  // Track handler *presence* (not identity). A handler supplied after mount
  // must (re)subscribe, and clearing the handler must unsubscribe — but
  // swapping one handler function for another must NOT cause resubscribe churn
  // (that is what the ref above is for). Gating the effect on this boolean
  // gives us exactly that: it flips only when the handler appears or disappears.
  const hasHandler = handlerRef.current != null;

  // Subscribe to events
  useEffect(() => {
    if (!eventType || !hasHandler) {
      return;
    }

    const wrappedHandler: MessageHandler = (message) => {
      handlerRef.current?.(message);
    };

    const unsubscribe = subscribe(eventType, wrappedHandler);

    return unsubscribe;
  }, [eventType, subscribe, hasHandler]);

  return {
    isConnected,
    isConnecting,
    error,
    send,
    reconnect,
  };
}

/**
 * Hook for subscribing to multiple WebSocket event types.
 *
 * Automatically cleans up subscriptions on unmount.
 *
 * @param subscriptions - Map of event types to handlers.
 * @returns Connection state and send method.
 *
 * @example
 * ```tsx
 * function Dashboard() {
 *   const { isConnected } = useWebSocketSubscriptions({
 *     'message:new': handleNewMessage,
 *     'notification:announcement': handleAnnouncement,
 *     'entity:updated': handleEntityUpdate,
 *   });
 *
 *   return <div>{isConnected ? 'Live' : 'Offline'}</div>;
 * }
 * ```
 */
export function useWebSocketSubscriptions(
  subscriptions: Record<string, MessageHandler>
): UseWebSocketResult {
  const { isConnected, isConnecting, error, subscribe, send, reconnect } = useWebSocketContext();

  // Store subscriptions in ref to handle updates properly
  const subscriptionsRef = useRef(subscriptions);
  subscriptionsRef.current = subscriptions;

  // Memoize the event types to detect when subscriptions change
  const eventTypes = Object.keys(subscriptions);
  const eventTypesKey = eventTypes.join(',');

  // Subscribe to all event types
  // biome-ignore lint/correctness/useExhaustiveDependencies: eventTypesKey is a memoized key that changes when eventTypes changes
  useEffect(() => {
    const unsubscribers: (() => void)[] = [];

    for (const eventType of eventTypes) {
      const wrappedHandler: MessageHandler = (message) => {
        subscriptionsRef.current[eventType]?.(message);
      };

      const unsubscribe = subscribe(eventType, wrappedHandler);
      unsubscribers.push(unsubscribe);
    }

    return () => {
      for (const unsubscribe of unsubscribers) {
        unsubscribe();
      }
    };
  }, [subscribe, eventTypesKey]);

  return {
    isConnected,
    isConnecting,
    error,
    send,
    reconnect,
  };
}

/**
 * Hook for getting WebSocket connection state without subscribing to events.
 *
 * @returns Connection state and send method.
 *
 * @example
 * ```tsx
 * function Header() {
 *   const { isConnected, error } = useWebSocketState();
 *
 *   return (
 *     <header>
 *       <ConnectionStatus isConnected={isConnected} error={error} />
 *     </header>
 *   );
 * }
 * ```
 */
export function useWebSocketState(): UseWebSocketResult {
  const { isConnected, isConnecting, error, send, reconnect } = useWebSocketContext();

  return {
    isConnected,
    isConnecting,
    error,
    send,
    reconnect,
  };
}

/**
 * Hook for sending messages via WebSocket.
 *
 * @returns A memoized send function.
 *
 * @example
 * ```tsx
 * function ChatInput() {
 *   const sendMessage = useWebSocketSend();
 *
 *   const handleSend = (text: string) => {
 *     sendMessage({
 *       type: 'chat:message',
 *       payload: { text },
 *       timestamp: new Date().toISOString(),
 *     });
 *   };
 *
 *   return <input onSubmit={handleSend} />;
 * }
 * ```
 */
export function useWebSocketSend(): (message: WebSocketMessage) => boolean {
  const { send } = useWebSocketContext();
  return useCallback((message: WebSocketMessage) => send(message), [send]);
}

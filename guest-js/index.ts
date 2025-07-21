/**
 * Tauri Awesome RPC Guest JS API
 * 
 * Provides TypeScript bindings for the AwesomeEvent system
 */

interface AwesomeEventListener {
  (data: any): void;
}

interface AwesomeEventAPI {
  /**
   * Listen to events emitted from the Rust backend
   * @param eventName The name of the event to listen for
   * @param callback Function to call when the event is received
   * @returns A function to unsubscribe from the event
   */
  listen(eventName: string, callback: AwesomeEventListener): () => void;
  
  /**
   * Listen to an event once. The listener is automatically removed after the first event
   * @param eventName The name of the event to listen for
   * @param callback Function to call when the event is received
   * @returns A function to unsubscribe from the event
   */
  once(eventName: string, callback: AwesomeEventListener): () => void;
}

declare global {
  interface Window {
    AwesomeEvent: AwesomeEventAPI;
  }
}

/**
 * Listen to an event emitted from the Rust backend using AwesomeEmit
 * @param eventName The name of the event to listen for
 * @param callback Function to call when the event is received
 * @returns A function to unsubscribe from the event
 */
export function listen(eventName: string, callback: AwesomeEventListener): () => void {
  if (!window.AwesomeEvent) {
    throw new Error('AwesomeEvent is not initialized. Make sure tauri-awesome-rpc is properly configured.');
  }
  
  return window.AwesomeEvent.listen(eventName, callback);
}

/**
 * Listen to an event once. The listener is automatically removed after the first event
 * @param eventName The name of the event to listen for
 * @param callback Function to call when the event is received
 * @returns A function to unsubscribe from the event
 */
export function once(eventName: string, callback: AwesomeEventListener): () => void {
  if (!window.AwesomeEvent) {
    throw new Error('AwesomeEvent is not initialized. Make sure tauri-awesome-rpc is properly configured.');
  }
  
  return window.AwesomeEvent.once(eventName, callback);
}

/**
 * Check if AwesomeEvent is available
 * @returns true if AwesomeEvent is initialized
 */
export function isAvailable(): boolean {
  return typeof window !== 'undefined' && window.AwesomeEvent !== undefined;
}

export type { AwesomeEventListener, AwesomeEventAPI };
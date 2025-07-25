/**
 * Tauri Awesome RPC Guest JS API
 *
 * Provides TypeScript bindings for the AwesomeListener system
 */

interface AwesomeEventListener {
	(data: unknown): void;
}

// Type for unlisten function to maintain compatibility
export type UnlistenFn = () => void;

interface AwesomeListenerAPI {
	/**
	 * Listen to events emitted from the Rust backend
	 * @param eventName The name of the event to listen for
	 * @param callback Function to call when the event is received
	 * @returns A function to unsubscribe from the event
	 */
	listen(eventName: string, callback: AwesomeEventListener): UnlistenFn;

	/**
	 * Listen to an event once. The listener is automatically removed after the first event
	 * @param eventName The name of the event to listen for
	 * @param callback Function to call when the event is received
	 * @returns A function to unsubscribe from the event
	 */
	once(eventName: string, callback: AwesomeEventListener): UnlistenFn;
}

declare global {
	interface Window {
		AwesomeListener: AwesomeListenerAPI;
	}
}

/**
 * Listen to an event emitted from the Rust backend
 * @param eventName The name of the event to listen for
 * @param callback Function to call when the event is received
 * @returns A function to unsubscribe from the event
 */
export function listen(
	eventName: string,
	callback: AwesomeEventListener,
): () => void {
	if (!window.AwesomeListener) {
		throw new Error(
			"AwesomeListener is not initialized. Make sure tauri-awesome-rpc is properly configured.",
		);
	}

	return window.AwesomeListener.listen(eventName, callback);
}

/**
 * Listen to an event once. The listener is automatically removed after the first event
 * @param eventName The name of the event to listen for
 * @param callback Function to call when the event is received
 * @returns A function to unsubscribe from the event
 */
export function once(
	eventName: string,
	callback: AwesomeEventListener,
): () => void {
	if (!window.AwesomeListener) {
		throw new Error(
			"AwesomeListener is not initialized. Make sure tauri-awesome-rpc is properly configured.",
		);
	}

	return window.AwesomeListener.once(eventName, callback);
}

/**
 * Check if AwesomeListener is available
 * @returns true if AwesomeListener is initialized
 */
export function isAvailable(): boolean {
	return typeof window !== "undefined" && window.AwesomeListener !== undefined;
}

export type { AwesomeEventListener, AwesomeListenerAPI };


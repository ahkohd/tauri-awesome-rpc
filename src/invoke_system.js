function processIpcMessage(message) {
	if (
		message instanceof ArrayBuffer ||
		ArrayBuffer.isView(message) ||
		Array.isArray(message)
	) {
		return {
			contentType: "application/octet-stream",
			data: message,
		};
	} else {
		const data = JSON.stringify(message, (_k, val) => {
			if (val instanceof Map) {
				let o = {};
				val.forEach((v, k) => (o[k] = v));
				return o;
			} else if (val instanceof Uint8Array) {
				return Array.from(val);
			} else if (val instanceof ArrayBuffer) {
				return Array.from(new Uint8Array(val));
			} else if (
				val instanceof Object &&
				"__TAURI_CHANNEL_MARKER__" in val &&
				typeof val.id === "number"
			) {
				return `__CHANNEL__:${val.id}`;
			} else {
				return val;
			}
		});

		return {
			contentType: "application/json",
			data,
		};
	}
}

const port = ${AWESOME_RPC_PORT};

// WebSocket Manager for single persistent connection
class WebSocketManager {
	constructor(port) {
		this.port = port;
		this.ws = null;
		this.state = 'disconnected'; // disconnected, connecting, connected
		this.pendingRequests = new Map(); // requestId -> {callback, error}
		this.eventListeners = new Map(); // eventName -> Set of {callback, once}
		this.messageQueue = [];
		this.reconnectTimer = null;
		this.reconnectDelay = 1000;
		this.maxReconnectDelay = 30000;
		this.requestCounter = 0;
		
		this.connect();
	}

	connect() {
		if (this.state === 'connecting' || this.state === 'connected') {
			return;
		}

		this.state = 'connecting';
		this.ws = new WebSocket(`ws://localhost:${this.port}`, "json");

		this.ws.onopen = () => {
			console.log("Awesome RPC WebSocket connected");
			this.state = 'connected';
			this.reconnectDelay = 1000; // Reset delay on successful connection
			
			// Send queued messages
			while (this.messageQueue.length > 0) {
				const msg = this.messageQueue.shift();
				this.ws.send(msg);
			}
			
			// Re-subscribe to events
			for (const [eventName, listeners] of this.eventListeners) {
				if (listeners.size > 0) {
					this.sendEventSubscribe(eventName);
				}
			}
		};

		this.ws.onmessage = (event) => {
			try {
				const message = JSON.parse(event.data);
				
				// Handle event messages
				if (message.event_name) {
					const currentWindow = window.__TAURI_INTERNALS__.metadata?.currentWindow ||
										 window.__TAURI_INTERNALS__.metadata?.currentWebview ||
										 window.__TAURI_INTERNALS__?.currentWindow ||
										 { label: 'main' };

					if ([null, currentWindow.label || 'main'].includes(message.window_label)) {
						const listeners = this.eventListeners.get(message.event_name);
						if (listeners) {
							const listenersToRemove = [];
							
							listeners.forEach(listener => {
								listener.callback(message.payload);
								if (listener.once) {
									listenersToRemove.push(listener);
								}
							});
							
							// Remove 'once' listeners
							listenersToRemove.forEach(listener => {
								listeners.delete(listener);
							});
							
							// If no more listeners, unsubscribe
							if (listeners.size === 0) {
								this.eventListeners.delete(message.event_name);
								this.sendEventUnsubscribe(message.event_name);
							}
						}
					}
					return;
				}
				
				// Handle RPC responses
				if (message.id && message.result) {
					const pending = this.pendingRequests.get(message.id);
					if (pending) {
						const isSuccess = message.result.status === "Success";
						const cb = isSuccess ? pending.callback : pending.error;
						const data = message.result.data;
						
						const callbacks = window.__TAURI_INTERNALS__.callbacks;
						if (callbacks && callbacks.has(cb)) {
							const callbackFn = callbacks.get(cb);
							callbackFn(data);
							callbacks.delete(cb);
						}
						
						this.pendingRequests.delete(message.id);
					}
				}
			} catch (e) {
				console.error('Error parsing WebSocket message:', e);
			}
		};

		this.ws.onerror = (error) => {
			console.error('WebSocket error:', error);
		};

		this.ws.onclose = () => {
			console.log("Awesome RPC WebSocket disconnected");
			this.state = 'disconnected';
			this.ws = null;
			
			// Reject all pending requests
			for (const [requestId, pending] of this.pendingRequests) {
				const callbacks = window.__TAURI_INTERNALS__.callbacks;
				if (callbacks && callbacks.has(pending.error)) {
					const errorFn = callbacks.get(pending.error);
					errorFn({
						message: "WebSocket connection lost",
						error: "Connection closed"
					});
					callbacks.delete(pending.error);
				}
			}
			this.pendingRequests.clear();
			
			// Schedule reconnection
			this.scheduleReconnect();
		};
	}

	scheduleReconnect() {
		if (this.reconnectTimer) {
			clearTimeout(this.reconnectTimer);
		}
		
		this.reconnectTimer = setTimeout(() => {
			console.log(`Attempting to reconnect WebSocket (delay: ${this.reconnectDelay}ms)`);
			this.connect();
		}, this.reconnectDelay);
		
		// Exponential backoff
		this.reconnectDelay = Math.min(this.reconnectDelay * 2, this.maxReconnectDelay);
	}

	generateRequestId() {
		return `${Date.now()}_${++this.requestCounter}`;
	}

	send(message) {
		const msgStr = JSON.stringify(message);
		
		if (this.state === 'connected' && this.ws && this.ws.readyState === WebSocket.OPEN) {
			this.ws.send(msgStr);
		} else {
			this.messageQueue.push(msgStr);
			if (this.state === 'disconnected') {
				this.connect();
			}
		}
	}

	sendInvoke(cmd, callback, error, payload, windowLabel) {
		const requestId = this.generateRequestId();
		this.pendingRequests.set(requestId, { callback, error });
		
		const rpcPayload = {
			cmd,
			callback,
			error,
			payload,
			invoke_key: __INVOKE_KEY__
		};

		const message = {
			jsonrpc: "2.0",
			id: requestId,
			method: "invoke",
			params: {
				window_label: windowLabel,
				payload: JSON.stringify(rpcPayload)
			}
		};

		this.send(message);
	}

	sendEventSubscribe(eventName) {
		const message = {
			jsonrpc: "2.0",
			method: "event_subscribe",
			params: {
				event_name: eventName
			}
		};
		this.send(message);
	}

	sendEventUnsubscribe(eventName) {
		const message = {
			jsonrpc: "2.0",
			method: "event_unsubscribe",
			params: {
				event_name: eventName
			}
		};
		this.send(message);
	}

	addEventListener(eventName, callback, once = false) {
		if (!this.eventListeners.has(eventName)) {
			this.eventListeners.set(eventName, new Set());
			this.sendEventSubscribe(eventName);
		}
		
		const listener = { callback, once };
		this.eventListeners.get(eventName).add(listener);
		
		// Return unsubscribe function
		return () => {
			const listeners = this.eventListeners.get(eventName);
			if (listeners) {
				listeners.delete(listener);
				if (listeners.size === 0) {
					this.eventListeners.delete(eventName);
					this.sendEventUnsubscribe(eventName);
				}
			}
		};
	}
}

// Create singleton WebSocket manager
const wsManager = new WebSocketManager(port);

(function () {
	function sendIpcMessage(message) {
		const { cmd, callback, error, payload, options } = message;

		const { contentType, data } = processIpcMessage(payload);
		const windowLabel = window.__TAURI_INTERNALS__.metadata.currentWindow.label;
		const processedPayload = typeof data === 'string' ? JSON.parse(data) : data;
		
		wsManager.sendInvoke(cmd, callback, error, processedPayload, windowLabel);
	}

	Object.defineProperty(window.__TAURI_INTERNALS__, "postMessage", {
		value: sendIpcMessage,
	});

	// Event system - AwesomeEvent
	Object.defineProperty(window, 'AwesomeEvent', {
		value: {
			listen: (event_name, callback) => {
				console.log("Setting up AwesomeEvent listener for:", event_name);
				return wsManager.addEventListener(event_name, callback, false);
			},
			once: (event_name, callback) => {
				console.log("Setting up AwesomeEvent once listener for:", event_name);
				return wsManager.addEventListener(event_name, callback, true);
			}
		}
	});
})();
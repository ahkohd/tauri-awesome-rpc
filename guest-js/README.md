# @tauri-awesome-rpc/api

TypeScript API for tauri-awesome-rpc event system.

## Installation

```bash
npm install @tauri-awesome-rpc/api
```

## Usage

```typescript
import { listen, isAvailable } from '@tauri-awesome-rpc/api';

// Check if AwesomeEvent is available
if (isAvailable()) {
  // Listen to events from the Rust backend
  const unlisten = listen('my-event', (data) => {
    console.log('Received:', data);
  });
  
  // Stop listening
  unlisten();
}
```

## API

### `listen(eventName: string, callback: (data: any) => void): () => void`

Listen to events emitted from the Rust backend using `AwesomeEmit`.

- `eventName`: The name of the event to listen for
- `callback`: Function to call when the event is received
- Returns: A function to unsubscribe from the event

### `isAvailable(): boolean`

Check if AwesomeEvent is available in the window object.

## Type Definitions

The module includes TypeScript definitions for the AwesomeEvent API, which are automatically included when you import the module.
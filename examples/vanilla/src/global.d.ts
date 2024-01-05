interface Window {
  AwesomeEvent: {
    listen(eventName: string, callback: (data: unknown) => void): () => void;
  };
}

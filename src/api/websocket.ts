export interface WebSocketMessage {
  type: string;
  payload: any;
}

export type WebSocketListener = (message: WebSocketMessage) => void;

class WebSocketClient {
  private static instance: WebSocketClient;
  private ws: WebSocket | null = null;
  private listeners: Set<WebSocketListener> = new Set();
  private reconnectInterval: number = 3000;
  private url: string = "";
  private shouldReconnect: boolean = true;

  private constructor() {}

  public static getInstance(): WebSocketClient {
    if (!WebSocketClient.instance) {
      WebSocketClient.instance = new WebSocketClient();
    }
    return WebSocketClient.instance;
  }

  public connect(url: string) {
    if (this.ws && (this.ws.readyState === WebSocket.OPEN || this.ws.readyState === WebSocket.CONNECTING)) {
      return;
    }
    this.url = url;
    this.shouldReconnect = true;

    try {
      this.ws = new WebSocket(url);
      
      this.ws.onopen = () => {
        console.log("WebSocket connected to", url);
        this.emit({ type: "CONNECTION_OPEN", payload: {} });
      };

      this.ws.onmessage = (event) => {
        try {
          const message: WebSocketMessage = JSON.parse(event.data);
          this.emit(message);
        } catch (e) {
          console.error("Failed to parse WebSocket message", e);
        }
      };

      this.ws.onclose = () => {
        console.log("WebSocket disconnected");
        this.emit({ type: "CONNECTION_CLOSE", payload: {} });
        if (this.shouldReconnect) {
          setTimeout(() => this.connect(this.url), this.reconnectInterval);
        }
      };

      this.ws.onerror = (error) => {
        console.error("WebSocket error", error);
        this.emit({ type: "CONNECTION_ERROR", payload: error });
      };
    } catch (e) {
      console.error("Failed to create WebSocket", e);
      if (this.shouldReconnect) {
        setTimeout(() => this.connect(this.url), this.reconnectInterval);
      }
    }
  }

  public disconnect() {
    this.shouldReconnect = false;
    if (this.ws) {
      this.ws.close();
      this.ws = null;
    }
  }

  public send(type: string, payload: any) {
    if (this.ws && this.ws.readyState === WebSocket.OPEN) {
      this.ws.send(JSON.stringify({ type, payload }));
    } else {
      console.warn("WebSocket is not open, cannot send message", type);
    }
  }

  public addListener(listener: WebSocketListener) {
    this.listeners.add(listener);
    return () => this.removeListener(listener);
  }

  public removeListener(listener: WebSocketListener) {
    this.listeners.delete(listener);
  }

  private emit(message: WebSocketMessage) {
    this.listeners.forEach((listener) => listener(message));
  }
}

export const wsClient = WebSocketClient.getInstance();

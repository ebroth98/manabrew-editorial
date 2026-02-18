import { wsClient } from "./websocket";

// Simulate server events for development
class MiddlewareMock {
  private static instance: MiddlewareMock;
  private interval: any;

  private constructor() {}

  public static getInstance(): MiddlewareMock {
    if (!MiddlewareMock.instance) {
      MiddlewareMock.instance = new MiddlewareMock();
    }
    return MiddlewareMock.instance;
  }

  public start() {
    console.log("Starting Middleware Mock...");
    
    // Simulate connection
    setTimeout(() => {
      wsClient.connect("ws://mock-server");
      // Manually trigger open since we aren't really connecting to a server in this mock if we wanted to fully intercept, 
      // but wsClient expects a real WS. 
      // For now, let's assume we use this alongside a real WS or just for testing.
      // If we want to mock the WS itself, we'd need to mock the WebSocket constructor.
    }, 1000);

    // Simulate random heartbeats or status updates
    this.interval = setInterval(() => {
      // wsClient.emit({ type: "HEARTBEAT", payload: { timestamp: Date.now() } });
    }, 5000);
  }

  public stop() {
    if (this.interval) clearInterval(this.interval);
  }
}

export const middlewareMock = MiddlewareMock.getInstance();

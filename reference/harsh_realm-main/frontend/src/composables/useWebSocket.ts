import { useConnectionStore } from "../stores/connection";
import { useGameStore } from "../stores/game";
import { useAdminStore } from "../stores/admin";
import { useMapStore } from "../stores/map";
import { useTownStore } from "../stores/town";
import { useWorldStore } from "../stores/world";
import { useEncounterStore } from "../stores/encounter";
import { useLootStore } from "../stores/loot";
import { handleWebSocketMessage } from "./_websocketHandlers";
import { worldModel } from "../worldModel/instance";
import { loadCharacter, loadWorldMap } from "../worldModel/hydrate";
import { installProjection } from "../worldModel/projection";

const RECONNECT_DELAY_MS = 3000;
const WS_URL = `${location.protocol === "https:" ? "wss" : "ws"}://${location.host}/ws`;

let socket: WebSocket | null = null;
let reconnectTimer: ReturnType<typeof setTimeout> | null = null;
/** Guard: projection is installed at most once per module lifetime. */
let projectionInstalled = false;

export function useWebSocket() {
  const connectionStore = useConnectionStore();
  const gameStore = useGameStore();
  const adminStore = useAdminStore();
  const mapStore = useMapStore();
  const townStore = useTownStore();
  const worldStore = useWorldStore();
  const encounterStore = useEncounterStore();
  const lootStore = useLootStore();

  // Install the Pinia projection once so model changes flow to the stores.
  if (!projectionInstalled) {
    installProjection(worldModel, {
      gameStore,
      mapStore,
      townStore,
      encounterStore,
      lootStore,
    });
    projectionInstalled = true;
  }

  function clearReconnect() {
    if (reconnectTimer !== null) {
      clearTimeout(reconnectTimer);
      reconnectTimer = null;
    }
  }

  function connect() {
    if (
      socket &&
      (socket.readyState === WebSocket.OPEN ||
        socket.readyState === WebSocket.CONNECTING)
    ) {
      return;
    }

    clearReconnect();
    connectionStore.setStatus("connecting");

    socket = new WebSocket(WS_URL);

    socket.addEventListener("open", () => {
      connectionStore.setStatus("connected");
      gameStore.addMessage("system", "Connected to Harsh Realm.", "system");
      // Hydrate model (→ projected to stores) when connection opens.
      if (worldStore.activeWorld) {
        void loadCharacter(worldModel).finally(() => {
          gameStore.characterChecked = true;
        });
        void loadWorldMap(worldModel).then((ok) => {
          if (!ok) mapStore.loaded = false;
        });
      }
    });

    socket.addEventListener("message", (event: MessageEvent) => {
      try {
        const msg = JSON.parse(event.data as string) as Record<string, unknown>;
        handleWebSocketMessage(msg, {
          gameStore,
          adminStore,
        });
      } catch {
        console.warn("Received non-JSON WebSocket message:", event.data);
      }
    });

    socket.addEventListener("close", () => {
      if (connectionStore.status === "disconnected") return; // Manual disconnect

      connectionStore.setStatus("reconnecting");
      gameStore.addMessage(
        "system",
        "Connection lost. Reconnecting…",
        "system",
      );
      reconnectTimer = setTimeout(connect, RECONNECT_DELAY_MS);
    });

    socket.addEventListener("error", () => {
      socket?.close();
    });
  }

  function send(text: string) {
    if (socket && socket.readyState === WebSocket.OPEN) {
      // Add player input to chat immediately (optimistic)
      gameStore.addMessage("you", text, "player_input");
      socket.send(JSON.stringify({ type: "command", text }));
    } else {
      gameStore.addMessage(
        "system",
        "Not connected — command not sent.",
        "system",
      );
    }
  }

  function disconnect() {
    clearReconnect();
    const oldSocket = socket;
    socket = null;
    if (oldSocket) {
      oldSocket.close();
    }
    connectionStore.setStatus("disconnected");
  }

  return { connect, send, disconnect };
}

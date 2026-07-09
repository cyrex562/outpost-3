import { defineStore } from "pinia";
import { ref } from "vue";

export type WsStatus = "connecting" | "connected" | "disconnected" | "reconnecting";

export const useConnectionStore = defineStore("connection", () => {
  const status = ref<WsStatus>("disconnected");

  function setStatus(s: WsStatus) {
    status.value = s;
  }

  return { status, setStatus };
});

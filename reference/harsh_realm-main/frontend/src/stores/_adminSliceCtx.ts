// Shared dependencies passed from the admin store into each domain slice.
import type { Ref } from "vue";

export interface AdminSliceCtx {
  worldParam: () => string;
  activeWorldPath: Ref<string>;
  loading: Ref<boolean>;
  error: Ref<string | null>;
}

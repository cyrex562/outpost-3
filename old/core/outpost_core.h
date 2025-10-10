#include <stdarg.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdlib.h>

/**
 * Opaque handle to GameState
 */
typedef struct GameStateHandle {
  uint8_t _private[0];
} GameStateHandle;

/**
 * Create new game state
 */
struct GameStateHandle *game_state_new(void);

/**
 * Free game state
 */
void game_state_free(struct GameStateHandle *handle);

/**
 * Apply command (returns JSON events)
 * Caller must free returned string with game_string_free
 */
char *game_state_apply_command(struct GameStateHandle *handle, const char *command_json);

/**
 * Get state as JSON
 */
char *game_state_to_json(const struct GameStateHandle *handle);

/**
 * Free string returned by Rust
 */
void game_string_free(char *s);

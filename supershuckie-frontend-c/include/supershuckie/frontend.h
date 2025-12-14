#ifndef __SUPERSHUCKIE_FRONTEND_H_
#define __SUPERSHUCKIE_FRONTEND_H_

#ifdef __cplusplus
extern "C" {
#endif

#include <stdlib.h>
#include <stdint.h>
#include <stdbool.h>

/**
 * Represents an opaque SuperShuckie frontend created with supershuckie_frontend_new() and freed with supershuckie_frontend_free().
 *
 * EXCEPT for supershuckie_frontend_free, no functions that take a pointer to a frontend accept a null SuperShuckieFrontendRaw pointer.
 */
struct SuperShuckieFrontendRaw;

struct SuperShuckieInput {
    bool a;
    bool b;
    bool start;
    bool select;

    bool d_up;
    bool d_down;
    bool d_left;
    bool d_right;

    bool l;
    bool r;
    bool x;
    bool y;

    // If touch_x and touch_y are not 0xFFFF, simulate a touch button input
    uint16_t touch_x;
    uint16_t touch_y;
};

struct SuperShuckieScreenData {
    uint32_t width;
    uint32_t height;
    uint32_t encoding;
};

typedef void (*SuperShuckieRefreshScreensCallback)(void *user_data, size_t screen_count, const uint32_t *const *pixels);
typedef void (*SuperShuckieNewCoreMetadataCallback)(void *user_data, size_t screen_count, const struct SuperShuckieScreenData *screen_data);

struct SuperShuckieFrontendCallbacks {
    void *user_data;

    SuperShuckieRefreshScreensCallback refresh_screens;
    SuperShuckieNewCoreMetadataCallback new_core_metadata;
};

/**
 * Initialize a new frontend.
 *
 * Safety:
 * - Both pointers must point to valid data.
 */
struct SuperShuckieFrontendRaw *supershuckie_frontend_new(
    const char *user_data_path,
    const struct SuperShuckieFrontendCallbacks *callbacks
);

/**
 * Set whether or not the frontend is paused.
 */
void supershuckie_frontend_set_paused(struct SuperShuckieFrontendRaw *frontend, bool paused);

/**
 * Manually invoke the refresh screens callback even if no updates have occurred.
 */
void supershuckie_frontend_force_refresh_screens(struct SuperShuckieFrontendRaw *frontend);

/**
 * Load the given ROM, returning true or false depending on whether or not it was successfully loaded.
 *
 * Safety:
 * - path must be null-terminated, UTF-8
 * - error must point to a buffer of at least `error_len` bytes (it can be null if error_len is 0)
 */
bool supershuckie_frontend_load_rom(struct SuperShuckieFrontendRaw *frontend, const char *path, char *error, size_t error_len);

/**
 * If there is a ROM running, return the name. Otherwise, return null.
 */
const char *supershuckie_frontend_get_rom_name(const struct SuperShuckieFrontendRaw *frontend);

/**
 * Return true if there is currently a game running.
 */
bool supershuckie_frontend_is_game_running(const struct SuperShuckieFrontendRaw *frontend);

/**
 * Unload the current ROM, if any.
 */
void supershuckie_frontend_unload_rom(struct SuperShuckieFrontendRaw *frontend);

/**
 * Should be called regularly.
 */
void supershuckie_frontend_tick(struct SuperShuckieFrontendRaw *frontend);

/**
 * Free the core
 *
 * Safety:
 * - frontend must either be created with supershuckie_frontend_new OR it can be null
 * - frontend, if non-null, may only be freed once
 */
void supershuckie_frontend_free(struct SuperShuckieFrontendRaw *frontend);

#ifdef __cplusplus
}
#endif

#endif

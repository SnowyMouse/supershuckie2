#ifndef __SUPERSHUCKIE_H_
#define __SUPERSHUCKIE_H_

#ifdef __cplusplus
extern "C" {
#endif

#include <stdlib.h>
#include <stdint.h>
#include <stdbool.h>

/**
 * Represents an opaque SuperShuckie core.
 *
 * To free, use supershuckie_core_free()
 */
struct SuperShuckieCoreRaw;

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

enum GameBoyType {
    GameBoyType__GameBoy,
    GameBoyType__GameBoyColor
};

/**
 * Instantiates a new Game Boy emulator instance.
 *
 * Safety:
 * - rom and bios must be non-null and valid for at least rom_size and bios_size bytes, respectively.
 * - type must correspond to a valid GameBoyType enum value
 */
struct SuperShuckieCoreRaw *supershuckie_core_new_gameboy(
    const void *rom,
    size_t rom_size,
    const void *bios,
    size_t bios_size,
    GameBoyType type
);

/**
 * Instantiates a null core that does not actually emulate anything.
 *
 * It is useful as a placeholder, and it provides a single empty screen.
 */
struct SuperShuckieCoreRaw *supershuckie_core_new_null(void);

/**
 * Gets the frame counter.
 *
 * This can be used as a cheap way to check if the frame has changed.
 */
uint32_t supershuckie_core_get_frame_count(const struct SuperShuckieCoreRaw *core);

/**
 * Get the number of screens.
 *
 * Note that this count is guaranteed to never change resolution for the duration of the core's existence.
 *
 * It is also guaranteed to be at least 1.
 */
size_t supershuckie_core_get_screen_count(const struct SuperShuckieCoreRaw *core);

/**
 * Start if paused.
 *
 * Note: The default state of a core is paused.
 */
void supershuckie_core_start(struct SuperShuckieCoreRaw *core);

/**
 * Pause if unpaused.
 *
 * Note: The default state of a core is paused.
 */
void supershuckie_core_pause(struct SuperShuckieCoreRaw *core);

/**
 * Enqueue an input.
 */
void supershuckie_core_enqueue_input(struct SuperShuckieCoreRaw *core, const struct SuperShuckieInput *input);

/**
 * Get the screen resolution.
 *
 * Return false if the screen does not exist.
 *
 * Note that this screen is guaranteed to never change resolution for the duration of the core's existence.
 *
 * Safety:
 * - all pointers passed in must be non-null
 */
bool supershuckie_core_get_screen_resolution(const struct SuperShuckieCoreRaw *core, size_t screen_index, size_t *width, size_t *height);

/**
 * Copy the screen data, returning the number of pixels the screen takes up, or 0 if the screen does not exist.
 *
 * Safety:
 * - all pointers passed in must be non-null (pixels can be null only if pixel_count is 0)
 * - pixels must contain at least `pixel_count` uint32_t elements
 */
size_t supershuckie_core_copy_screen_data(const struct SuperShuckieCoreRaw *core, size_t screen_index, uint32_t *pixels, size_t pixel_count);

/**
 * Free the core
 *
 * Safety:
 * - core must be created with a supershuckie_* function OR null
 * - core, if non-null, may only be freed once
 */
void supershuckie_core_free(struct SuperShuckieCoreRaw *core);

#ifdef __cplusplus
}
#endif

#endif

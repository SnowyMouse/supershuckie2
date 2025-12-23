#ifndef __SUPERSHUCKIE_STRING_ARRAY_H_
#define __SUPERSHUCKIE_STRING_ARRAY_H_

#ifdef __cplusplus
extern "C" {
#endif

#include <stdlib.h>

/**
 * A string array holding zero or more null-terminated UTF8 strings.
 *
 * This can never be null EXCEPT in the supershuckie_stringarray_free function (which this array must be freed in if
 * retrieved from SuperShuckie).
 */
struct SuperShuckieStringArrayRaw;

/**
 * Get the length of a string array.
 */
size_t supershuckie_stringarray_len(const struct SuperShuckieStringArrayRaw *array);

/**
 * Get the element at the given position in the array, or null if out-of-bounds.
 */
const char *supershuckie_stringarray_get(const struct SuperShuckieStringArrayRaw *array, size_t position);

/**
 * Free the string array.
 *
 * Safety:
 * - A pointer may only be freed once (unless the pointer is null)
 */
void supershuckie_stringarray_free(struct SuperShuckieStringArrayRaw *array);

#ifdef __cplusplus
}
#endif

#endif

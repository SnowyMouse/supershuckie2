// shared memory in Windows

#include <stdint.h>
#include <windows.h>

static HANDLE handle = INVALID_HANDLE_VALUE;
static const char *mmf_name = "EDPS_MemoryData.bin";

uint8_t *supershuckie_pokeabyte_try_create_shared_memory(size_t len, const char **error) {
    if(handle != INVALID_HANDLE_VALUE) {
        if(error) {
            *error = "shared memory already created";
        }
        return NULL;
    }

    HANDLE handle_maybe = CreateFileMappingA(
        INVALID_HANDLE_VALUE,
        NULL,
        PAGE_READWRITE,
        (uint32_t)((uint64_t)(len) >> 32),
        (uint32_t)len,
        mmf_name
    );

    if(handle_maybe == INVALID_HANDLE_VALUE) {
        if(error) {
            *error = "CreateFileMappingA failed";
        }

        return NULL;
    }

    handle = handle_maybe;

    return MapViewOfFile(
        handle,
        FILE_MAP_ALL_ACCESS,
        0,
        0,
        len
    );
}

void supershuckie_pokeabyte_close_shared_memory(void) {
    if(handle == INVALID_HANDLE_VALUE) {
        CloseHandle(handle);
        handle = INVALID_HANDLE_VALUE;
    }
}

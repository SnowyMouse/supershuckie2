#ifndef __SUPERSHUCKIE_HPP_
#define __SUPERSHUCKIE_HPP_

#include <memory>
#include <span>
#include <cstdint>

#include "supershuckie.h"

struct SuperShuckieScreenData {
    std::size_t width;
    std::size_t height;
    std::vector<std::uint32_t> pixels;
};

class SuperShuckieCore {
public:
    /**
     * Instantiate from an existing SuperShuckieCoreRaw type
     *
     * The core will take ownership over it and free it with supershuckie_core_free.
     *
     * Note: `raw` must not be null!
     */
    SuperShuckieCore(SuperShuckieCoreRaw *raw): raw(raw, supershuckie_core_free) {
        if(raw == nullptr) {
            std::terminate();
        }

        size_t screen_count = supershuckie_core_get_screen_count(raw);
        this->screens.reserve(screen_count);

        for(size_t s = 0; s < screen_count; s++) {
            size_t width;
            size_t height;
            bool result = supershuckie_core_get_screen_resolution(raw, s, &width, &height);
            if(result == false) {
                std::terminate();
            }

            SuperShuckieScreenData data;
            data.width = width;
            data.height = height;
            data.pixels.resize(width * height, 0x00000000);

            this->screens.emplace_back(data);
        }

        this->refresh_screens(true);
    }

    /**
     * Instantiate a null core.
     */
    static SuperShuckieCore new_null() noexcept {
        return SuperShuckieCore(supershuckie_core_new_null());
    }

    /**
     * Instantiate a new Game Boy core with the given ROM, BIOS, and type.
     */
    static SuperShuckieCore new_from_gameboy(std::span<const std::byte> rom, std::span<const std::byte> bios, GameBoyType type) noexcept {
        return SuperShuckieCore::new_from_gameboy(
            rom.data(),
            rom.size(),
            bios.data(),
            bios.size(),
            type
        );
    }

    /**
     * Instantiate a new Game Boy core with the given ROM, BIOS, and type.
     */
    static SuperShuckieCore new_from_gameboy(const void *rom, std::size_t rom_size, const void *bios, std::size_t bios_size, GameBoyType type) noexcept {
        SuperShuckieCoreRaw *raw = supershuckie_core_new_gameboy(
            rom,
            rom_size,
            bios,
            bios_size,
            type
        );
        return SuperShuckieCore(raw);
    }

    const std::vector<SuperShuckieScreenData> &get_screens(bool &updated) noexcept {
        updated = this->refresh_screens(false);
        return this->screens;
    }

    std::uint32_t get_frame_count() noexcept {
        return supershuckie_core_get_frame_count(this->raw.get());
    }

    void start() noexcept {
        supershuckie_core_start(this->raw.get());
    }

    void pause() noexcept {
        supershuckie_core_pause(this->raw.get());
    }

    void enqueue_input(const SuperShuckieInput &input) noexcept {
        supershuckie_core_enqueue_input(this->raw.get(), &input);
    }

private:
    std::unique_ptr<SuperShuckieCoreRaw, decltype(&supershuckie_core_free)> raw;

    std::vector<SuperShuckieScreenData> screens;
    std::uint32_t frame_count = 0;

    bool refresh_screens(bool force) {
        std::uint32_t new_frame_count = this->get_frame_count();
        if(!force && new_frame_count == this->frame_count) {
            return false;
        }

        this->frame_count = new_frame_count;

        size_t screen_count = this->screens.size();
        for(size_t i = 0; i < screen_count; i++) {
            auto &screen = this->screens[i];
            auto pixel_count = supershuckie_core_copy_screen_data(this->raw.get(), i, screen.pixels.data(), screen.pixels.size());
            if(pixel_count != screen.pixels.size()) {
                std::terminate();
            }
        }

        return true;
    }

};



#endif

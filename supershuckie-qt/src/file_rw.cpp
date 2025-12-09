#include <cstdio>

#include "error.hpp"
#include "file_rw.hpp"

template<typename T> static std::optional<std::vector<T>> read_file(const std::filesystem::path &path) {
    static_assert(sizeof(T) == sizeof(std::byte));

    auto path_cstr = path.string();
    auto *f = std::fopen(path_cstr.c_str(), "rb");

    if(!f) {
        DISPLAY_ERROR_DIALOG("Can't open file", "Can't open '%s' for reading!", path_cstr.c_str());
        return std::nullopt;
    }

    std::fseek(f, 0, SEEK_END);
    auto len = static_cast<std::size_t>(std::ftell(f));
    std::fseek(f, 0, SEEK_SET);

    std::vector<T> final;
    final.resize(len);
    std::fread(final.data(), len, 1, f);
    std::fclose(f);

    return final;
}

static bool write_file(const std::filesystem::path &path, const std::byte *buffer_data, std::size_t buffer_size) {
    auto path_cstr = path.string();
    auto *f = std::fopen(path_cstr.c_str(), "wb");

    if(!f) {
        DISPLAY_ERROR_DIALOG("Can't open file", "Can't open '%s' for writing!", path_cstr.c_str());
        return false;
    }

    int result = std::fwrite(buffer_data, buffer_size, 1, f);
    std::fclose(f);

    if(result != 1) {
        DISPLAY_ERROR_DIALOG("Can't write file", "Failed to write %zu to '%s'!", buffer_size, path_cstr.c_str());
    }

    return result == 1;
}

std::optional<std::vector<std::byte>> SuperShuckie64::read_file(const std::filesystem::path &path) {
    return ::read_file<std::byte>(path);
}

std::optional<std::vector<std::uint8_t>> SuperShuckie64::read_file_u8(const std::filesystem::path &path) {
    return ::read_file<std::uint8_t>(path);
}

bool SuperShuckie64::write_file(const std::filesystem::path &path, const std::vector<std::byte> &buffer) {
    return ::write_file(path, buffer.data(), buffer.size());
}

bool SuperShuckie64::write_file(const std::filesystem::path &path, const std::vector<std::uint8_t> &buffer) {
    return ::write_file(path, reinterpret_cast<const std::byte *>(buffer.data()), buffer.size());
}

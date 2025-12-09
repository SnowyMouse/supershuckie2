#ifndef SS64_FILE_RW_HPP
#define SS64_FILE_RW_HPP

#include <filesystem>
#include <vector>

namespace SuperShuckie64 {
    std::optional<std::vector<std::byte>> read_file(const std::filesystem::path &path);
    std::optional<std::vector<std::uint8_t>> read_file_u8(const std::filesystem::path &path);
    bool write_file(const std::filesystem::path &path, const std::vector<std::byte> &buffer);
    bool write_file(const std::filesystem::path &path, const std::vector<std::uint8_t> &buffer);
}

#endif

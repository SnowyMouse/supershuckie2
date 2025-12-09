import sys

i = sys.argv[1]
o = sys.argv[2]

with open(i, "rb") as f:
    ib = f.read()

headerguard = "AUTOGEN_" + o.replace(".", "_").upper()

bytes_encoded = ""
for b in ib:
    if bytes_encoded == "":
        bytes_encoded = "0x{:02X}".format(b)
    else:
        bytes_encoded += ", 0x{:02X}".format(b)

with open(o, "w") as f:
    f.write("// auto-generated\n\n#ifndef {guard}\n#include <cstdint>\n#define {guard}\nstatic const std::uint8_t {guard}_VAL[] = {{ {bytes_encoded} }};\n#endif\n".format(guard = headerguard, bytes_encoded = bytes_encoded))
    pass

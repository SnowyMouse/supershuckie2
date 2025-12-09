all: defaultrom.h

clean:
	rm -f defaultrom.o defaultrom.hpp defaultrom.gbc

defaultrom.h:
	rgbasm defaultrom.asm -h -o defaultrom.o
	rgblink defaultrom.o -o defaultrom.gbc
	rgbfix defaultrom.gbc -v -m MBC5 -p 0xFF -t "DEFAULTROM"
	python3 to_header.py defaultrom.gbc defaultrom.hpp

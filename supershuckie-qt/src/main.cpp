#include <stdio.h>

#include <SDL3/sdl.h>
#include <QApplication>

#include "main_window.hpp"

int main(int argc, char **argv) {
    SDL_Init(SDL_INIT_EVENTS | SDL_INIT_GAMEPAD | SDL_INIT_VIDEO);

    QCoreApplication::setOrganizationName("SnowyMouse");
    QCoreApplication::setApplicationName("SuperShuckie");

    QApplication app(argc, argv);

    SuperShuckie64::SuperShuckieMainWindow window;
    window.show();

    if(argc == 2) {
        window.load_rom(argv[1]);
    }

    int result = app.exec();
    SDL_Quit();

    return result;
}
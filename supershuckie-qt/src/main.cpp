#include <stdio.h>

#include <SDL3/sdl.h>
#include <QApplication>

#include <supershuckie/supershuckie.hpp>

#include "main_window.hpp"

int main(int argc, char **argv) {
    SDL_Init(SDL_INIT_EVENTS | SDL_INIT_GAMEPAD | SDL_INIT_VIDEO);

    QCoreApplication::setOrganizationName("SnowyMouse");
    QCoreApplication::setApplicationName("SuperShuckie");

    QApplication app(argc, argv);

    SuperShuckieMainWindow window;
    window.show();

    int result = app.exec();
    SDL_Quit();

    return result;
}
#ifndef __SUPERSHUCKIE_SDL_EVENT_WRAPPER_HPP__
#define __SUPERSHUCKIE_SDL_EVENT_WRAPPER_HPP__

#include <supershuckie/supershuckie.h>
#include <unordered_map>
#include <string>
#include <vector>
#include <cstdint>

namespace SuperShuckie64 {

enum SDLEventWrapperAction {
    SDLEventWrapper_NoOp,
    SDLEventWrapper_Quit,

    SDLEventWrapper_Button,
    SDLEventWrapper_Axis,
};

struct ConnectedController {
    SuperShuckieConnectedControllerIndex mapping;
    std::string name;
};

struct SDLEventWrapperResult {
    SDLEventWrapperAction discriminator;
    union {
        struct {} no_op;
        struct {} quit;
        struct {
            ConnectedController *controller;
            std::int32_t button;
            bool pressed;
        } button;
        struct {
            ConnectedController *controller;
            std::int32_t axis;
            double value;
        } axis;
    };
};

class MainWindow;

class SDLEventWrapper {
    friend MainWindow;
public:
    SDLEventWrapper();
    SDLEventWrapperResult next();
private:
    SuperShuckieFrontendRaw *frontend = nullptr;
    std::unordered_map<std::uint32_t, ConnectedController> connected_controllers;

    std::vector<std::string> events_to_print;
};

}

#endif

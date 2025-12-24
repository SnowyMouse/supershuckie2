#include "sdl_event_wrapper.hpp"

#include <SDL3/SDL.h>

using namespace SuperShuckie64;

SDLEventWrapper::SDLEventWrapper() {

}

SDLEventWrapperResult SDLEventWrapper::next() {
    SDLEventWrapperResult result = {};
    char msg[256];

    SDL_Event event;
    while(SDL_PollEvent(&event)) {
        switch(event.type) {
            // If we hit ctrl-c, close the window (saves)
            case SDL_EventType::SDL_EVENT_QUIT:
                result.discriminator = SDLEventWrapperAction::SDLEventWrapper_Quit;
                return result;
            
            case SDL_EventType::SDL_EVENT_GAMEPAD_ADDED: {
                auto id = event.gdevice.which;
                auto *gamepad = SDL_OpenGamepad(id);
                if(SDL_OpenGamepad(id) == nullptr) {
                    break;
                }
                auto *name = SDL_GetGamepadName(gamepad);
                auto mapping = supershuckie_frontend_connect_controller(this->frontend, name);

                std::snprintf(msg, sizeof(msg), "Connected controller \"%s\"", name);
                this->events_to_print.emplace_back(msg);

                ConnectedController controller;
                controller.name = std::move(name);
                controller.mapping = mapping;
                this->connected_controllers.emplace(id, std::move(controller));
                break;
            }
            
            case SDL_EventType::SDL_EVENT_GAMEPAD_REMOVED: {
                auto id = event.gdevice.which;
                if(!this->connected_controllers.contains(id)) {
                    break;
                }

                auto &disconnected = this->connected_controllers[id];
                supershuckie_frontend_disconnect_controller(this->frontend, this->connected_controllers[id].mapping);

                std::snprintf(msg, sizeof(msg), "Disconnected controller \"%s\"", disconnected.name.c_str());
                this->events_to_print.emplace_back(msg);

                this->connected_controllers.erase(id);
                break;
            }

            case SDL_EventType::SDL_EVENT_GAMEPAD_AXIS_MOTION: {
                auto &event_data = event.gaxis;
                auto id = event_data.which;
                if(!this->connected_controllers.contains(id)) {
                    break;
                }

                auto &controller = this->connected_controllers[id];
                result.discriminator = SDLEventWrapperAction::SDLEventWrapper_Axis;
                result.axis.controller = &controller;
                result.axis.axis = event_data.axis;

                double value = event_data.value / 32767.0;
                if(value > -0.05 && value < 0.05) {
                    value = 0.0;
                }
                if(value > 1.0) {
                    value = 1.0;
                }
                if(value < -1.0) {
                    value = -1.0;
                }
                result.axis.value = value;
                return result;
            }

            case SDL_EventType::SDL_EVENT_GAMEPAD_BUTTON_UP:
            case SDL_EventType::SDL_EVENT_GAMEPAD_BUTTON_DOWN: {
                auto &event_data = event.gbutton;
                auto id = event_data.which;
                if(!this->connected_controllers.contains(id)) {
                    break;
                }
                auto &controller = this->connected_controllers[id];
                result.discriminator = SDLEventWrapperAction::SDLEventWrapper_Button;
                result.button.controller = &controller;
                result.button.button = event_data.button;
                result.button.pressed = event.type == SDL_EventType::SDL_EVENT_GAMEPAD_BUTTON_DOWN;
                return result;
            }

            default:
                // std::printf("UNHANDLED: %d\n", event.type);
                break;
        }
    }

    result.discriminator = SDLEventWrapperAction::SDLEventWrapper_NoOp;
    return result;
}
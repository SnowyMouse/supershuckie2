#include <cstdio>
#include <QLayout>
#include <SDL3/sdl.h>

#include "render_widget.hpp"
#include "main_window.hpp"

SuperShuckieMainWindow::SuperShuckieMainWindow(): QMainWindow(), core(SuperShuckieCore::new_null()) {
    // Remove rounded corners (Windows)
    #ifdef _WIN32
    DWORD one = 1;
    DwmSetWindowAttribute(reinterpret_cast<HWND>(this->winId()), 33, &one, sizeof(one));
    #endif

    this->set_title("No ROM loaded");
    this->render_widget = new SuperShuckieRenderWidget(this);
    this->setCentralWidget(this->render_widget);

    this->setWindowFlags(Qt::MSWindowsFixedSizeDialogHint);
    this->layout()->setSizeConstraint(QLayout::SetFixedSize);

    this->ticker.setInterval(1);
    this->ticker.callOnTimeout(this, &SuperShuckieMainWindow::tick);
    this->ticker.start();

    this->refresh_screen_dimensions();
    this->render_widget->refresh_screen(true);
}

void SuperShuckieMainWindow::set_title(const char *title) {
    char fmt[512];
    std::snprintf(fmt, sizeof(fmt), "Super Shuckie 2 (name TBD) - %s", title);
    this->setWindowTitle(fmt);
}

void SuperShuckieMainWindow::refresh_screen_dimensions() {
    bool updated = false;

    const auto &screens = this->core.get_screens(updated);
    const auto &first_screen = screens[0];
    this->render_widget->set_dimensions(first_screen.width,first_screen.height,this->scale);
}

void SuperShuckieMainWindow::tick() {
    SDL_Event event;
    while(SDL_PollEvent(&event)) {
        switch(event.type) {
            // If we hit ctrl-c, close the window (saves)
            case SDL_EventType::SDL_EVENT_QUIT:
                this->close();
                // If the window wasn't closed, warn
                if(this->isVisible()) {
                    std::fputs("Can't close the main window. Finish what you're doing, first!\n", stderr);
                    break;
                }
                return;
        }
    }

    this->render_widget->refresh_screen();
}
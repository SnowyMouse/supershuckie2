#ifndef __SUPERSHUCKIE_MAIN_WINDOW_HPP_
#define __SUPERSHUCKIE_MAIN_WINDOW_HPP_

#include <QMainWindow>
#include <QTimer>

#include <supershuckie/supershuckie.hpp>

class SuperShuckieRenderWidget;

class SuperShuckieMainWindow: public QMainWindow {
    friend SuperShuckieRenderWidget;
    
public:
    SuperShuckieMainWindow();
private:
    void set_title(const char *title);
    SuperShuckieRenderWidget *render_widget;

    SuperShuckieCore core;

    unsigned scale = 6;

    QTimer ticker;

    void refresh_screen_dimensions();
    void tick();
};

#endif
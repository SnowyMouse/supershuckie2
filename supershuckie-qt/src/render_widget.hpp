#ifndef __SUPERSHUCKIE_RENDER_VIEW_HPP_
#define __SUPERSHUCKIE_RENDER_VIEW_HPP_

#include <QWidget>
#include <QGraphicsView>
#include <QPixmap>

class SuperShuckieMainWindow;
class SuperShuckieGraphicsView;
class QGraphicsScene;

class SuperShuckieRenderWidget: public QGraphicsView {
    friend SuperShuckieMainWindow;
public:
    void set_dimensions(unsigned width, unsigned height, unsigned scale);

private:
    SuperShuckieRenderWidget(SuperShuckieMainWindow *parent);
    SuperShuckieMainWindow *main_window;

    unsigned nearest_scaling = 1;
    unsigned width = 1;
    unsigned height = 1;

    QPixmap pixmap;
    QGraphicsScene *scene = nullptr;
    QGraphicsPixmapItem *pixmap_item = nullptr;

    void rebuild_scene();
    void refresh_screen(bool force = false);
};

#endif
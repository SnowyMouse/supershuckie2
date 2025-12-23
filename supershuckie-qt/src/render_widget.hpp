#ifndef __SUPERSHUCKIE_RENDER_VIEW_HPP__
#define __SUPERSHUCKIE_RENDER_VIEW_HPP__

#include <QWidget>
#include <QGraphicsView>
#include <QPixmap>

class QGraphicsScene;

namespace SuperShuckie64 {

class MainWindow;
class SuperShuckieGraphicsView;

class GameRenderWidget: public QGraphicsView {
    friend MainWindow;
public:
    void set_dimensions(unsigned width, unsigned height, unsigned scale);

private:
    GameRenderWidget(MainWindow *parent);
    MainWindow *main_window;

    unsigned nearest_scaling = 1;
    unsigned width = 1;
    unsigned height = 1;

    QPixmap pixmap;
    QGraphicsScene *scene = nullptr;
    QGraphicsPixmapItem *pixmap_item = nullptr;

    void rebuild_scene();
    void force_refresh_screen();
    void refresh_screen(const uint32_t *pixels);

    void keyPressEvent(QKeyEvent *event) override;
    void keyReleaseEvent(QKeyEvent *event) override;
    
    void dragEnterEvent(QDragEnterEvent *event) override;
    void dragMoveEvent(QDragMoveEvent *event) override;
    void dropEvent(QDropEvent *event) override;
};

}

#endif
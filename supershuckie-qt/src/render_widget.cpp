#include "render_widget.hpp"
#include "main_window.hpp"
#include <QGraphicsPixmapItem>

using namespace SuperShuckie64;

SuperShuckieRenderWidget::SuperShuckieRenderWidget(SuperShuckieMainWindow *parent): QGraphicsView(parent), main_window(parent) {
    this->setFrameStyle(0);
    this->setHorizontalScrollBarPolicy(Qt::ScrollBarPolicy::ScrollBarAlwaysOff);
    this->setVerticalScrollBarPolicy(Qt::ScrollBarPolicy::ScrollBarAlwaysOff);
    this->setSizePolicy(QSizePolicy::Policy::Fixed, QSizePolicy::Policy::Fixed);
}

void SuperShuckieRenderWidget::set_dimensions(unsigned width, unsigned height, unsigned scale) {
    if(scale == 0) {
        scale = 1;
    }

    this->scale(scale, scale);
    this->width = width;
    this->height = height;
    this->setTransform(QTransform::fromScale(scale, scale));

    this->setFixedSize(this->width * scale, this->height * scale);

    this->rebuild_scene();
}

void SuperShuckieRenderWidget::rebuild_scene() {
    if(this->scene == nullptr) {
        delete this->scene;
        this->scene = nullptr;
    }

    this->pixmap = {};
    auto *new_scene = new QGraphicsScene(this);
    auto *new_pixmap = new_scene->addPixmap(this->pixmap);

    if(this->scene != nullptr) {
        delete this->pixmap_item;
        auto items = this->scene->items();
        for(auto &i : items) {
            new_scene->addItem(i);
        }
        delete this->scene;
    }

    this->pixmap_item = new_pixmap;
    this->scene = new_scene;
    this->setScene(this->scene);
}

void SuperShuckieRenderWidget::refresh_screen(bool force) {
    bool updated;
    const auto &screens = this->main_window->core.get_screens(updated);

    if(!force && !updated) {
        return;
    }

    const auto &first_screen = screens[0];
    this->pixmap.convertFromImage(QImage(reinterpret_cast<const uchar *>(first_screen.pixels.data()), first_screen.width, first_screen.height, QImage::Format::Format_ARGB32));
    this->pixmap_item->setPixmap(this->pixmap);

}
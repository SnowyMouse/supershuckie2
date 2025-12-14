#include "render_widget.hpp"
#include "main_window.hpp"
#include <QGraphicsPixmapItem>
#include <QKeyEvent>

#include <supershuckie/frontend.h>

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

void SuperShuckieRenderWidget::force_refresh_screen() {
    supershuckie_frontend_force_refresh_screens(this->main_window->frontend);
}

void SuperShuckieRenderWidget::refresh_screen(const uint32_t *pixels) {
    this->pixmap.convertFromImage(QImage(reinterpret_cast<const uchar *>(pixels), this->width, this->height, QImage::Format::Format_ARGB32));
    this->pixmap_item->setPixmap(this->pixmap);
}

void SuperShuckieRenderWidget::keyPressEvent(QKeyEvent *event) {
    QWidget::keyPressEvent(event);
    
    if(!event->isAutoRepeat() && this->main_window->frontend != nullptr) {
        supershuckie_frontend_key_press(this->main_window->frontend, event->key(), true);
    }
}

void SuperShuckieRenderWidget::keyReleaseEvent(QKeyEvent *event) {
    QWidget::keyPressEvent(event);

    if(this->main_window->frontend != nullptr) {
        supershuckie_frontend_key_press(this->main_window->frontend, event->key(), false);
    }
}
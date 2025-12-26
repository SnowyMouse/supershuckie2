#include "render_widget.hpp"
#include "main_window.hpp"
#include <QGraphicsPixmapItem>
#include <QKeyEvent>
#include <QMimeData>

#include <supershuckie/supershuckie.h>

using namespace SuperShuckie64;

GameRenderWidget::GameRenderWidget(MainWindow *window, QWidget *parent): QGraphicsView(parent), main_window(window) {
    this->setFrameStyle(0);
    this->setHorizontalScrollBarPolicy(Qt::ScrollBarPolicy::ScrollBarAlwaysOff);
    this->setVerticalScrollBarPolicy(Qt::ScrollBarPolicy::ScrollBarAlwaysOff);
    this->setSizePolicy(QSizePolicy::Policy::Fixed, QSizePolicy::Policy::Fixed);
}

void GameRenderWidget::set_dimensions(unsigned width, unsigned height, unsigned scale) {
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

void GameRenderWidget::rebuild_scene() {
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

void GameRenderWidget::force_refresh_screen() {
    supershuckie_frontend_force_refresh_screens(this->main_window->frontend);
}

void GameRenderWidget::refresh_screen(const uint32_t *pixels) {
    this->pixmap.convertFromImage(QImage(reinterpret_cast<const uchar *>(pixels), this->width, this->height, QImage::Format::Format_ARGB32));
    this->pixmap_item->setPixmap(this->pixmap);
}

void GameRenderWidget::keyPressEvent(QKeyEvent *event) {
    QWidget::keyPressEvent(event);
    
    if(this->main_window->frontend != nullptr) {
        // Replay controls
        int key = event->key();
        bool auto_repeat = event->isAutoRepeat();
        bool is_paused = supershuckie_frontend_is_paused(this->main_window->frontend);

        if(
            this->main_window->keyboard_replay_controls->isChecked() && 
            supershuckie_frontend_get_replay_playback_time(this->main_window->frontend, nullptr, nullptr)
        ) {
            switch(key) {
                case Qt::Key_Space:
                    if(!auto_repeat) {
                        supershuckie_frontend_set_paused(
                            this->main_window->frontend,
                            !supershuckie_frontend_is_paused(this->main_window->frontend)
                        );
                    }
                    return;
                case Qt::Key_Left:
                    supershuckie_frontend_advance_playback_frames(
                        this->main_window->frontend,
                        -240
                    );
                    return;
                case Qt::Key_Right:
                    supershuckie_frontend_advance_playback_frames(
                        this->main_window->frontend,
                        240
                    );
                    return;
                case Qt::Key_Up:
                case Qt::Key_Down:
                    // up/down arrow keys do not do anything yet
                    return;
                case Qt::Key_Comma:
                    if(is_paused) {
                        supershuckie_frontend_advance_playback_frames(
                            this->main_window->frontend,
                            -1
                        );
                    }
                    return;
                case Qt::Key_Period:
                    if(is_paused) {
                        supershuckie_frontend_advance_playback_frames(
                            this->main_window->frontend,
                            1
                        );
                    }
                    return;
            }
        }

        if(!auto_repeat) {
            supershuckie_frontend_key_press(this->main_window->frontend, key, true);
        }
    }
}

void GameRenderWidget::keyReleaseEvent(QKeyEvent *event) {
    QWidget::keyPressEvent(event);

    if(this->main_window->frontend != nullptr) {
        supershuckie_frontend_key_press(this->main_window->frontend, event->key(), false);
    }
}


template<typename T> static std::optional<std::filesystem::path> validate_event(T *event) {
    auto *d = event->mimeData();
    if(d->hasUrls()) {
        auto urls = d->urls();
        if(urls.length() == 1) {
            auto path = std::filesystem::path(urls[0].toLocalFile().toStdString());
            return path;
        }
    }
    return std::nullopt;
}

void GameRenderWidget::dragEnterEvent(QDragEnterEvent *event) {
    if(validate_event(event)) {
        event->accept();
    }
}

void GameRenderWidget::dragMoveEvent(QDragMoveEvent *event) {
    if(validate_event(event)) {
        event->accept();
    }
}

void GameRenderWidget::dropEvent(QDropEvent *event) {
    auto path = validate_event(event);
    if(path) {
        this->main_window->load_rom(*path);
    }
}

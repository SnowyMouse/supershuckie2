#include <QPainter>
#include <QMouseEvent>
#include <QGuiApplication>
#include <QStyleHints>

#include "replay_playback_controls.hpp"
#include "main_window.hpp"

using namespace SuperShuckie64;

#define PLAYBACK_HEIGHT 24

#define PAUSE_BUTTON_THICKNESS 4
#define BUTTON_PADDING_HORIZ 8
#define BUTTON_PADDING_VERT 4
#define BUTTON_ICON_WIDTH 12
#define BUTTON_FULL_WIDTH (BUTTON_PADDING_HORIZ*2 + BUTTON_ICON_WIDTH)

#define BAR_THICKNESS 4
#define BAR_PADDING BUTTON_PADDING_HORIZ

#define INDICATOR_RADIUS 4

ReplayPlaybackControls::ReplayPlaybackControls(MainWindow *main_window, QWidget *parent): QWidget(parent), main_window(main_window) {
    this->setFixedHeight(PLAYBACK_HEIGHT);
    this->setMinimumWidth(PLAYBACK_HEIGHT);
    this->setFocusPolicy(Qt::NoFocus);
}

void ReplayPlaybackControls::paintEvent(QPaintEvent *event) {
    auto dark_theme = QGuiApplication::styleHints()->colorScheme() == Qt::ColorScheme::Dark;

    QPainter painter(this);
    QRect fill_rectangle = QRect(0, 0, this->width(), PLAYBACK_HEIGHT);
    painter.setRenderHint(QPainter::Antialiasing);

    if(dark_theme) {
        painter.fillRect(fill_rectangle, Qt::black);
        painter.setBrush(QColor(Qt::white));
    }
    else {
        painter.fillRect(fill_rectangle, Qt::white);
        painter.setBrush(QColor(60, 60, 60));
    }


    if(this->is_paused) {
        static const QPointF PLAY_BUTTON[3] = {
            QPointF(
                BUTTON_PADDING_HORIZ, BUTTON_PADDING_VERT
            ),
            QPointF(
                BUTTON_PADDING_HORIZ, PLAYBACK_HEIGHT - BUTTON_PADDING_VERT
            ),
            QPointF(
                BUTTON_PADDING_HORIZ + BUTTON_ICON_WIDTH, PLAYBACK_HEIGHT / 2.0
            ),
        };
        painter.drawPolygon(PLAY_BUTTON, 3);
    }
    else {
        static const QRectF PAUSE_BUTTON[2] = {
            QRectF(
                BUTTON_PADDING_HORIZ, BUTTON_PADDING_VERT,
                PAUSE_BUTTON_THICKNESS, PLAYBACK_HEIGHT - BUTTON_PADDING_VERT * 2.0
            ),
            QRectF(
                BUTTON_FULL_WIDTH - PAUSE_BUTTON_THICKNESS - BUTTON_PADDING_HORIZ, BUTTON_PADDING_VERT,
                PAUSE_BUTTON_THICKNESS, PLAYBACK_HEIGHT - BUTTON_PADDING_VERT * 2.0
            ),
        };

        painter.drawRects(PAUSE_BUTTON, 2);
    }
    
    auto bounds = this->playback_bar_bounds();

    QColor progress_color = dark_theme ? QColor(20, 140, 255) : QColor(20, 60, 255);
    QColor remaining_color = QColor(127, 127, 127);

    QPointF center_point(bounds.x(), bounds.y() + BAR_THICKNESS / 2.0);

    if(this->playback_progress <= 0.0) {
        painter.fillRect(bounds, remaining_color);
    }
    else if(this->playback_progress >= 1.0) {
        painter.fillRect(bounds, progress_color);
        center_point.setX(bounds.x() + bounds.width());
    }
    else {
        auto elapsed_bounds = bounds;
        auto remaining_bounds = bounds;

        int elapsed_width = static_cast<int>(elapsed_bounds.width() * this->playback_progress);
        int remaining_width = remaining_bounds.width() - elapsed_width;
        center_point.setX(elapsed_bounds.x() + elapsed_width);

        elapsed_bounds.setWidth(elapsed_width);
        remaining_bounds.setX(remaining_bounds.x() + elapsed_width);

        painter.fillRect(elapsed_bounds, progress_color);
        painter.fillRect(remaining_bounds, remaining_color);
    }

    painter.drawEllipse(center_point, INDICATOR_RADIUS, INDICATOR_RADIUS);

    QWidget::paintEvent(event);
}

QRectF ReplayPlaybackControls::playback_bar_bounds() {
    float x = BUTTON_FULL_WIDTH + BAR_PADDING - BUTTON_PADDING_HORIZ;
    return QRectF(
        x, PLAYBACK_HEIGHT / 2.0 - BAR_THICKNESS / 2.0,
        this->width() - BAR_PADDING - x, BAR_THICKNESS
    );
}

void ReplayPlaybackControls::tick() {
    // TODO: check if dimensions have changed?

    bool needs_repaint = false;

    if(supershuckie_frontend_is_paused(this->main_window->frontend) != this->is_paused) {
        this->is_paused = !this->is_paused;
        needs_repaint = true;
    }

    std::uint32_t elapsed_frames;
    std::uint32_t total_frames;

    supershuckie_frontend_get_replay_playback_time(this->main_window->frontend, &total_frames, nullptr);
    supershuckie_frontend_get_elapsed_time(this->main_window->frontend, &elapsed_frames, nullptr);

    double calculated_progress = total_frames == 0 ? 0.0 : static_cast<double>(elapsed_frames) / static_cast<double>(total_frames);
    if(!this->is_clicking_on_bar && this->playback_progress != calculated_progress) {
        this->playback_progress = calculated_progress;
        needs_repaint = true;
    }
    
    if(needs_repaint) {
        this->repaint();
    }
}

void ReplayPlaybackControls::mousePressEvent(QMouseEvent *event) {
    int x = event->position().x();

    if(x < BUTTON_FULL_WIDTH) {
        supershuckie_frontend_set_paused(this->main_window->frontend, !supershuckie_frontend_is_paused(this->main_window->frontend));
        return;
    }

    auto progress_requested = this->progress_on_bar(x);
    if(progress_requested < 0.0 || progress_requested > 1.0) {
        return;
    }

    this->playback_progress = progress_requested;

    supershuckie_frontend_set_playback_frozen(this->main_window->frontend, true);
    supershuckie_frontend_set_playback_frame(this->main_window->frontend, this->progress_to_frame(progress_requested));
    this->is_clicking_on_bar = true;
    this->repaint();
}

void ReplayPlaybackControls::mouseReleaseEvent(QMouseEvent *event) {
    if(this->is_clicking_on_bar) {
        supershuckie_frontend_set_playback_frozen(this->main_window->frontend, false);
        this->is_clicking_on_bar = false;
    }
}

double ReplayPlaybackControls::progress_on_bar(int x) {
    auto bounds = this->playback_bar_bounds();
    return static_cast<double>(x - bounds.x()) / static_cast<double>(bounds.width());
}

std::uint32_t ReplayPlaybackControls::progress_to_frame(double progress) {
    if(progress <= 0.0) {
        return 0;
    }

    std::uint32_t total_frames;
    supershuckie_frontend_get_replay_playback_time(this->main_window->frontend, &total_frames, nullptr);

    if(progress >= 1.0) {
        return total_frames;
    }

    return static_cast<double>(total_frames * progress + 0.5);
}

void ReplayPlaybackControls::mouseMoveEvent(QMouseEvent *event) {
    if(!this->is_clicking_on_bar) {
        return;
    }

    double progress = this->progress_on_bar(event->position().x());
    this->playback_progress = progress;

    supershuckie_frontend_set_playback_frame(
        this->main_window->frontend,
        this->progress_to_frame(progress)
    );

    this->repaint();
}

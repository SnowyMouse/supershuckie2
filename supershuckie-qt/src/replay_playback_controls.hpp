#include <QWidget>

#include <cstdint>

namespace SuperShuckie64 {

class MainWindow;

class ReplayPlaybackControls: public QWidget {
    Q_OBJECT
    friend MainWindow;
public:
    ReplayPlaybackControls(MainWindow *main_window, QWidget *parent);
private:
    MainWindow *main_window;

    void paintEvent(QPaintEvent *event) override;
    void mouseMoveEvent(QMouseEvent *event) override;
    void mousePressEvent(QMouseEvent *event) override;
    void mouseReleaseEvent(QMouseEvent *event) override;

    QRectF playback_bar_bounds();
    double progress_on_bar(int x);
    std::uint32_t progress_to_frame(double progress);
    
    void tick();

    bool is_paused = false;
    bool is_clicking_on_bar = false;

    double playback_progress = 0.0;
};
}

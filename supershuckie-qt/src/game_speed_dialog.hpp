#ifndef __SUPERSHUCKIE_GAME_SPEED_DIALOG_HPP__
#define __SUPERSHUCKIE_GAME_SPEED_DIALOG_HPP__

#include <QDialog>

class QSpinBox;
class QLabel;

namespace SuperShuckie64 {

class MainWindow;

class GameSpeedDialog: public QDialog {
    Q_OBJECT
    friend MainWindow;
private:
    GameSpeedDialog(MainWindow *parent);
    MainWindow *parent;
    QSpinBox *base_speed_slider;
    QSpinBox *turbo_speed_slider;
    QLabel *base_speed_text;
    QLabel *turbo_speed_text;

    void accept() override;

private slots:
    void do_update_speed();
};

}

#endif
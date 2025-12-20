#ifndef __SUPERSHUCKIE_GAME_SPEED_DIALOG_HPP__
#define __SUPERSHUCKIE_GAME_SPEED_DIALOG_HPP__

#include <QDialog>

class QSpinBox;
class QLabel;

namespace SuperShuckie64 {

class SuperShuckieMainWindow;

class SuperShuckieGameSpeedDialog: public QDialog {
    Q_OBJECT
    friend SuperShuckieMainWindow;
private:
    SuperShuckieGameSpeedDialog(SuperShuckieMainWindow *parent);
    SuperShuckieMainWindow *parent;
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
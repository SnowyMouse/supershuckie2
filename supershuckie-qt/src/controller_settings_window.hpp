#ifndef __SUPERSHUCKIE_CONTROLLER_SETTING_DIALOG_HPP__
#define __SUPERSHUCKIE_CONTROLLER_SETTING_DIALOG_HPP__

#include <QDialog>

namespace SuperShuckie64 {

class MainWindow;

class ControlsSettingsWindow : public QDialog {
    Q_OBJECT;
public:
    ControlsSettingsWindow(MainWindow *parent);
    int exec() override;
private:
    MainWindow *parent;
};

}

#endif

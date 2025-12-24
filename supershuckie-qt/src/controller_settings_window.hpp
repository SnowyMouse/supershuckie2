#ifndef __SUPERSHUCKIE_CONTROLLER_SETTING_DIALOG_HPP__
#define __SUPERSHUCKIE_CONTROLLER_SETTING_DIALOG_HPP__

#include <QDialog>
#include <QLineEdit>
#include <vector>
#include <memory>

#include <supershuckie/control_settings.h>

class QLineEdit;

namespace SuperShuckie64 {

class MainWindow;

class ControlSettingsSetting;

class ControlsSettingsWindow : public QDialog {
    Q_OBJECT;
    friend ControlSettingsSetting;
public:
    ControlsSettingsWindow(MainWindow *parent, SuperShuckieControlSettingsRaw *settings);
    int exec() override;
private:
    std::vector<ControlSettingsSetting *> edit_boxes;
    MainWindow *parent;
    std::unique_ptr<SuperShuckieControlSettingsRaw, decltype(&supershuckie_control_settings_free)> settings;
    const char *ss_device_name();
    void update_textboxes();
};

class ControlSettingsSetting : public QLineEdit {
    Q_OBJECT;
    friend ControlsSettingsWindow;
private:
    ControlSettingsSetting(ControlsSettingsWindow *window, SuperShuckieControlType control_type, SuperShuckieControlModifier control_modifier);

    ControlsSettingsWindow *window;

    SuperShuckieControlType control_type;
    SuperShuckieControlModifier control_modifier;

    void mousePressEvent(QMouseEvent *event) override;
    void keyPressEvent(QKeyEvent *event) override;

    void update_parent();
};

}

#endif

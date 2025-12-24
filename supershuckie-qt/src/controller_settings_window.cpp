#include "controller_settings_window.hpp"
#include "main_window.hpp"

#include <QGridLayout>
#include <QLabel>
#include <QLineEdit>
#include <QPushButton>
#include <QKeyEvent>
#include <QMouseEvent>
#include <QString>

using namespace SuperShuckie64;

ControlsSettingsWindow::ControlsSettingsWindow(MainWindow *parent, SuperShuckieControlSettingsRaw *settings): QDialog(parent), parent(parent), settings(settings, supershuckie_control_settings_free) {
    this->setWindowTitle("Controls settings");
    
    SuperShuckieControlType control_types = 0;
    SuperShuckieControlModifier control_modifiers = 0;
    const char *label = nullptr;

    auto *layout = new QGridLayout(this);

    int control_box_y_offset = 100;
    
    for(control_types = 0; (label = supershuckie_control_settings_control_name(control_types)) != nullptr; control_types++) {
        if(supershuckie_control_settings_control_is_spoiler(control_types)) {
            continue;
        }
        
        auto *name = new QLabel(label, this);
        layout->addWidget(name, control_box_y_offset + control_types + 1, 0);
    }
    
    for(control_modifiers = 0; (label = supershuckie_control_settings_modifier_name(control_modifiers)) != nullptr; control_modifiers++) {
        auto *name = new QLabel(label, this);
        
        int y = control_box_y_offset;
        int x = control_modifiers + 1;
        layout->addWidget(name, y, x);

        for(SuperShuckieControlType control_type = 0; control_type < control_types; control_type++) {
            if(!supershuckie_control_settings_control_is_button(control_type) && control_modifiers != 0) {
                continue;
            }
            if(supershuckie_control_settings_control_is_spoiler(control_type)) {
                continue;
            }

            auto *edit = new ControlSettingsSetting(this, control_type, control_modifiers);
            this->edit_boxes.emplace_back(edit);
            layout->addWidget(edit, control_box_y_offset + control_type + 1, x);
        }
    }

    int offset_for_remaining_things = control_box_y_offset + control_types + 1;
    int width_span = control_modifiers + 1;

    auto *note = new QLabel("Left-click to select a setting. Right-click to clear.", this);
    note->setAlignment(Qt::AlignHCenter);
    note->setAttribute(Qt::WA_MacSmallSize);
    layout->addWidget(note, offset_for_remaining_things++, 0, 1, width_span);

    auto *save = new QPushButton("OK", this);
    layout->addWidget(save, offset_for_remaining_things++, 0, 1, width_span);
    connect(save, SIGNAL(clicked()), this, SLOT(accept()));

    this->setFixedSize(this->sizeHint());
    this->update_textboxes();
}

int ControlsSettingsWindow::exec() {
    this->parent->stop_timer();
    int return_value = QDialog::exec();
    this->parent->start_timer();
    return return_value;
}

ControlSettingsSetting::ControlSettingsSetting(ControlsSettingsWindow *window, SuperShuckieControlType control_type, SuperShuckieControlModifier control_modifier):
    QLineEdit(window),
    window(window),
    control_type(control_type),
    control_modifier(control_modifier) {
    this->setContextMenuPolicy(Qt::NoContextMenu);
}

const char *ControlsSettingsWindow::ss_device_name() {
    return nullptr;
}

void ControlSettingsSetting::mousePressEvent(QMouseEvent *event) {
    auto button = event->button();

    if(button & Qt::RightButton) {
        event->ignore();
        supershuckie_control_settings_clear_controls_for_device(
            this->window->settings.get(),
            this->window->ss_device_name(),
            this->control_type,
            this->control_modifier
        );
        this->window->update_textboxes();
    }
    else if(button & Qt::LeftButton) {

    }
}

void ControlSettingsSetting::keyPressEvent(QKeyEvent *event) {
    event->ignore();

    const char *device = this->window->ss_device_name();
    if(device != nullptr) {
        return;
    }

    supershuckie_control_settings_set_control_for_device(
        this->window->settings.get(),
        device,
        false,
        event->key(),
        this->control_type,
        this->control_modifier
    );

    this->window->update_textboxes();
}

void ControlsSettingsWindow::update_textboxes() {
    std::vector<std::int32_t> buffer_button;
    std::vector<std::int32_t> buffer_axis;

    const char *device = this->ss_device_name();

    for(ControlSettingsSetting *setting: this->edit_boxes) {
        auto button_len = supershuckie_control_settings_get_controls_for_device(
            this->settings.get(),
            device,
            false,
            setting->control_type,
            setting->control_modifier,
            nullptr,
            0
        );
        buffer_button.resize(button_len);
        supershuckie_control_settings_get_controls_for_device(
            this->settings.get(),
            device,
            false,
            setting->control_type,
            setting->control_modifier,
            buffer_button.data(),
            buffer_button.size()
        );

        if(device != nullptr) {
            auto axis_len = supershuckie_control_settings_get_controls_for_device(
                this->settings.get(),
                device,
                true,
                setting->control_type,
                setting->control_modifier,
                nullptr,
                0
            );
            buffer_axis.resize(axis_len);
            supershuckie_control_settings_get_controls_for_device(
                this->settings.get(),
                device,
                false,
                setting->control_type,
                setting->control_modifier,
                buffer_axis.data(),
                buffer_axis.size()
            );
        }
        
        QString label;

        if(device == nullptr) {
            for(auto button : buffer_button) {
                auto name = QKeySequence(button).toString();
                if(label.isEmpty()) {
                    label = name;
                }
                else {
                    label += ", ";
                    label += name;
                }
            }
        }

        setting->setText(label);
    }
}
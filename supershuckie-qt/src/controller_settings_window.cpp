#include "controller_settings_window.hpp"
#include "main_window.hpp"

using namespace SuperShuckie64;

ControlsSettingsWindow::ControlsSettingsWindow(MainWindow *parent): QDialog(parent), parent(parent) {

}

int ControlsSettingsWindow::exec() {
    this->parent->stop_timer();
    int return_value = QDialog::exec();
    this->parent->start_timer();
    return return_value;
}
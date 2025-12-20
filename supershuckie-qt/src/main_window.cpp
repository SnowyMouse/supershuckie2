#include <cstdio>
#include <QLayout>
#include <SDL3/SDL.h>
#include <QMenuBar>
#include <QCloseEvent>
#include <QFileDialog>

#include <supershuckie/frontend.h>

#include "ask_for_text_dialog.hpp"
#include "error.hpp"
#include "file_rw.hpp"
#include "game_speed_dialog.hpp"
#include "render_widget.hpp"
#include "main_window.hpp"

using namespace SuperShuckie64;

static const char *USE_NUMBER_KEYS_FOR_QUICK_SLOTS = "number_keys_for_quick_slots";
static const char *WINDOW_XY = "window_xy";

SuperShuckieMainWindow::SuperShuckieMainWindow(): QMainWindow() {
    // Remove rounded corners (Windows)
    #ifdef _WIN32
    DWORD one = 1;
    DwmSetWindowAttribute(reinterpret_cast<HWND>(this->winId()), 33, &one, sizeof(one));
    #endif

    this->render_widget = new SuperShuckieRenderWidget(this);
    this->setCentralWidget(this->render_widget);

    this->setWindowFlags(Qt::MSWindowsFixedSizeDialogHint);
    this->layout()->setSizeConstraint(QLayout::SetFixedSize);

    this->ticker.setInterval(1);
    this->ticker.callOnTimeout(this, &SuperShuckieMainWindow::tick);
    this->ticker.start();

    this->set_up_menu();

    SuperShuckieFrontendCallbacks callbacks = {};
    callbacks.user_data = this;
    callbacks.refresh_screens = SuperShuckieMainWindow::on_refresh_screens;
    callbacks.change_video_mode = SuperShuckieMainWindow::on_change_video_mode;
    this->frontend = supershuckie_frontend_new("./UserData", &callbacks);

    const char *quick_slots = supershuckie_frontend_get_custom_setting(this->frontend, USE_NUMBER_KEYS_FOR_QUICK_SLOTS);
    if(quick_slots != nullptr && quick_slots[0] == '1') {
        this->use_number_keys_for_quick_slots = true;
        this->use_number_row_for_quick_slots->setChecked(true);
        this->set_quick_load_shortcuts();
    }

    const char *xy = supershuckie_frontend_get_custom_setting(this->frontend, WINDOW_XY);
    if(xy != nullptr) {
        int x;
        int y;
        if(std::sscanf(xy, "%d|%d", &x, &y) == 2) {
            auto geometry = this->geometry();
            geometry.setX(x);
            geometry.setY(y);
            this->setGeometry(geometry);
        }
    }
}

void SuperShuckieMainWindow::set_title(const char *title) {
    std::strncpy(this->title_text, title, sizeof(this->title_text) - 1);
    this->refresh_title();
}

void SuperShuckieMainWindow::refresh_title() {
    char fmt[512];

    const char *rom_name = this->frontend ? supershuckie_frontend_get_rom_name(this->frontend) : "(Frontend not yet loaded)";
    if(rom_name == nullptr) {
        rom_name = "No ROM Loaded";
    };
    
    if(this->title_text[0] == 0) {
        std::snprintf(fmt, sizeof(fmt), "Super Shuckie 2 (name TBD) - %s - %.00f FPS", rom_name, this->current_fps);
    }
    else {
        std::snprintf(fmt, sizeof(fmt), "Super Shuckie 2 (name TBD) - %s - %s - %.00f FPS", rom_name, this->title_text, this->current_fps);
    }

    this->setWindowTitle(fmt);
}

void SuperShuckieMainWindow::tick() {
    SDL_Event event;
    while(SDL_PollEvent(&event)) {
        switch(event.type) {
            // If we hit ctrl-c, close the window (saves)
            case SDL_EventType::SDL_EVENT_QUIT:
                this->close();
                // If the window wasn't closed, warn
                if(this->isVisible()) {
                    std::fputs("Can't close the main window. Finish what you're doing, first!\n", stderr);
                    break;
                }
                return;
        }
    }

    auto now = clock::now();
    auto time_since_last_second_us = std::chrono::duration_cast<std::chrono::microseconds>(now - this->second_start).count();
    if(time_since_last_second_us > 1000000) {
        this->current_fps = 1000000.0 * static_cast<double>(this->frames_in_last_second) / static_cast<double>(time_since_last_second_us);
        this->frames_in_last_second = 0;
        this->second_start = now;
        this->refresh_title();
    }

    supershuckie_frontend_tick(this->frontend);
}

void SuperShuckieMainWindow::set_up_menu() {
    this->menu_bar = new QMenuBar(this);
    this->setMenuBar(this->menu_bar);

    // Add base menus
    this->set_up_file_menu();
    this->set_up_gameplay_menu();
    this->set_up_save_states_menu();
    this->set_up_replays_menu();
    this->set_up_settings_menu();

    this->refresh_action_states();
}

void SuperShuckieMainWindow::set_up_file_menu() {
    this->file_menu = this->menu_bar->addMenu("File");

    this->open_rom = this->file_menu->addAction("Open ROM...");
    this->open_rom->setShortcut(QKeyCombination(Qt::ControlModifier, Qt::Key_O));
    connect(this->open_rom, SIGNAL(triggered()), this, SLOT(do_open_rom()));

    this->close_rom = this->file_menu->addAction("Close ROM");
    this->close_rom->setShortcut(QKeyCombination(Qt::ControlModifier, Qt::Key_W));
    connect(this->close_rom, SIGNAL(triggered()), this, SLOT(do_close_rom()));

    this->unload_rom = this->file_menu->addAction("Unload ROM without saving");
    this->unload_rom->setShortcut(QKeyCombination(Qt::ControlModifier | Qt::ShiftModifier, Qt::Key_W));
    connect(this->unload_rom, SIGNAL(triggered()), this, SLOT(do_unload_rom()));

    this->file_menu->addSeparator();
    this->quit = this->file_menu->addAction("Quit");
    this->quit->setShortcut(QKeyCombination(Qt::ControlModifier, Qt::Key_Q));
    connect(this->quit, SIGNAL(triggered()), this, SLOT(close()));
}

void SuperShuckieMainWindow::set_up_gameplay_menu() {
    this->gameplay_menu = this->menu_bar->addMenu("Gameplay");

    this->new_game = this->gameplay_menu->addAction("New game...");
    this->new_game->setShortcut(QKeyCombination(Qt::ControlModifier, Qt::Key_N));
    connect(this->new_game, SIGNAL(triggered()), this, SLOT(do_new_game()));

    this->save_game = this->gameplay_menu->addAction("Save game");
    this->save_game->setShortcut(QKeyCombination(Qt::ControlModifier, Qt::Key_S));
    connect(this->save_game, SIGNAL(triggered()), this, SLOT(do_save_game()));

    this->save_new_game = this->gameplay_menu->addAction("Save as new game...");
    this->save_new_game->setShortcut(QKeyCombination(Qt::ControlModifier | Qt::ShiftModifier, Qt::Key_S));
    connect(this->save_new_game, SIGNAL(triggered()), this, SLOT(do_save_new_game()));

    this->gameplay_menu->addSeparator();

    this->reset_console = this->gameplay_menu->addAction("Reset console");
    connect(this->reset_console, SIGNAL(triggered()), this, SLOT(do_reset_console()));

    this->pause = this->gameplay_menu->addAction("Pause");
    this->pause->setCheckable(true);
    this->pause->setShortcut(QKeyCombination(Qt::ControlModifier, Qt::Key_P));
    connect(this->pause, SIGNAL(triggered()), this, SLOT(do_toggle_pause()));
}

void SuperShuckieMainWindow::set_up_save_states_menu() {
    this->save_states_menu = this->menu_bar->addMenu("Save states");

    this->quick_slots = this->save_states_menu->addMenu("Quick slot");
    for(std::size_t i = 1; i <= SuperShuckieMainWindow::QUICK_SAVE_STATE_COUNT; i++) {
        char fmt[64];

        std::snprintf(fmt, sizeof(fmt), "Quick slot #%zu", i);
        QMenu *menu = quick_slots->addMenu(fmt);

        std::snprintf(fmt, sizeof(fmt), "Load quick slot #%zu", i);
        auto *quick_load = new SuperShuckieNumberedAction(this, fmt, i, &SuperShuckieMainWindow::quick_load);

        std::snprintf(fmt, sizeof(fmt), "Save quick slot #%zu", i);
        auto *quick_save = new SuperShuckieNumberedAction(this, fmt, i, &SuperShuckieMainWindow::quick_save);

        this->quick_load_save_states[i - 1] = quick_load;
        menu->addAction(quick_load);
        this->quick_save_save_states[i - 1] = quick_save;
        menu->addAction(quick_save);
    }
    
    quick_slots->addSeparator();
    
    this->use_number_row_for_quick_slots = quick_slots->addAction("Use number row instead of function keys");
    this->use_number_row_for_quick_slots->setCheckable(true);
    connect(this->use_number_row_for_quick_slots, SIGNAL(triggered()), this, SLOT(do_toggle_number_row_for_save_states()));

    this->save_states_menu->addSeparator();
    
    this->undo_load_save_state = this->save_states_menu->addAction("Undo load save state");
    this->undo_load_save_state->setShortcut(QKeyCombination(Qt::ControlModifier, Qt::Key_U));
    connect(this->undo_load_save_state, SIGNAL(triggered()), this, SLOT(do_undo_load_save_state()));
    
    this->redo_load_save_state = this->save_states_menu->addAction("Redo load save state");
    this->redo_load_save_state->setShortcut(QKeyCombination(Qt::ControlModifier | Qt::ShiftModifier, Qt::Key_U));
    connect(this->redo_load_save_state, SIGNAL(triggered()), this, SLOT(do_redo_load_save_state()));

    this->set_quick_load_shortcuts();
}

void SuperShuckieMainWindow::set_up_replays_menu() {
    this->replays_menu = this->menu_bar->addMenu("Replays");
    
    this->record_replay = this->replays_menu->addAction("Nidooooooooooooooooooo");
    this->resume_replay = this->replays_menu->addAction("Resume recording replay");
    this->replays_menu->addSeparator();
    this->play_replay = this->replays_menu->addAction("NidoNidoNidoNido");

    connect(this->record_replay, SIGNAL(triggered()), this, SLOT(do_record_replay()));
    connect(this->resume_replay, SIGNAL(triggered()), this, SLOT(do_resume_replay()));
    connect(this->play_replay, SIGNAL(triggered()), this, SLOT(do_play_replay()));

    this->record_replay->setShortcut(QKeyCombination(Qt::ControlModifier, Qt::Key_R));
    this->resume_replay->setShortcut(QKeyCombination(Qt::ShiftModifier | Qt::ControlModifier, Qt::Key_R));
    this->play_replay->setShortcut(QKeyCombination(Qt::ShiftModifier | Qt::ControlModifier, Qt::Key_P));
}

SuperShuckieNumberedAction::SuperShuckieNumberedAction(SuperShuckieMainWindow *parent, const char *text, std::uint8_t number, on_activated activated): QAction(text, parent), number(number), parent(parent), activated_fn(activated) {
    connect(this, SIGNAL(triggered()), this, SLOT(activated()));
}

void SuperShuckieNumberedAction::activated() {
    if(this->parent->frontend == nullptr) {
        return;
    }
    (this->parent->*this->activated_fn)(this->number);
}

void SuperShuckieMainWindow::set_video_scale(std::uint8_t scale) {
    supershuckie_frontend_set_video_scale(this->frontend, scale);
}

void SuperShuckieMainWindow::make_save_state(const char *state) {
    char error[256];
    auto success = supershuckie_frontend_create_save_state(this->frontend, state, error, sizeof(error));
    if(success) {
        char title[512];
        std::snprintf(title, sizeof(title), "Created state \"%s\"", error);
        this->set_title(title);
    }
    else {
        DISPLAY_ERROR_DIALOG("Failed to create save state", "%s", error);
    }
}

void SuperShuckieMainWindow::load_save_state(const char *state) {
    char error[256];
    auto success = supershuckie_frontend_load_save_state(this->frontend, state, error, sizeof(error));
    if(success) {
        char title[512];
        std::snprintf(title, sizeof(title), "Loaded state \"%s\"", state);
        this->set_title(title);
    }
    else if(error[0] != 0) {
        DISPLAY_ERROR_DIALOG("Failed to load save state", "%s", error);
    }
    else {
        char title[512];
        std::snprintf(title, sizeof(title), "State \"%s\" does not exist", state);
        this->set_title(title);
    }
}

void SuperShuckieMainWindow::quick_save(std::uint8_t index) {
    char fmt[16];
    std::snprintf(fmt, sizeof(fmt), "quick-%d", index);
    this->make_save_state(fmt);
}

void SuperShuckieMainWindow::quick_load(std::uint8_t index) {
    char fmt[16];
    std::snprintf(fmt, sizeof(fmt), "quick-%d", index);
    this->load_save_state(fmt);
}

void SuperShuckieMainWindow::set_up_settings_menu() {
    this->settings_menu = this->menu_bar->addMenu("Settings");
    
    auto *video_scaling = this->settings_menu->addMenu("Video scaling");
    for(std::size_t i = 1; i <= SuperShuckieMainWindow::VIDEO_SCALE_COUNT; i++) {
        char fmt[256];
        std::snprintf(fmt, sizeof(fmt), "%zux", i);

        auto *action = new SuperShuckieNumberedAction(this, fmt, static_cast<uint8_t>(i), &SuperShuckieMainWindow::set_video_scale);
        video_scaling->addAction(action);
        this->change_video_scale[i - 1] = action;
        action->setCheckable(true);
    }

    auto *game_speed = this->settings_menu->addAction("Game speed...");
    connect(game_speed, SIGNAL(triggered()), this, SLOT(do_open_game_speed_dialog()));
}

void SuperShuckieMainWindow::refresh_action_states() {
    bool game_loaded = this->is_game_running();

    this->gameplay_menu->setEnabled(game_loaded);
    this->replays_menu->setEnabled(game_loaded);
    this->close_rom->setEnabled(game_loaded);

    for(auto &state : this->quick_load_save_states) {
        state->setEnabled(game_loaded);
    }

    for(auto &state : this->quick_save_save_states) {
        state->setEnabled(game_loaded);
    }

    this->undo_load_save_state->setEnabled(game_loaded);
    this->redo_load_save_state->setEnabled(game_loaded);
    
    switch(this->replay_status) {
        case ReplayStatus::NoReplay: {
            this->play_replay->setEnabled(true);
            this->record_replay->setEnabled(true);
            this->resume_replay->setEnabled(true);
            
            this->close_rom->setShortcut(QKeyCombination(Qt::ControlModifier, Qt::Key_W));

            this->record_replay->setText("Record replay");
            this->play_replay->setText("Play replay");
            break;
        }
        case ReplayStatus::Recording: {
            this->play_replay->setEnabled(false);
            this->record_replay->setEnabled(true);
            this->resume_replay->setEnabled(false);

            this->record_replay->setText("Stop recording replay");
            this->play_replay->setText("Play replay");
            break;
        }
        case ReplayStatus::PlayingBack: {
            this->play_replay->setEnabled(true);
            this->record_replay->setEnabled(false);
            this->resume_replay->setEnabled(false);

            this->record_replay->setText("Record replay");
            this->play_replay->setText("Stop replay");
            break;
        }
    }
}

void SuperShuckieMainWindow::do_open_rom() {
    QFileDialog rom_opener;
    rom_opener.setFileMode(QFileDialog::FileMode::ExistingFile);
    rom_opener.setNameFilters(QStringList({"GB/GBC ROM dumps (*.gb *.gbc)", "Any files (*)"}));
    rom_opener.setWindowTitle("Select a ROM to open");
    rom_opener.exec();

    auto files = rom_opener.selectedFiles();
    if(files.size() != 1) {
        return;
    }

    this->load_rom(files[0].toStdString());
}

void SuperShuckieMainWindow::load_rom(const std::filesystem::path &path) {
    char error[256] = "";

    auto path_string = path.string();
    if(!supershuckie_frontend_load_rom(this->frontend, path.string().c_str(), error, sizeof(error))) {
        DISPLAY_ERROR_DIALOG("Can't load ROM", "\"%s\" failed to load:\n\n%s", path_string.c_str(), error);
    }
}

void SuperShuckieMainWindow::do_close_rom() {
    supershuckie_frontend_close_rom(this->frontend);
}

void SuperShuckieMainWindow::do_unload_rom() {
    supershuckie_frontend_unload_rom(this->frontend);
}

void SuperShuckieMainWindow::do_new_game() noexcept {
    auto text = AskForTextDialog::ask(this, "New game", "Enter the name of the new (empty) save file", "WARNING: If the file exists, it will be deleted immediately.");
    if(text == std::nullopt) {
        return;
    }
    supershuckie_frontend_load_or_create_save_file(this->frontend, text->c_str(), true);

    char fmt[256];
    std::snprintf(fmt, sizeof(fmt), "Created empty save file \"%s\"", text->c_str());
    this->set_title(fmt);
}

void SuperShuckieMainWindow::do_save_game() {
    char err[256];
    if(supershuckie_frontend_save_sram(this->frontend, err, sizeof(err))) {
        this->set_title("Saved SRAM successfully!");
    }
    else {
        DISPLAY_ERROR_DIALOG("Can't save SRAM", "%s", err);
    }
}

void SuperShuckieMainWindow::do_save_new_game() {
    auto text = AskForTextDialog::ask(this, "Save as new game", "Enter the name of the new (copied) save file", "WARNING: If the file exists, it will be overwritten on save.");
    if(text == std::nullopt) {
        return;
    }
    supershuckie_frontend_set_current_save_file(this->frontend, text->c_str());

    char fmt[256];
    std::snprintf(fmt, sizeof(fmt), "Switched to save file \"%s\"", text->c_str());
    this->set_title(fmt);
}

void SuperShuckieMainWindow::do_reset_console() {
    supershuckie_frontend_hard_reset_console(this->frontend);
}

void SuperShuckieMainWindow::do_toggle_pause() {
    supershuckie_frontend_set_paused(this->frontend, this->pause->isChecked());
}

void SuperShuckieMainWindow::do_toggle_number_row_for_save_states() {
    this->use_number_keys_for_quick_slots = this->use_number_row_for_quick_slots->isChecked();
    this->set_quick_load_shortcuts();
    
    supershuckie_frontend_set_custom_setting(this->frontend, USE_NUMBER_KEYS_FOR_QUICK_SLOTS, this->use_number_keys_for_quick_slots ? "1" : "0");
}

void SuperShuckieMainWindow::set_quick_load_shortcuts() {
    Qt::KeyboardModifiers control = static_cast<Qt::KeyboardModifiers>(this->use_number_keys_for_quick_slots ? Qt::ControlModifier : 0);

    for(std::size_t i = 0; i < SuperShuckieMainWindow::QUICK_SAVE_STATE_COUNT; i++) {
        Qt::Key key = static_cast<Qt::Key>((this->use_number_keys_for_quick_slots ? Qt::Key_1 : Qt::Key_F1) + i);
        this->quick_save_save_states[i]->setShortcut(QKeyCombination(control | Qt::ShiftModifier, key));
        this->quick_load_save_states[i]->setShortcut(QKeyCombination(control, key));
    }
}

void SuperShuckieMainWindow::closeEvent(QCloseEvent *event) {
    QWidget::closeEvent(event);

    if(this->frontend) {
        char xy[256];
        auto geometry = this->geometry();
        std::snprintf(xy, sizeof(xy), "%d|%d", geometry.x(), geometry.y());
        supershuckie_frontend_set_custom_setting(this->frontend, WINDOW_XY, xy);
        supershuckie_frontend_write_settings(this->frontend);
        supershuckie_frontend_save_sram(this->frontend, nullptr, 0);
    }

    // if(!this->try_unload_rom()) {
        // event->ignore();
    // }
}

SuperShuckieMainWindow::~SuperShuckieMainWindow() {
    if(this->frontend) {
        supershuckie_frontend_free(this->frontend);
        this->frontend = nullptr;
    }
}

void SuperShuckieMainWindow::do_record_replay() {
    // FIXME
}

void SuperShuckieMainWindow::do_resume_replay() {
    // FIXME
}

void SuperShuckieMainWindow::do_play_replay() {
    // FIXME
}

void SuperShuckieMainWindow::on_refresh_screens(void *user_data, std::size_t screen_count, const uint32_t *const *pixels) {
    auto *self = reinterpret_cast<SuperShuckieMainWindow *>(user_data);
    
    const uint32_t *first_screen = pixels[0];
    self->frames_in_last_second += 1;
    self->render_widget->refresh_screen(first_screen);
}

void SuperShuckieMainWindow::on_change_video_mode(void *user_data, std::size_t screen_count, const SuperShuckieScreenData *screen_data, std::uint8_t video_scale) {
    auto *self = reinterpret_cast<SuperShuckieMainWindow *>(user_data);
    
    const SuperShuckieScreenData &first_screen = screen_data[0];
    self->render_widget->set_dimensions(first_screen.width, first_screen.height, video_scale);
    self->refresh_action_states();
    self->frames_in_last_second = 0;
    self->current_fps = 0.0;
    self->second_start = clock::now();
    if(self->is_game_running()) {
        self->set_title("Loaded ROM successfully!");
    }
    else {
        self->set_title();
    }

    for(auto &scale : self->change_video_scale) {
        scale->setChecked(scale->number == video_scale);
    }
}

bool SuperShuckieMainWindow::is_game_running() {
    return this->frontend != nullptr && supershuckie_frontend_is_game_running(this->frontend);
}

void SuperShuckieMainWindow::do_open_game_speed_dialog() noexcept {
    SuperShuckieGameSpeedDialog *dialog = new SuperShuckieGameSpeedDialog(this);

    dialog->exec();

    delete dialog;
}

void SuperShuckieMainWindow::do_undo_load_save_state() {
    if(supershuckie_frontend_undo_load_save_state(this->frontend)) {
        this->set_title("Undo load save state successful");
    }
    else {
        this->set_title("No more states in the stack!");
    }
}

void SuperShuckieMainWindow::do_redo_load_save_state() {
    if(supershuckie_frontend_redo_load_save_state(this->frontend)) {
        this->set_title("Redo load save state successful");
    }
    else {
        this->set_title("No more states in the stack!");
    }
}

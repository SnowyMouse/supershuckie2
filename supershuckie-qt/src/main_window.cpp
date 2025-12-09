#include <cstdio>
#include <QLayout>
#include <SDL3/sdl.h>
#include <QMenuBar>
#include <QCloseEvent>
#include <QFileDialog>

#include "../../../../bootrom/cgb/cgb_boot/cgb_boot_fast.hpp"

#include "error.hpp"
#include "file_rw.hpp"

#include "render_widget.hpp"
#include "main_window.hpp"

using namespace SuperShuckie64;

SuperShuckieMainWindow::SuperShuckieMainWindow(): QMainWindow(), core(SuperShuckieCore::new_null()) {
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
    this->try_unload_rom();
}

void SuperShuckieMainWindow::set_title(const char *title) {
    char fmt[512];

    const char *rom_name = (this->current_rom_name == std::nullopt) ? "No ROM loaded" : this->current_rom_name.value().c_str();
    
    if(title == nullptr || title[0] == 0) {
        std::snprintf(fmt, sizeof(fmt), "Super Shuckie 2 (name TBD) - %s", rom_name);
    }
    else {
        std::snprintf(fmt, sizeof(fmt), "Super Shuckie 2 (name TBD) - %s - %s", rom_name, title);
    }

    this->setWindowTitle(fmt);
}

void SuperShuckieMainWindow::refresh_screen_dimensions() {
    bool updated = false;

    const auto &screens = this->core.get_screens(updated);
    const auto &first_screen = screens[0];

    this->render_widget->set_dimensions(first_screen.width,first_screen.height,this->scale);
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

    this->render_widget->refresh_screen();
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
    this->pause->setShortcut(QKeyCombination(Qt::ControlModifier, Qt::Key_P));
    connect(this->pause, SIGNAL(triggered()), this, SLOT(do_toggle_pause()));
}

void SuperShuckieMainWindow::set_up_save_states_menu() {
    this->save_states_menu = this->menu_bar->addMenu("Save states");

    this->quick_slots = this->save_states_menu->addMenu("Quick slot");
    for(std::size_t i = 0; i < SuperShuckieMainWindow::QUICK_SAVE_STATE_COUNT; i++) {
        char fmt[64];

        std::snprintf(fmt, sizeof(fmt), "Quick slot #%zu", i + 1);
        QMenu *menu = quick_slots->addMenu(fmt);

        std::snprintf(fmt, sizeof(fmt), "Save quick slot #%zu", i + 1);
        this->quick_load_save_states[i] = menu->addAction(fmt);

        std::snprintf(fmt, sizeof(fmt), "Load quick slot #%zu", i + 1);
        this->quick_save_save_states[i] = menu->addAction(fmt);
    }
    
    quick_slots->addSeparator();
    
    this->use_number_row_for_quick_slots = quick_slots->addAction("Use number row instead of function keys");
    this->use_number_row_for_quick_slots->setCheckable(true);
    connect(this->use_number_row_for_quick_slots, SIGNAL(triggered()), this, SLOT(do_toggle_number_row_for_save_states()));

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
    this->play_replay->setShortcut(QKeyCombination(Qt::ControlModifier, Qt::Key_P));
}

void SuperShuckieMainWindow::set_up_settings_menu() {
    this->settings_menu = this->menu_bar->addMenu("Settings");
}

void SuperShuckieMainWindow::refresh_action_states() {
    this->gameplay_menu->setEnabled(this->game_loaded);
    this->replays_menu->setEnabled(this->game_loaded);
    this->close_rom->setEnabled(this->game_loaded);

    for(auto &state : this->quick_load_save_states) {
        state->setEnabled(this->game_loaded);
    }

    for(auto &state : this->quick_save_save_states) {
        state->setEnabled(this->game_loaded);
    }
    
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

SuperShuckieMainWindow::~SuperShuckieMainWindow() {

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
    auto extension = path.extension().string();
    for(char &c : extension) {
        if(c >= 'A' && c <= 'Z') {
            c = std::tolower(c);
        }
    }

    auto file = read_file(path);
    if(file == std::nullopt) {
        return;
    }

    if(extension == ".gb" || extension == ".gbc") {
        this->load_gbc(path.filename().string(), file.value());
    }
    else if(extension == "" || extension == ".") {
        DISPLAY_ERROR_DIALOG("Can't load ROM", "\"%s\" does not appear to be a valid ROM file!", path.filename().string().c_str());
    }
    else {
        DISPLAY_ERROR_DIALOG("Can't load ROM", "\"%s\" does not appear to be a valid ROM file!\n\n(unknown extension \"%s\")", path.filename().string().c_str(), extension.c_str());
    }
}

void SuperShuckieMainWindow::load_gbc(const std::string &name, const std::vector<std::byte> &data) {
    if(!this->try_unload_rom()) {
        return;
    }

    this->core = SuperShuckieCore::new_from_gameboy(data.data(), data.size(), AUTOGEN_CGB_BOOT_FAST_HPP_VAL, sizeof(AUTOGEN_CGB_BOOT_FAST_HPP_VAL), GameBoyType::GameBoyType__GameBoyColor);
    this->game_loaded = true;
    this->current_rom_name = name;
    this->on_rom_switch();
}

void SuperShuckieMainWindow::do_close_rom() {
    this->try_unload_rom();
}

void SuperShuckieMainWindow::do_new_game() {
    // FIXME
}

void SuperShuckieMainWindow::do_save_game() {
    // FIXME
}

void SuperShuckieMainWindow::do_save_new_game() {
    // FIXME
}

void SuperShuckieMainWindow::do_reset_console() {
    // FIXME
}

void SuperShuckieMainWindow::do_toggle_pause() {
    // FIXME
}

void SuperShuckieMainWindow::do_toggle_number_row_for_save_states() {
    this->use_number_keys_for_quick_slots = this->use_number_row_for_quick_slots->isChecked();
    this->set_quick_load_shortcuts();
    // FIXME: persist to config
}

void SuperShuckieMainWindow::set_quick_load_shortcuts() {
    Qt::KeyboardModifiers control = static_cast<Qt::KeyboardModifiers>(this->use_number_keys_for_quick_slots ? Qt::ControlModifier : 0);

    for(std::size_t i = 0; i < SuperShuckieMainWindow::QUICK_SAVE_STATE_COUNT; i++) {
        Qt::Key key = static_cast<Qt::Key>((this->use_number_keys_for_quick_slots ? Qt::Key_1 : Qt::Key_F1) + i);
        this->quick_load_save_states[i]->setShortcut(QKeyCombination(control | Qt::ShiftModifier, key));
        this->quick_save_save_states[i]->setShortcut(QKeyCombination(control, key));
    }
}

bool SuperShuckieMainWindow::try_unload_rom() {
    if(this->game_loaded) {
        this->do_save_game();
    }
    
    this->core = SuperShuckieCore::new_null();
    this->game_loaded = false;
    this->current_rom_name = std::nullopt;
    this->on_rom_switch();
    return true;
}

void SuperShuckieMainWindow::closeEvent(QCloseEvent *event) {
    if(!this->try_unload_rom()) {
        event->ignore();
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

void SuperShuckieMainWindow::on_rom_switch() {
    if(this->game_loaded) {
        this->set_title("Loaded ROM successfully!");
    }
    else {
        this->set_title();
    }
    this->refresh_action_states();
    this->refresh_screen_dimensions();
    this->render_widget->refresh_screen(true);

    if(this->game_loaded && !this->pause->isChecked()) {
        this->core.start();
    }
}
#ifndef __SUPERSHUCKIE_MAIN_WINDOW_HPP_
#define __SUPERSHUCKIE_MAIN_WINDOW_HPP_

#include <QMainWindow>
#include <QTimer>
#include <filesystem>
#include <memory>
#include <chrono>
#include <supershuckie/frontend.h>

class QMenu;
class QAction;
class QCloseEvent;

namespace SuperShuckie64 {

class SuperShuckieRenderWidget;

enum ReplayStatus {
    NoReplay,
    Recording,
    PlayingBack
};

class SuperShuckieMainWindow: public QMainWindow {
    Q_OBJECT
    friend SuperShuckieRenderWidget;
    
public:
    SuperShuckieMainWindow();
    ~SuperShuckieMainWindow();

    void load_rom(const std::filesystem::path &path);

private:
    typedef std::chrono::steady_clock clock;

    void set_title(const char *title = "");
    SuperShuckieRenderWidget *render_widget;
    SuperShuckieFrontendRaw *frontend = nullptr;

    unsigned scale = 6;

    QTimer ticker;

    void tick();

    ReplayStatus replay_status = ReplayStatus::NoReplay;

    void set_up_menu();
    QMenuBar *menu_bar;

    QMenu *file_menu;
    QMenu *gameplay_menu;
    QMenu *save_states_menu;
    QMenu *replays_menu;
    QMenu *settings_menu;

    QMenu *quick_slots;

    QAction *open_rom;
    QAction *close_rom;

    QAction *new_game;
    QAction *save_game;
    QAction *save_new_game;
    QAction *reset_console;
    QAction *pause;
    QAction *quit;

    QAction *record_replay;
    QAction *resume_replay;
    QAction *play_replay;

    QAction *use_number_row_for_quick_slots;

    static const std::size_t QUICK_SAVE_STATE_COUNT = 9;

    QAction *quick_load_save_states[QUICK_SAVE_STATE_COUNT];
    QAction *quick_save_save_states[QUICK_SAVE_STATE_COUNT];

    bool use_number_keys_for_quick_slots = false;

    void set_up_file_menu();
    void set_up_gameplay_menu();
    void set_up_save_states_menu();
    void set_up_replays_menu();
    void set_up_settings_menu();

    void refresh_action_states();
    void set_quick_load_shortcuts();

    void closeEvent(QCloseEvent *event) override;

    bool is_game_running();

    char title_text[128] = {};

    static void on_refresh_screens(void *user_data, std::size_t screen_count, const uint32_t *const *pixels);
    static void on_new_core_metadata(void *user_data, std::size_t screen_count, const SuperShuckieScreenData *screen_data);

    std::uint32_t frames_in_last_second = 0;
    double current_fps = 0.0;
    clock::time_point second_start;
    void refresh_title();


private slots:
    void do_open_rom();
    void do_close_rom();
    void do_new_game();
    void do_save_game();
    void do_save_new_game();
    void do_reset_console();
    void do_toggle_pause();
    void do_toggle_number_row_for_save_states();
    void do_record_replay();
    void do_resume_replay();
    void do_play_replay();
};

}

#endif
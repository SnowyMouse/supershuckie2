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
class SuperShuckieNumberedAction;
class SuperShuckieGameSpeedDialog;

enum ReplayStatus {
    NoReplay,
    Recording,
    PlayingBack
};

class SuperShuckieMainWindow: public QMainWindow {
    Q_OBJECT
    friend SuperShuckieRenderWidget;
    friend SuperShuckieNumberedAction;
    friend SuperShuckieGameSpeedDialog;
    
public:
    SuperShuckieMainWindow();
    ~SuperShuckieMainWindow();

    void load_rom(const std::filesystem::path &path);

private:
    typedef std::chrono::steady_clock clock;

    void set_title(const char *title = "");
    SuperShuckieRenderWidget *render_widget;
    SuperShuckieFrontendRaw *frontend = nullptr;

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
    QAction *undo_load_save_state;
    QAction *redo_load_save_state;

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

    static const std::size_t VIDEO_SCALE_COUNT = 12;

    SuperShuckieNumberedAction *change_video_scale[VIDEO_SCALE_COUNT];

    bool use_number_keys_for_quick_slots = false;

    void set_up_file_menu();
    void set_up_gameplay_menu();
    void set_up_save_states_menu();
    void set_up_replays_menu();
    void set_up_settings_menu();

    void refresh_action_states();
    void set_quick_load_shortcuts();

    void quick_save(std::uint8_t index);
    void quick_load(std::uint8_t index);

    void make_save_state(const char *state);
    void load_save_state(const char *state);

    void set_video_scale(std::uint8_t scale);

    void closeEvent(QCloseEvent *event) override;

    bool is_game_running();

    char title_text[128] = {};

    static void on_refresh_screens(void *user_data, std::size_t screen_count, const uint32_t *const *pixels);
    static void on_change_video_mode(void *user_data, std::size_t screen_count, const SuperShuckieScreenData *screen_data, std::uint8_t scaling);

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
    void do_open_game_speed_dialog() noexcept;
    void do_undo_load_save_state();
    void do_redo_load_save_state();
};

class SuperShuckieNumberedAction: public QAction {
    Q_OBJECT
    friend SuperShuckieMainWindow;
public:
    typedef void (SuperShuckieMainWindow::*on_activated)(std::uint8_t);
    SuperShuckieNumberedAction(SuperShuckieMainWindow *parent, const char *text, std::uint8_t number, on_activated activated);
private:
    std::uint8_t number;
    SuperShuckieMainWindow *parent;
    on_activated activated_fn;
private slots:
    void activated();
};

}

#endif
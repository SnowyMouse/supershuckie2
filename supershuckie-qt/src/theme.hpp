// SPDX-License-Identifier: GPL-3.0-only

#ifndef SIX_SHOOTER_THEME_HPP
#define SIX_SHOOTER_THEME_HPP

#include <QObject>
#include <QPalette>

namespace SixShooter {
    class Theme: public QObject {
        Q_OBJECT
    public:
        Theme();
        QPalette original_palette;
    private slots:
        #ifdef _WIN32
        void set_win32_theme(Qt::ColorScheme);
        #endif
    };
}

#endif

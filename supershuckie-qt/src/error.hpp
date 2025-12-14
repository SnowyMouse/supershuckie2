#ifndef SS64_ERROR_HPP
#define SS64_ERROR_HPP

#include <QMessageBox>
#include <cstring>

#define DISPLAY_ERROR_DIALOG(title, ...) { \
    QMessageBox qmb; \
    qmb.setWindowTitle(title); \
    qmb.setIcon(QMessageBox::Icon::Critical); \
    char ____________error_fmt[1024]; \
    std::snprintf(____________error_fmt, sizeof(____________error_fmt), __VA_ARGS__); \
    qmb.setText(____________error_fmt); \
    qmb.exec(); \
}

#endif

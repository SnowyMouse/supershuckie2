#ifndef SS64_ERROR_HPP
#define SS64_ERROR_HPP

#include <QMessageBox>
#include <cstring>

#define DISPLAY_ERROR_DIALOG(title, ...) { \
    QMessageBox qmb; \
    qmb.setWindowTitle(title); \
    qmb.setIcon(QMessageBox::Icon::Critical); \
    char error[1024]; \
    std::snprintf(error, sizeof(error), __VA_ARGS__); \
    qmb.setText(error); \
    qmb.exec(); \
}

#endif

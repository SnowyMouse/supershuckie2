#ifndef __SUPERSHUCKIE_ASK_FOR_TEXT_DIALOG_HPP__
#define __SUPERSHUCKIE_ASK_FOR_TEXT_DIALOG_HPP__

#include <QDialog>
#include <optional>
#include <string>

class QString;
class QLineEdit;

namespace SuperShuckie64 {

class MainWindow;

class AskForTextDialog: public QDialog {
    Q_OBJECT
public:
    AskForTextDialog(MainWindow *parent, const QString &title, const QString &message, const QString &subtext = "");
    QString text() const;

    int exec() override;

    static std::optional<std::string> ask(MainWindow *parent, const QString &title, const QString &message, const QString &subtext = "");
private:
    QLineEdit *textbox = nullptr;
    MainWindow *parent;
};

}

#endif
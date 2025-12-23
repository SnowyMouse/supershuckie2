#ifndef __SUPERSHUCKIE_SELECT_ITEM_DIALOG_HPP__
#define __SUPERSHUCKIE_SELECT_ITEM_DIALOG_HPP__

#include <QDialog>
#include <optional>
#include <string>
#include <vector>

class QString;
class QListWidget;

namespace SuperShuckie64 {

class MainWindow;

class SelectItemDialog: public QDialog {
    Q_OBJECT
public:
    SelectItemDialog(MainWindow *parent, std::vector<std::string> items, const QString &title, const QString &message, const QString &subtext = "");
    QString text() const;
    int exec() override;
    static std::optional<std::string> ask(MainWindow *parent, std::vector<std::string> items, const QString &title, const QString &message, const QString &subtext = "");
private:
    QListWidget *list = nullptr;
    MainWindow *parent;
};

}

#endif
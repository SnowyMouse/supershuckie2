#include "game_speed_dialog.hpp"
#include "main_window.hpp"

#include <QGridLayout>
#include <QLabel>
#include <QSpinBox>
#include <QPushButton>
#include <QSizePolicy>

using namespace SuperShuckie64;

static void fixup_box(QSpinBox *spinbox) {
    spinbox->setSuffix("%");
    spinbox->setMinimum(25);
    spinbox->setMaximum(25575);
    spinbox->setSingleStep(25);
}

SuperShuckieGameSpeedDialog::SuperShuckieGameSpeedDialog(SuperShuckieMainWindow *parent): QDialog(parent), parent(parent) {
    this->setWindowTitle("Change game speed");

    double turbo = 0.0;
    double base = 0.0;
    supershuckie_frontend_get_speed_settings(parent->frontend, &base, &turbo);

    QGridLayout *layout = new QGridLayout(this);

    layout->addWidget(new QLabel("Base speed", this), 0, 0, Qt::AlignLeft);
    layout->addWidget(new QLabel("Turbo modifier", this), 1, 0, Qt::AlignLeft);

    this->base_speed_slider = new QSpinBox(this);
    this->turbo_speed_slider = new QSpinBox(this);

    layout->addWidget(this->base_speed_slider, 0, 1, Qt::AlignLeft);
    layout->addWidget(this->turbo_speed_slider, 1, 1, Qt::AlignLeft);
    
    this->base_speed_text = new QLabel("= ~9999999 FPS", this);
    this->turbo_speed_text = new QLabel("= ~9999999 FPS", this);

    this->base_speed_text->setFixedSize(this->base_speed_text->sizeHint());
    this->turbo_speed_text->setFixedSize(this->turbo_speed_text->sizeHint());

    fixup_box(this->base_speed_slider);
    fixup_box(this->turbo_speed_slider);

    this->base_speed_slider->setValue(base * 100.0);
    this->turbo_speed_slider->setValue(turbo * 100.0);

    layout->addWidget(this->base_speed_text, 0, 2, Qt::AlignLeft);
    layout->addWidget(this->turbo_speed_text, 1, 2, Qt::AlignLeft);

    this->do_update_speed();

    connect(this->base_speed_slider, SIGNAL(valueChanged(int)), this, SLOT(do_update_speed()));
    connect(this->turbo_speed_slider, SIGNAL(valueChanged(int)), this, SLOT(do_update_speed()));

    layout->setColumnStretch(0, 1);
    layout->setColumnStretch(1, 0);
    layout->setColumnStretch(2, 1);

    QLabel *note = new QLabel("Notes:\n• Turbo speed = base speed × turbo modifier\n• Actual game performance may vary.", this);
    note->setAttribute(Qt::WA_MacSmallSize);
    layout->addWidget(note, 10, 0, 1, 3, Qt::AlignLeft);

    auto *save = new QPushButton("OK", this);
    connect(save, SIGNAL(clicked()), this, SLOT(accept()));
    layout->addWidget(save, 11, 0, 1, 3);
    this->setFixedSize(this->sizeHint());
}

void SuperShuckieGameSpeedDialog::accept() {
    supershuckie_frontend_set_speed_settings(
        this->parent->frontend,
        static_cast<double>(this->base_speed_slider->value()) / 100.0,
        static_cast<double>(this->turbo_speed_slider->value()) / 100.0
    );
    QDialog::accept();
}

void SuperShuckieGameSpeedDialog::do_update_speed() {
    char fmt[256];

    double base_speed = 60.0 * this->base_speed_slider->value() / 100.0;
    std::snprintf(fmt, sizeof(fmt), "= ~%d FPS", static_cast<int>(base_speed));
    this->base_speed_text->setText(fmt);
    std::snprintf(fmt, sizeof(fmt), "= ~%d FPS", static_cast<int>(base_speed * this->turbo_speed_slider->value() / 100.0));
    this->turbo_speed_text->setText(fmt);
}
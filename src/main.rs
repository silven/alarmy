extern crate gtk;
#[macro_use]
extern crate relm;
#[macro_use]
extern crate relm_derive;

use rodio;

use relm::{Relm, Update, Widget};
use gtk::prelude::*;
use gtk::{CssProvider, Window, Button, Inhibit, WindowType};

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

#[derive(Clone)]
struct Alarm {
    active: Arc<AtomicBool>,
}

struct Model {
    alarm: Alarm,
}

impl Alarm {
    fn new() -> Self {
        Alarm {
            active: Arc::new(AtomicBool::from(true)),
        }
    }

    fn is_alarm_on(&self) -> bool {
        self.active.load(Ordering::Relaxed)
    }

    fn invert_alarm(&mut self) {
        self.active.fetch_xor(true, Ordering::Relaxed);
    }
}

#[derive(Msg)]
enum Msg {
    Quit,
}

struct Win {
    window: Window,
}

fn sound_the_alarm() {
    use rodio::{Source, Sink};

    let device = rodio::default_output_device().unwrap();
    let sink = Sink::new(&device);
    sink.pause();
    let source = rodio::source::SineWave::new(440)
        .take_duration(Duration::from_millis(500));

    sink.append(source);

    sink.play();
    sink.sleep_until_end();
}

impl Update for Win {
    type Model = Model;
    type ModelParam = ();
    type Msg = Msg;

    fn model(_relm: &Relm<Self>, _params: Self::ModelParam) -> Model {
        let alarm = Alarm::new();

        let bg_alarm = alarm.clone();
        thread::spawn(move || {
            loop {
                if bg_alarm.is_alarm_on() && !is_power_on() {
                    sound_the_alarm();
                    thread::sleep(Duration::from_millis(200));
                } else {
                    thread::sleep(Duration::from_secs(1));
                }
            }
        });

        Model {
            alarm: alarm,
        }
    }

    fn update(&mut self, event: Msg) {
        match event {
            Msg::Quit => gtk::main_quit(),
        }
    }
}

fn set_button_status(button: &Button, status: bool) {
    let style = button.get_style_context().expect("Could not get style context");
    style.remove_class("active_button");
    style.remove_class("inactive_button");

    if status {
        style.add_class("active_button");
    } else {
        style.add_class("inactive_button");
    }
}

impl Widget for Win {
    type Root = Window;

    fn root(&self) -> Self::Root {
        self.window.clone()
    }

    fn view(relm: &Relm<Self>, model: Self::Model) -> Self {
        let window = Window::new(WindowType::Toplevel);
        window.set_title("Alarmy");
        window.set_position(gtk::WindowPosition::Center);
        window.set_default_size(350, 70);

        let css = "
            .inactive_button {
                font-size: 1em;
                background: green;
            }

            .active_button {
                font-size: 2em;
                background: red;
            }
        ";

        let provider = CssProvider::get_default().expect("Could not get CSSProvider");
        provider.load_from_data(&css.as_bytes()).expect("Could not load CSS from string");

        let the_button = Button::new_with_label("Activate");

        the_button.get_style_context().map(|c|
            c.add_provider(&provider, gtk::STYLE_PROVIDER_PRIORITY_APPLICATION));
        set_button_status(&the_button, model.alarm.is_alarm_on());

        window.add(&the_button);

        connect!(relm, window, connect_delete_event(_, _), return (Some(Msg::Quit), Inhibit(false)));

        // Minor hack to get mutable access inside the move closure
        // (I also know I don't really need mutable access because the AtomicBool methods are borked, but I like it this way)
        let bg_alarm = model.alarm.clone();
        the_button.connect_clicked(move |b| {
            let mut bg_alarm = bg_alarm.clone();
            bg_alarm.invert_alarm();
            set_button_status(&b, bg_alarm.is_alarm_on());
        });

        window.show_all();

        Win {
            window: window,
        }
    }
}

enum PowerError {
    Io(std::io::Error),
    Utf8(std::string::FromUtf8Error)
}

fn get_power() -> Result<bool, PowerError> {
    use std::process::Command;
    Command::new("acpi")
        .arg("-a")
        .output()
        .map_err(|e| PowerError::Io(e))
        .and_then(|output|
            String::from_utf8(output.stdout)
                .map_err(|e| PowerError::Utf8(e)))
        .map(|line|
            line.contains("on-line"))
}

fn is_power_on() -> bool {
    get_power().ok().unwrap_or(false)
}

fn main() {
    Win::run(()).unwrap();
}
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::time::{Duration, Instant};
use gtk::prelude::*;
use gtk::{glib, CssProvider};
use gtk::gdk::Display;
use gtk4_layer_shell::{Edge, Layer, LayerShell};
use libpulse_binding as pulse;
use libpulse_glib_binding as pulse_glib;
use pulse::callbacks::ListResult;
use pulse::context::subscribe::{Facility, InterestMaskSet};
use pulse::context::{Context, FlagSet as CtxFlagSet, State as CtxState};
use pulse::proplist::Proplist;

const DEBOUNCE: Duration = Duration::from_millis(500);
const RECONNECT_DELAY: Duration = Duration::from_secs(3);
const SLIDE_DISTANCE: i32 = 340;
const SLIDE_DURATION_SECS: f64 = 0.3;

struct Notifier {
    window: gtk::ApplicationWindow,
    fade_running: Cell<bool>,
    last_muted: Cell<bool>,
    last_trigger: Cell<Option<Instant>>,
}

fn slide_in(window: &gtk::ApplicationWindow) {
    window.set_margin(Edge::Right, -SLIDE_DISTANCE);
    let start = Instant::now();
    let window = window.clone();
    window.add_tick_callback(move |win, _clock| {
        let t = (start.elapsed().as_secs_f64() / SLIDE_DURATION_SECS).min(1.0);
        let eased = 1.0 - (1.0 - t).powi(3);
        let margin = (-SLIDE_DISTANCE as f64 + eased * (SLIDE_DISTANCE as f64 + 20.0)) as i32;
        win.set_margin(Edge::Right, margin);
        if t >= 1.0 {
            win.set_margin(Edge::Right, 20);
            glib::ControlFlow::Break
        } else {
            glib::ControlFlow::Continue
        }
    });
}

impl Notifier {
    fn trigger(self: &Rc<Self>) {
        if self.fade_running.get() {
            return; // not so many notifications
        }
        if let Some(last) = self.last_trigger.get() {
            if last.elapsed() < DEBOUNCE {
                return;
            }
        }
        self.last_trigger.set(Some(Instant::now()));
        self.fade_running.set(true);
        self.window.set_opacity(1.0);
        slide_in(&self.window);

        let this = Rc::clone(self);
        glib::timeout_add_local_once(Duration::from_secs(2), move || {
            let opacity = Rc::new(Cell::new(1.0_f64));
            let this2 = Rc::clone(&this);
            this.window.add_tick_callback(move |win, _clock| {
                let o = (opacity.get() - 0.03).max(0.0);
                opacity.set(o);
                win.set_opacity(o);
                if o <= 0.0 {
                    this2.fade_running.set(false);
                    glib::ControlFlow::Break
                } else {
                    glib::ControlFlow::Continue
                }
            });
        });
    }
}

fn subscribe_sink_changes(context: &Rc<RefCell<Context>>, notifier: &Rc<Notifier>) {
    let context_query = Rc::clone(context);
    let notifier_cb = Rc::clone(notifier);

    context.borrow_mut().set_subscribe_callback(Some(Box::new(move |facility, _op, index| {
        if facility != Some(Facility::Sink) {
            return;
        }
        let notifier = Rc::clone(&notifier_cb);
        context_query.borrow_mut().introspect().get_sink_info_by_index(index, move |result| {
            if let ListResult::Item(info) = result {
                let muted = info.mute;
                if muted && !notifier.last_muted.get() {
                    notifier.trigger();
                }
                notifier.last_muted.set(muted);
            }
        });
    })));

    context.borrow_mut().subscribe(InterestMaskSet::SINK, |_success| {});
}

fn connect_pulse(notifier: Rc<Notifier>) {
    let mut proplist = Proplist::new().unwrap();
    proplist.set_str(pulse::proplist::properties::APPLICATION_NAME, "OhNoSound").unwrap();

    let mainloop: &'static pulse_glib::Mainloop =
        Box::leak(Box::new(pulse_glib::Mainloop::new(None).expect("failed to create glib mainloop")));

    let context = Rc::new(RefCell::new(
        Context::new_with_proplist(mainloop, "OhNoSoundContext", &proplist)
            .expect("failed to create pulse context"),
    ));
    context.borrow_mut().connect(None, CtxFlagSet::NOFLAGS, None)
        .expect("failed to connect to pulseaudio");

    let context_state = Rc::clone(&context);
    let notifier_state = Rc::clone(&notifier);
    context.borrow_mut().set_state_callback(Some(Box::new(move || {
        let state = context_state.borrow().get_state();
        match state {
            CtxState::Ready => subscribe_sink_changes(&context_state, &notifier_state),
            CtxState::Failed | CtxState::Terminated => {
                let context_retry = Rc::clone(&context_state);
                glib::timeout_add_local_once(RECONNECT_DELAY, move || {
                    let _ = context_retry.borrow_mut().connect(None, CtxFlagSet::NOFLAGS, None);
                });
            }
            _ => {}
        }
    })));
}

fn load_css() {
    let provider = CssProvider::new();
    provider.load_from_data(
        "
        window { background-color: transparent; }
        #notif-box {
            background-color: rgba(20, 20, 20, 0.9);
            border-radius: 12px;
            padding: 10px 16px;
        }
        #notif-label { color: white; font-size: 14px; font-weight: bold; }
        #notif-icon { border-radius: 8px; }
        "
    );
    gtk::style_context_add_provider_for_display(
        &Display::default().expect("no display"),
        &provider,
        gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
    );
}

fn main() {
    let app = gtk::Application::new(Some("dev.jacobballnarch.ohnosound"), Default::default());
    app.connect_activate(|app| {
        load_css();

        let window = gtk::ApplicationWindow::new(app);
        window.init_layer_shell();
        window.set_layer(Layer::Overlay);
        window.set_anchor(Edge::Top, true);
        window.set_anchor(Edge::Right, true);
        window.set_margin(Edge::Top, 45); // panel height
        window.set_margin(Edge::Right, -SLIDE_DISTANCE);
        window.set_exclusive_zone(-1);
        window.set_can_target(false); // one stupid line blocks me a firefox buttons kekw

        let image = gtk::Image::from_file(concat!(env!("CARGO_MANIFEST_DIR"), "/assets/icon.jpg"));
        image.set_pixel_size(48);
        image.set_widget_name("notif-icon");
        image.set_overflow(gtk::Overflow::Hidden);
        let label = gtk::Label::new(Some("Oh, now there is no sound ♪"));
        label.set_widget_name("notif-label");
        let hbox = gtk::Box::new(gtk::Orientation::Horizontal, 10);
        hbox.set_widget_name("notif-box");
        hbox.append(&image);
        hbox.append(&label);

        window.set_child(Some(&hbox));
        window.set_default_size(300, 70);
        window.set_resizable(false);

        window.connect_realize(|win| {
            if let Some(surface) = win.surface() {
                let empty_region = gtk::cairo::Region::create();
                surface.set_input_region(&empty_region);
            }
        });
        window.present();

        let notifier = Rc::new(Notifier {
            window: window.clone(),
            fade_running: Cell::new(false),
            last_muted: Cell::new(false),
            last_trigger: Cell::new(None),
        });

        connect_pulse(notifier);
    });
    app.run();
}
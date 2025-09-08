use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use enigo::{Enigo, Mouse, Settings};
use gpui::prelude::*;
use gpui::*;

use objc::msg_send;
use objc::runtime::{Class, Object};
use objc::sel;
use objc::sel_impl;
use serde::{Deserialize, Serialize};

#[repr(u64)]
enum ActivationPolicy {
    Accessory = 1,
}

#[allow(unexpected_cfgs)]
fn set_activation_policy(policy: ActivationPolicy) {
    let ns_app_class = Class::get("NSApplication").unwrap();
    let ns_app: *mut Object = unsafe { msg_send![ns_app_class, sharedApplication] };

    unsafe {
        let _: () = msg_send![ns_app, setActivationPolicy: policy as u64];
    }
}

struct Dock {
    config: Config,
    is_visible: Arc<AtomicBool>,
    mouse_thread_started: bool,
    window_handle: Option<AnyWindowHandle>,
}

#[derive(Debug, Deserialize, Serialize)]
struct Config {
    dock_height: f32,
    dock_width: f32,
    apps: Vec<DockApp>,
}

impl Config {
    pub fn load_config() -> Self {
        let home_dir = std::env::var("HOME").expect("Error, HOME env var not set.");
        let config_file = std::fs::read_to_string(home_dir + "/.config/docky/apps.json").expect("Unable to read config file.");
        serde_json::from_str(&config_file).expect("Invalid json syntax")
    }
}

#[derive(Debug, Deserialize, Serialize)]
struct DockApp {
    name: String,
    icon: PathBuf,
}

pub fn window_options() -> WindowOptions {
    let mut window_options = WindowOptions::default();
    window_options.titlebar = None;
    window_options.window_decorations = None;
    window_options.window_background = WindowBackgroundAppearance::Blurred;
    window_options.is_movable = false;
    window_options.kind = WindowKind::PopUp;
    window_options
}



impl Dock {
    fn new() -> Self {
        let config = Config::load_config();
        Self {
            config,
            is_visible: Arc::new(AtomicBool::new(false)),
            window_handle: None,
            mouse_thread_started: false,
        }
    }

    fn start_mouse_monitoring_thread(&mut self, screen_height: f32, cx: &mut Context<Self>) {
        if self.mouse_thread_started {
            return;
        }

        self.mouse_thread_started = true;
        let is_visible = Arc::clone(&self.is_visible);

        // Spawn background task that runs on a separate thread
        cx.background_executor()
            .spawn(async move {
                // Create Enigo instance in the background thread
                let enigo = match Enigo::new(&Settings::default()) {
                    Ok(e) => e,
                    Err(_) => return, // Exit if we can't create Enigo
                };

                loop {
                    let should_show = match enigo.location() {
                        Ok(location) => {
                            let mouse_y = location.1 as f32;
                            // Show dock if mouse is within specified distance from bottom
                            if is_visible.load(Ordering::Relaxed) {
                                mouse_y >= (screen_height - 100.0) // Hide when mouse moves away
                            } else {
                                mouse_y >= (screen_height - 20.0) // Show when mouse is close to bottom
                            }
                        }
                        Err(_) => false,
                    };

                    // Update the atomic boolean
                    is_visible.store(should_show, Ordering::Relaxed);

                    // Sleep for 50ms
                    Timer::after(Duration::from_millis(50)).await;
                }
            })
            .detach();

        // Start a UI timer to check the atomic boolean and update window visibility
        cx.spawn(async move |dock_entity, cx| {
            let mut last_visible_state = false;

            loop {
                let mut current_visible = false;
                let mut window_handle_opt = None;

                if let Some(dock) = dock_entity.upgrade() {
                    cx.update_entity(&dock, |dock, _cx| {
                        current_visible = dock.is_visible.load(Ordering::Relaxed);
                        window_handle_opt = dock.window_handle;
                    })
                    .ok();
                }

                // Only update if visibility state changed
                if current_visible != last_visible_state {
                    last_visible_state = current_visible;

                    if let Some(window_handle) = window_handle_opt {
                        if current_visible {
                            // Show the window
                            cx.update_window(window_handle, |_, window, _| {
                                window.activate_window();
                            })
                            .ok();
                        }
                    }

                    // Notify for re-render
                    if let Some(dock) = dock_entity.upgrade() {
                        cx.update_entity(&dock, |_, cx| {
                            cx.notify();
                        })
                        .ok();
                    }
                }

                Timer::after(Duration::from_millis(16)).await;
            }
        })
        .detach();
    }
}

impl DockApp {
    fn launch_app(&self) {
        let app_name = self.name.clone();
        std::thread::spawn(move || {
            let _ = Command::new("open").arg("-a").arg(&app_name).output();
        });
    }
}

impl Render for Dock {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // Store window handle if we don't have it
        if self.window_handle.is_none() {
            self.window_handle = Some(window.window_handle());
        }

        let dock_width = px(self.config.dock_width);
        let dock_height = px(self.config.dock_height);

        let dock_size = Size {
            width: dock_width,
            height: dock_height,
        };

        if self.window_handle.is_none() {
            self.window_handle = Some(window.window_handle());
        }

        // Start monitoring thread if not already started
        if !self.mouse_thread_started {
            let screen_height = window.display(cx).unwrap().bounds().size.height.0;
            self.start_mouse_monitoring_thread(screen_height, cx);
        }

        // Check if dock should be visible
        let is_visible = self.is_visible.load(Ordering::Relaxed);

        if !is_visible {
            window.resize(Size {
                width: px(1.),
                height: px(1.),
            });
            // Return a completely transparent/empty element when hidden
            return div().size_full().absolute().opacity(0.0);
        } else {
            window.resize(dock_size);
        }

        // Set window properties when visible

        let dock = div()
            .flex()
            .size_full()
            .justify_center()
            .items_center()
            .text_xl()
            .text_color(rgb(0xffffff))
            .rounded_xl()
            .children(self.config.apps.iter().enumerate().map(|(index, app)| {
                let icon_path = app.icon.clone();

                div()
                    .p_3()
                    .m(px(-5.))
                    .rounded_lg()
                    .cursor_pointer()
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(move |dock, _event, _cx, _| {
                            if let Some(dock_app) = dock.config.apps.get(index) {
                                dock_app.launch_app();
                            }
                        }),
                    )
                    .child(
                        img(icon_path.clone())
                            .size_11()
                            .rounded_md()
                            .with_fallback(|| {
                                div()
                                    .size_12()
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .rounded_md()
                                    .text_sm()
                                    .text_color(rgb(0x888888))
                                    .child("?")
                                    .into_any_element()
                            }),
                    )
            }));

        dock
    }
}

fn main() {
    Application::new().run(|cx: &mut App| {
        let dock = Dock::new();

        let config = &dock.config;

        let displays = cx.displays();
        let primary_display = &displays[0];

        let dock_width = px(config.dock_width);
        let dock_height = px(config.dock_height);
        let margin_from_bottom = px(0.0);

        let dock_size = Size {
            width: dock_width,
            height: dock_height,
        };

        let dock_bounds = Bounds {
            origin: point(
                primary_display.bounds().center().x - (dock_size.width / 2.),
                primary_display.bounds().size.height - dock_height - margin_from_bottom,
            ),
            size: dock_size,
        };
        let mut window_opts = window_options();
        window_opts.window_bounds = Some(WindowBounds::Windowed(dock_bounds));
        window_opts.display_id = Some(primary_display.id());

        cx.open_window(window_opts, |_, cx| cx.new(|_cx| dock))
            .unwrap();

        set_activation_policy(ActivationPolicy::Accessory);
    });
}

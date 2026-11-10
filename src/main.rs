use gtk::{gio, glib, prelude::*};
use webkit2gtk::{NavigationPolicyDecisionExt, PolicyDecisionExt, URIRequestExt, WebViewExt, SettingsExt, UserContentManagerExt};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    glib::set_prgname(Some("roadmap-app"));
    if std::env::args().any(|a| a == "--serve") {
        use std::io::Write;
        let app = roadmap_app::App::new(|_| String::new())?;
        println!("{}", app.serve()?);
        std::io::stdout().flush()?;
        loop { std::thread::park(); }
    }
    gtk::init()?;
    let win = gtk::Window::new(gtk::WindowType::Toplevel);
    win.set_title("Roadmap");
    let titlebar = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    // A hidden title bar widget keeps GTK's own shadow and resize edges but takes no height;
    // a visible empty one still gets the theme's title bar height.
    titlebar.set_no_show_all(true);
    win.set_titlebar(Some(&titlebar));
    titlebar.hide();
    win.set_default_size(1280, 840);
    win.set_icon_name(Some("roadmap-app"));
    // Server threads send picker requests to the GTK main thread and wait for its answer.
    let (tx, rx) = std::sync::mpsc::channel::<(String, std::sync::mpsc::SyncSender<String>)>();
    let parent = win.clone();
    glib::timeout_add_local(std::time::Duration::from_millis(20), move || {
        while let Ok((kind, answer)) = rx.try_recv() {
            let folder = kind == "folder";
            let dlg = gtk::FileChooserNative::new(
                Some(if folder {
                    "Choose the project folder"
                } else {
                    "Choose a roadmap file"
                }),
                Some(&parent),
                if folder {
                    gtk::FileChooserAction::SelectFolder
                } else {
                    gtk::FileChooserAction::Open
                },
                None,
                None,
            );
            let got = if dlg.run() == gtk::ResponseType::Accept {
                dlg.filename()
                    .map(|p| p.to_string_lossy().into_owned())
                    .unwrap_or_default()
            } else {
                String::new()
            };
            dlg.destroy();
            let _ = answer.send(got);
        }
        glib::ControlFlow::Continue
    });
    let app = roadmap_app::App::new(move |kind| {
        let (answer, rx) = std::sync::mpsc::sync_channel(1);
        if tx.send((kind.into(), answer)).is_err() {
            return String::new();
        }
        rx.recv().unwrap_or_default()
    })?;
    let url = app.serve()?;
    let base = url.clone();
    let manager = webkit2gtk::UserContentManager::new();
    manager.register_script_message_handler("roadmapWindow");
    let view = webkit2gtk::WebView::with_user_content_manager(&manager);
    let settings = webkit2gtk::Settings::new();
    settings.set_hardware_acceleration_policy(webkit2gtk::HardwareAccelerationPolicy::Always);
    settings.set_enable_smooth_scrolling(true);
    view.set_settings(&settings);
    let window = win.clone();
    manager.connect_script_message_received(Some("roadmapWindow"), move |_, result| {
        let Some(value) = result.js_value() else { return };
        let message = value.to_string();
        let parts: Vec<_> = message.split(',').collect();
        match parts[0] {
            "minimise" => window.iconify(),
            "maximise" => if window.is_maximized() { window.unmaximize() } else { window.maximize() },
            "close" => window.close(),
            "move" if parts.len() == 4 => {
                if let (Ok(x), Ok(y), Ok(time)) = (parts[1].parse(), parts[2].parse(), parts[3].parse()) {
                    window.begin_move_drag(1, x, y, time);
                }
            }
            _ => {}
        }
    });
    let page = view.clone();
    win.connect_window_state_event(move |window, _| {
        page.evaluate_javascript(&format!("window.roadmapMaximised?.({})", window.is_maximized()), None, None, None::<&gio::Cancellable>, |_| {});
        glib::Propagation::Proceed
    });
    view.connect_decide_policy(move |_, decision, kind| {
        if !matches!(
            kind,
            webkit2gtk::PolicyDecisionType::NavigationAction
                | webkit2gtk::PolicyDecisionType::NewWindowAction
        ) {
            return false;
        }
        if let Some(d) = decision.downcast_ref::<webkit2gtk::NavigationPolicyDecision>() {
            if let Some(a) = d.navigation_action() {
                let clicked = kind == webkit2gtk::PolicyDecisionType::NewWindowAction
                    || a.navigation_type() == webkit2gtk::NavigationType::LinkClicked;
                if let Some(uri) = a.request().and_then(|r| r.uri()) {
                    if clicked
                        && (uri.starts_with("http://") || uri.starts_with("https://"))
                        && !uri.starts_with(&base)
                    {
                        let _ = gio::AppInfo::launch_default_for_uri(
                            &uri,
                            None::<&gio::AppLaunchContext>,
                        );
                        decision.ignore();
                        return true;
                    }
                }
            }
        }
        false
    });
    view.load_uri(&url);
    win.add(&view);
    win.connect_destroy(|_| gtk::main_quit());
    win.show_all();
    gtk::main();
    Ok(())
}

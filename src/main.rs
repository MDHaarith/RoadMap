use gtk::{gio, glib, prelude::*};
use webkit2gtk::{NavigationPolicyDecisionExt, PolicyDecisionExt, URIRequestExt, WebViewExt};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    glib::set_prgname(Some("roadmap-app"));
    gtk::init()?;
    let win = gtk::Window::new(gtk::WindowType::Toplevel);
    win.set_title("Roadmap");
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
    let view = webkit2gtk::WebView::new();
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

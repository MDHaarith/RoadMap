use roadmap_app::harness;

// With no argument this is the MCP server. The installers run it with `connect`, `disconnect` or `install-opencode`.
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let app = roadmap_app::App::new(|_| String::new())?;
    let off = harness::off_marker(&app.data);
    match std::env::args().nth(1).as_deref() {
        Some(how @ ("connect" | "disconnect")) => {
            let dirs = harness::Dirs::here().ok_or("No home folder.")?;
            let mcp = std::env::current_exe()?;
            if how == "connect" {
                let _ = std::fs::remove_file(&off);
            } else {
                std::fs::write(&off, b"")?;
            }
            for (name, r) in harness::connect(&dirs, (how == "connect").then_some(mcp.as_path())) {
                match r {
                    Ok(changed) => println!("{name}: {}", if !changed { "nothing to change" } else if how == "connect" { "connected" } else { "disconnected" }),
                    Err(why) => println!("{name}: skipped, {why}"),
                }
            }
        }
        Some("install-opencode") => println!("OpenCode: {}", harness::install_opencode()?.display()),
        _ => roadmap_app::mcp::serve(&app, std::io::stdin().lock(), std::io::stdout().lock())?,
    }
    Ok(())
}

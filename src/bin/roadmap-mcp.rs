fn main() -> Result<(), Box<dyn std::error::Error>> {
    let app = roadmap_app::App::new(|_| String::new())?;
    roadmap_app::mcp::serve(&app, std::io::stdin().lock(), std::io::stdout().lock())?;
    Ok(())
}

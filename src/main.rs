use onyx::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    app::App::new().run()?;
    Ok(())
}

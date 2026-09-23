use onyx::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();
    app::App::new().run()?;
    Ok(())
}

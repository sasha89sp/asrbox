use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    let config = slint_build::CompilerConfiguration::new().with_style("fluent".to_owned());
    slint_build::compile_with_config("ui/app.slint", config)?;
    Ok(())
}

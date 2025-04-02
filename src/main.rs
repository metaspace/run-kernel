use anyhow::Result;

mod command;
mod config;
mod run;

fn main() -> Result<()> {
    let args = config::get_config()?;

    if args.print_config {
        println!("{:#?}", &args);
        return Ok(());
    }

    let config = &args.config;
    run::run_kernel(&config.run_config)
}

use anyhow::Result;

mod bringup;
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

    if config.bringup {
        bringup::bring_up(config)
    } else {
        run::run_kernel(&config.run_config)
    }
}

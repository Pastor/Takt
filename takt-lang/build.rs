extern crate lalrpop;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    unsafe {
        std::env::set_var("LALRPOP_LANE_TABLE", "disabled");
    }
    lalrpop::Configuration::new()
        .always_use_colors()
        // .use_cargo_dir_conventions()
        .emit_rerun_directives(true)
        .process_current_dir()?;
    Ok(())
}

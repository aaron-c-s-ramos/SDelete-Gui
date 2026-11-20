#![windows_subsystem = "windows"]

use sdelete_gui::command::parse_runner_args;
use sdelete_gui::gui::run_gui;
use sdelete_gui::runner::execute_runner;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();

    if let Some(request) = parse_runner_args(&args) {
        let code = execute_runner(&request);
        std::process::exit(code);
    }

    if let Err(err) = run_gui() {
        eprintln!("Failed to launch SDelete Gui: {err}");
        std::process::exit(1);
    }
}

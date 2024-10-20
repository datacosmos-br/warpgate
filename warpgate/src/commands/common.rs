use std::io::{self, IsTerminal};

use tracing::{error, info};

pub fn assert_interactive_terminal() {
    let stdin = io::stdin();

    if !stdin.is_terminal() {
        error!("Please run this command from an interactive terminal.");
        if is_docker() {
            info!("(have you forgotten `-it`?)");
        }
        std::process::exit(1);
    }
}

pub fn is_docker() -> bool {
    std::env::var("DOCKER").is_ok()
}

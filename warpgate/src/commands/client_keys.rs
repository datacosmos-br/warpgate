use anyhow::Result;
use warpgate_protocol_ssh::helpers::PublicKeyAsOpenSSH;

use crate::config::load_config;

pub async fn command(cli: &crate::Cli) -> Result<()> {
    let config = load_config(&cli.config, true)?;
    let keys = warpgate_protocol_ssh::load_client_keys(&config)?;
    println!("Warpgate SSH client keys:");
    println!("(add these to your target's authorized_keys file)");
    println!();
    for key in keys {
        println!("{}", key.as_openssh());
    }
    Ok(())
}

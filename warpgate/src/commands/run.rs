use std::path::PathBuf;

use anyhow::Result;
use futures::StreamExt;
#[cfg(target_os = "linux")]
use sd_notify::NotifyState;
use tracing::*;
use warpgate_common::db::cleanup_db;
use warpgate_common::logging::install_database_logger;
use warpgate_common::{ProtocolServer, Services};
use warpgate_protocol_http::HTTPProtocolServer;
use warpgate_protocol_mysql::MySQLProtocolServer;
use warpgate_protocol_ssh::SSHProtocolServer;

use crate::config::{load_config, watch_config};

pub(crate) async fn command(cli: &crate::Cli) -> Result<()> {
    let version = env!("CARGO_PKG_VERSION");
    info!(%version, "Warpgate");

    let config = load_config(&cli.config, true)?;
    let services = Services::new(config.clone()).await?;

    install_database_logger(services.db.clone());

    let mut protocol_futures = futures::stream::FuturesUnordered::new();

    if config.store.ssh.enable {
        protocol_futures.push(
            SSHProtocolServer::new(&services)
                .await?
                .run(*config.store.ssh.listen),
        );
    }

    if config.store.http.enable {
        protocol_futures.push(
            HTTPProtocolServer::new(&services)
                .await?
                .run(*config.store.http.listen),
        );
    }

    if config.store.mysql.enable {
        protocol_futures.push(
            MySQLProtocolServer::new(&services)
                .await?
                .run(*config.store.mysql.listen),
        );
    }

    tokio::spawn({
        let services = services.clone();
        async move {
            loop {
                let retention = { services.config.lock().await.store.log.retention };
                let interval = retention / 10;
                #[allow(clippy::explicit_auto_deref)]
                match cleanup_db(&mut *services.db.lock().await, &retention).await {
                    Err(error) => error!(?error, "Failed to cleanup the database"),
                    Ok(_) => debug!("Database cleaned up, next in {:?}", interval),
                }
                tokio::time::sleep(interval).await;
            }
        }
    });

    if console::user_attended() {
        info!("--------------------------------------------");
        info!("Warpgate is now running.");
        if config.store.ssh.enable {
            info!("Accepting SSH connections on {:?}", config.store.ssh.listen);
        }
        if config.store.http.enable {
            info!(
                "Accepting HTTP connections on https://{:?}",
                config.store.http.listen
            );
        }
        if config.store.mysql.enable {
            info!(
                "Accepting MySQL connections on {:?}",
                config.store.mysql.listen
            );
        }
        info!("--------------------------------------------");
    }

    #[cfg(target_os = "linux")]
    if let Ok(true) = sd_notify::booted() {
        use std::time::Duration;
        tokio::spawn(async {
            if let Err(error) = async {
                sd_notify::notify(false, &[NotifyState::Ready])?;
                loop {
                    sd_notify::notify(false, &[NotifyState::Watchdog])?;
                    tokio::time::sleep(Duration::from_secs(15)).await;
                }
                #[allow(unreachable_code)]
                Ok::<(), anyhow::Error>(())
            }
            .await
            {
                error!(?error, "Failed to communicate with systemd");
            }
        });
    }

    drop(config);

    if protocol_futures.is_empty() {
        anyhow::bail!("No protocols are enabled in the config file, exiting");
    }

    tokio::spawn(watch_config_and_reload(
        PathBuf::from(&cli.config),
        services.clone(),
    ));

    loop {
        tokio::select! {
            _ = tokio::signal::ctrl_c() => {
                std::process::exit(1);
            }
            result = protocol_futures.next() => {
                match result {
                    Some(Err(error)) => {
                        error!(?error, "Server error");
                        std::process::exit(1);
                    },
                    None => break,
                    _ => (),
                }
            }
        }
    }

    info!("Exiting");
    Ok(())
}

pub async fn watch_config_and_reload(path: PathBuf, services: Services) -> Result<()> {
    let mut reload_event = watch_config(path, services.config.clone())?;

    while let Ok(()) = reload_event.recv().await {
        let state = services.state.lock().await;
        let mut cp = services.config_provider.lock().await;
        for (id, session) in state.sessions.iter() {
            let mut session = session.lock().await;
            if let (Some(username), Some(target)) =
                (session.username.as_ref(), session.target.as_ref())
            {
                if !cp.authorize_target(username, &target.name).await? {
                    warn!(sesson_id=%id, %username, target=&target.name, "Session no longer authorized after config reload");
                    session.handle.close();
                }
            }
        }
    }

    Ok(())
}

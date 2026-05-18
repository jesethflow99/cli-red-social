use anyhow::Result;
use clap::Parser;
use std::sync::Arc;

mod app;
mod db;
mod models;
mod ssh;
mod theme;

#[derive(Parser)]
#[command(name = "cli-red-social")]
struct Cli {
    #[arg(long)]
    tui: bool,

    #[arg(long, default_value = "2222")]
    port: u16,

    #[arg(long, default_value = "postgres://social:social@localhost/social")]
    db: String,

    #[arg(long, default_value = "host_key")]
    key: String,
}

fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .init();
    let cli = Cli::parse();

    let db_url = std::env::var("DATABASE_URL").unwrap_or_else(|_| cli.db);

    if cli.tui {
        app::run_tui(&db_url)?;
    } else {
        println!("Iniciando servidor SSH en puerto {}...", cli.port);
        let database = Arc::new(db::Database::new(&db_url)?);
        database.cleanup_old_data(90).ok();
        spawn_cleanup_thread(database.clone());
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()?;
        runtime.block_on(run_server(
            database,
            db_url,
            cli.port,
            cli.key,
        ))?;
    }

    Ok(())
}

async fn run_server(db: Arc<db::Database>, db_url: String, port: u16, key: String) -> Result<()> {
    let mut server = ssh::SshServer::new(db, &db_url);
    server.run(port, &key).await?;
    Ok(())
}

fn spawn_cleanup_thread(db: Arc<db::Database>) {
    std::thread::spawn(move || loop {
        std::thread::sleep(std::time::Duration::from_secs(86400));
        match db.cleanup_old_data(90) {
            Ok((m, n)) => tracing::info!("Cleanup: {} mensajes, {} notificaciones eliminados", m, n),
            Err(e) => tracing::error!("Error en cleanup: {}", e),
        }
    });
}

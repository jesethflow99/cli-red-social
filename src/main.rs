use anyhow::Result;
use clap::Parser;
use std::sync::Arc;

mod app;
mod db;
mod firewall;
mod i18n;
mod models;
mod plugins;
mod ssh;
mod theme;

#[derive(Parser)]
#[command(name = "agora")]
struct Cli {
    #[arg(long)]
    tui: bool,

    #[arg(long, default_value = "2222")]
    port: u16,

    #[arg(long, default_value = "postgres://social:agora@localhost/social")]
    db: String,

    #[arg(long, default_value = "host_key")]
    key: String,

    #[arg(long)]
    seed: bool,

    #[arg(long)]
    export: bool,

    #[arg(long)]
    user: Option<String>,

    #[arg(long, default_value = "json")]
    format: String,

    #[arg(long, default_value = "")]
    log: String,
}

fn setup_logging(log_file: &str, stderr: bool) {
    if log_file.is_empty() {
        if stderr {
            tracing_subscriber::fmt()
                .with_writer(std::io::stderr)
                .with_max_level(tracing::Level::INFO)
                .init();
        }
        return;
    }

    let file = match std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(log_file)
    {
        Ok(f) => f,
        Err(e) => {
            if stderr {
                eprintln!("No se pudo abrir archivo de log: {}", e);
            }
            return;
        }
    };

    tracing_subscriber::fmt()
        .with_writer(std::sync::Mutex::new(file))
        .with_max_level(tracing::Level::DEBUG)
        .init();
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    setup_logging(&cli.log, !cli.tui && !cli.export);

    let db_url = std::env::var("DATABASE_URL").unwrap_or_else(|_| cli.db);
    let ssh_password = std::env::var("SSH_PASSWORD").unwrap_or_else(|_| "agora".to_string());

    if cli.export {
        let database = db::Database::new(&db_url)?;
        let username = cli.user.as_deref().unwrap_or("");
        if username.is_empty() {
            anyhow::bail!("Debes especificar --user <username> para exportar");
        }
        let result = database.export_user_data(username)?;
        println!("Export exitoso: {}", result);
        println!("Descarga con: scp -P 2222 localhost:{} .", result);
        return Ok(());
    }

    if cli.seed {
        let database = db::Database::new(&db_url)?;
        database.seed_data()?;
        println!("Datos de ejemplo insertados.");
        return Ok(());
    }

    if cli.tui {
        app::run_tui(&db_url)?;
    } else {
        println!("Iniciando servidor SSH en puerto {}...", cli.port);
        if !ssh_password.is_empty() {
            println!("Autenticación SSH por contraseña habilitada.");
        } else {
            println!("⚠  SSH_PASSWORD no configurada. Usando contraseña por defecto: \"agora\". Configurá SSH_PASSWORD para producción.");
        }
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
            &ssh_password,
        ))?;
    }

    Ok(())
}

async fn run_server(db: Arc<db::Database>, db_url: String, port: u16, key: String, ssh_password: &str) -> Result<()> {
    let mut server = ssh::SshServer::new(db, &db_url, ssh_password);
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
        match db.cleanup_inactive_users(730) {
            Ok(n) if n > 0 => tracing::info!("Cleanup: {} cuentas inactivas eliminadas", n),
            Ok(_) => {}
            Err(e) => tracing::error!("Error en cleanup de cuentas: {}", e),
        }
    });
}

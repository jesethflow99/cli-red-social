use anyhow::Result;
use clap::Parser;
use std::sync::Arc;

mod app;
mod db;
mod firewall;
mod i18n;
mod models;
mod plugins;
mod shard;
mod ssh;
mod theme;

#[derive(Parser)]
#[command(name = "agora")]
struct Cli {
    #[arg(long)]
    tui: bool,

    #[arg(long, default_value = "2222")]
    port: u16,

    #[arg(long, default_value = "agora.db")]
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

    /// Genera una invitación de un solo uso y muestra el código una sola vez.
    #[arg(long)]
    invite_create: bool,

    /// Días de validez para --invite-create.
    #[arg(long, default_value_t = 7)]
    invite_days: i64,

    /// Lista invitaciones sin revelar sus códigos.
    #[arg(long)]
    invite_list: bool,

    /// Revoca una invitación pendiente mediante su código.
    #[arg(long)]
    invite_revoke: Option<String>,

    /// Fusiona todos los shards del mesh en un único archivo principal.
    #[arg(long)]
    condense: bool,
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

    if cli.condense {
        let n = shard::shard_count();
        if n <= 1 {
            println!("No hay shards que condensar (AGORA_DB_SHARDS={}).", n);
            return Ok(());
        }
        let sm = shard::ShardManager::new(&db_url, n)?;
        let result = sm.condense()?;
        println!("Mesh condensado en: {}", result);
        return Ok(());
    }

    let admin_actions = usize::from(cli.invite_create)
        + usize::from(cli.invite_list)
        + usize::from(cli.invite_revoke.is_some());
    if admin_actions > 1 {
        anyhow::bail!("Usa una sola acción de invitaciones a la vez.");
    }
    if admin_actions == 1 {
        let database = shard::open_database(&db_url)?;
        if cli.invite_create {
            let code = database.create_invitation(cli.invite_days)?;
            println!("Invitación creada (válida {} días):", cli.invite_days);
            println!("{}", code);
            println!("El código no puede consultarse de nuevo.");
        } else if cli.invite_list {
            println!("ID\tESTADO\tEXPIRA\tUSUARIO");
            for (id, _created, expires, used, username) in database.list_invitations()? {
                let state = if used { "usada" } else { "pendiente" };
                println!(
                    "{}\t{}\t{}\t{}",
                    id,
                    state,
                    expires,
                    username.unwrap_or_else(|| "-".to_string())
                );
            }
        } else if let Some(code) = cli.invite_revoke.as_deref() {
            if database.revoke_invitation(code)? {
                println!("Invitación revocada.");
            } else {
                anyhow::bail!("La invitación no existe o ya fue utilizada.");
            }
        }
        return Ok(());
    }

    if cli.export {
        let database = shard::open_database(&db_url)?;
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
        let database = shard::open_database(&db_url)?;
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
            println!(
                "⚠  SSH_PASSWORD no configurada. Usando contraseña por defecto: \"agora\". Configurá SSH_PASSWORD para producción."
            );
        }
        let database: Arc<dyn db::DatabaseOps> = Arc::from(shard::open_database(&db_url)?);
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

async fn run_server(
    db: Arc<dyn db::DatabaseOps>,
    db_url: String,
    port: u16,
    key: String,
    ssh_password: &str,
) -> Result<()> {
    let mut server = ssh::SshServer::new(db, &db_url, ssh_password);
    server.run(port, &key).await?;
    Ok(())
}

fn spawn_cleanup_thread(db: Arc<dyn db::DatabaseOps>) {
    std::thread::spawn(move || {
        loop {
            std::thread::sleep(std::time::Duration::from_secs(86400));
            match db.cleanup_old_data(90) {
                Ok((m, n)) => {
                    tracing::info!("Cleanup: {} mensajes, {} notificaciones eliminados", m, n)
                }
                Err(e) => tracing::error!("Error en cleanup: {}", e),
            }
            match db.cleanup_inactive_users(730) {
                Ok(n) if n > 0 => tracing::info!("Cleanup: {} cuentas inactivas eliminadas", n),
                Ok(_) => {}
                Err(e) => tracing::error!("Error en cleanup de cuentas: {}", e),
            }
        }
    });
}

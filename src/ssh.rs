use std::io::Write;
use std::os::fd::AsRawFd;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use anyhow::Result;
use async_trait::async_trait;
use nix::pty::{forkpty, ForkptyResult};
use nix::sys::wait::waitpid;
use nix::unistd::{close, write};
use russh::keys::key::KeyPair;
use russh::keys::load_secret_key;
use russh::server::*;
use russh::{Channel, ChannelId, CryptoVec};

use crate::db::Database;

fn load_or_generate_key(path: &str) -> Result<KeyPair> {
    if std::path::Path::new(path).exists() {
        return Ok(load_secret_key(path, None)?);
    }

    let key = KeyPair::generate_ed25519();
    match std::fs::File::create(path) {
        Ok(file) => {
            let mut writer = std::io::BufWriter::new(file);
            russh_keys::encode_pkcs8_pem(&key, &mut writer)?;
            writer.flush()?;
            println!("Clave SSH generada y guardada en {}", path);
        }
        Err(e) if e.kind() == std::io::ErrorKind::PermissionDenied => {
            let fallback = "/tmp/host_key";
            let file = std::fs::File::create(fallback)?;
            let mut writer = std::io::BufWriter::new(file);
            russh_keys::encode_pkcs8_pem(&key, &mut writer)?;
            writer.flush()?;
            tracing::warn!(
                "Sin permisos para escribir clave SSH en '{}', usando fallback '{}'",
                path,
                fallback
            );
        }
        Err(e) => return Err(e.into()),
    }

    Ok(key)
}

pub struct SshServer {
    _db: Arc<Database>,
    db_conn: String,
    ssh_password: String,
}

impl SshServer {
    pub fn new(db: Arc<Database>, db_conn: &str, ssh_password: &str) -> Self {
        Self { _db: db, db_conn: db_conn.to_string(), ssh_password: ssh_password.to_string() }
    }

    pub async fn run(&mut self, port: u16, key_path: &str) -> Result<()> {
        let key = load_or_generate_key(key_path)?;
        let config = Config {
            inactivity_timeout: Some(std::time::Duration::from_secs(3600)),
            auth_rejection_time: std::time::Duration::from_secs(3),
            auth_rejection_time_initial: Some(std::time::Duration::from_secs(0)),
            keys: vec![key],
            ..Default::default()
        };
        self.run_on_address(Arc::new(config), ("0.0.0.0", port))
            .await?;
        Ok(())
    }
}

impl Server for SshServer {
    type Handler = SshSession;

    fn new_client(&mut self, _: Option<std::net::SocketAddr>) -> SshSession {
        SshSession {
            authed: false,
            input_tx: None,
            input_rx: None,
            channel_id: None,
            master_fd: Arc::new(Mutex::new(None)),
            db_conn: self.db_conn.clone(),
            ws_col: 80,
            ws_row: 24,
            ws_xpixel: 0,
            ws_ypixel: 0,
            ssh_password: self.ssh_password.clone(),
        }
    }
}

pub struct SshSession {
    authed: bool,
    input_tx: Option<tokio::sync::mpsc::UnboundedSender<Vec<u8>>>,
    input_rx: Option<tokio::sync::mpsc::UnboundedReceiver<Vec<u8>>>,
    channel_id: Option<ChannelId>,
    master_fd: Arc<Mutex<Option<i32>>>,
    db_conn: String,
    ws_col: u32,
    ws_row: u32,
    ws_xpixel: u32,
    ws_ypixel: u32,
    ssh_password: String,
}

#[async_trait]
impl Handler for SshSession {
    type Error = anyhow::Error;

    async fn auth_password(&mut self, _user: &str, password: &str) -> Result<Auth, Self::Error> {
        if self.ssh_password.is_empty() || password == self.ssh_password {
            self.authed = true;
            Ok(Auth::Accept)
        } else {
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            Ok(Auth::Reject { proceed_with_methods: None })
        }
    }

    async fn auth_publickey(&mut self, _: &str, _: &russh::keys::key::PublicKey) -> Result<Auth, Self::Error> {
        Ok(Auth::Reject { proceed_with_methods: None })
    }

    async fn channel_open_session(
        &mut self,
        channel: Channel<Msg>,
        _: &mut Session,
    ) -> Result<bool, Self::Error> {
        if !self.authed {
            return Ok(false);
        }
        let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
        self.input_tx = Some(tx);
        self.input_rx = Some(rx);
        self.channel_id = Some(channel.id());
        Ok(true)
    }

    async fn data(
        &mut self,
        _: ChannelId,
        data: &[u8],
        _: &mut Session,
    ) -> Result<(), Self::Error> {
        if let Some(tx) = &self.input_tx {
            let _ = tx.send(data.to_vec());
        }
        Ok(())
    }

    async fn shell_request(
        &mut self,
        channel_id: ChannelId,
        session: &mut Session,
    ) -> Result<(), Self::Error> {
        session.channel_success(channel_id);
        let handle = session.handle();
        let mut rx = match self.input_rx.take() {
            Some(rx) => rx,
            None => return Ok(()),
        };

        let master_fd = self.master_fd.clone();
        let db_conn = self.db_conn.clone();
        let ws_col = self.ws_col;
        let ws_row = self.ws_row;
        let ws_xpixel = self.ws_xpixel;
        let ws_ypixel = self.ws_ypixel;

        tokio::task::spawn_blocking(move || {
            let fork_result = unsafe { forkpty(None, None) }.unwrap();

            match fork_result {
                ForkptyResult::Child => {
                    drop(handle);
                    let _ = std::panic::catch_unwind(|| {
                        if let Err(e) = crate::app::run_tui(&db_conn) {
                            eprintln!("Error iniciando TUI: {e}");
                        }
                    });
                    std::process::exit(0);
                }
                ForkptyResult::Parent { master, child } => {
                    let fd = master.as_raw_fd();
                    *master_fd.lock().unwrap() = Some(fd);
                    let ws = nix::pty::Winsize {
                        ws_col: ws_col as u16,
                        ws_row: ws_row as u16,
                        ws_xpixel: ws_xpixel as u16,
                        ws_ypixel: ws_ypixel as u16,
                    };
                    unsafe { nix::libc::ioctl(fd, nix::libc::TIOCSWINSZ, &ws); }
                    let child_exited = Arc::new(AtomicBool::new(false));

                    let h1 = handle.clone();
                    let cid = channel_id;
                    let exit_flag = child_exited.clone();
                    std::thread::spawn(move || {
                        let mut buf = vec![0u8; 4096];
                        loop {
                            match nix::unistd::read(fd, &mut buf) {
                                Ok(0) | Err(_) => {
                                    exit_flag.store(true, Ordering::SeqCst);
                                    break;
                                }
                                Ok(n) => {
                                    let data: CryptoVec = buf[..n].to_vec().into();
                                    let _ = futures::executor::block_on(h1.data(cid, data));
                                }
                            }
                        }
                    });

                    use tokio::runtime::Handle;
                    let rt = Handle::current();
                    loop {
                        if child_exited.load(Ordering::SeqCst) {
                            break;
                        }
                        match rt.block_on(async {
                            tokio::time::timeout(
                                std::time::Duration::from_millis(200),
                                rx.recv(),
                            )
                            .await
                        }) {
                            Ok(Some(data)) => {
                                let _ = write(&master, &data);
                            }
                            Ok(None) => break,
                            Err(_) => {}
                        }
                    }

                    *master_fd.lock().unwrap() = None;
                    let _ = waitpid(child, None);
                    let _ = close(fd);
                    let _ = futures::executor::block_on(handle.close(channel_id));
                }
            }
        });

        Ok(())
    }

    async fn pty_request(
        &mut self,
        channel: ChannelId,
        _: &str,
        col_width: u32,
        row_height: u32,
        pix_width: u32,
        pix_height: u32,
        _: &[(russh::Pty, u32)],
        session: &mut Session,
    ) -> Result<(), Self::Error> {
        self.ws_col = col_width;
        self.ws_row = row_height;
        self.ws_xpixel = pix_width;
        self.ws_ypixel = pix_height;
        session.channel_success(channel);
        Ok(())
    }

    async fn window_change_request(
        &mut self,
        _: ChannelId,
        col: u32,
        row: u32,
        xpixel: u32,
        ypixel: u32,
        _: &mut Session,
    ) -> Result<(), Self::Error> {
        if let Some(fd) = *self.master_fd.lock().unwrap() {
            let ws = nix::pty::Winsize {
                ws_col: col as u16,
                ws_row: row as u16,
                ws_xpixel: xpixel as u16,
                ws_ypixel: ypixel as u16,
            };
            unsafe { nix::libc::ioctl(fd, nix::libc::TIOCSWINSZ, &ws); }
        }
        Ok(())
    }

    async fn channel_eof(
        &mut self,
        _: ChannelId,
        _: &mut Session,
    ) -> Result<(), Self::Error> {
        self.input_tx = None;
        Ok(())
    }

    async fn channel_close(
        &mut self,
        _: ChannelId,
        _: &mut Session,
    ) -> Result<(), Self::Error> {
        self.input_tx = None;
        Ok(())
    }
}

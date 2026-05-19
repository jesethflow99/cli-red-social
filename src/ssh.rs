use std::collections::HashMap;
use std::io::Write;
use std::os::fd::AsRawFd;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::sync::Mutex;

use anyhow::Result;
use async_trait::async_trait;
use image::GenericImageView;
use nix::pty::{forkpty, ForkptyResult};
use nix::sys::wait::waitpid;
use nix::unistd::{close, write};
use russh::keys::key::KeyPair;
use russh::keys::load_secret_key;
use russh::server::*;
use russh::{Channel, ChannelId, CryptoVec};
use russh_sftp::protocol::{
    Attrs, Data, File, FileAttributes, Handle, Name, OpenFlags, Status, StatusCode, Version,
};

use crate::db::Database;

const UPLOAD_DIR: &str = "/data/uploads";
const MAX_IMAGE_SIZE: u64 = 10 * 1024 * 1024; // 10MB
const MAX_IMAGE_DIM: u32 = 512;

struct OpenFile {
    path: std::path::PathBuf,
    file: Arc<Mutex<std::fs::File>>,
    is_write: bool,
}

#[derive(Default)]
struct SftpSession {
    version: Option<u32>,
    open_files: HashMap<String, OpenFile>,
    open_dirs: HashMap<String, std::path::PathBuf>,
    handle_counter: AtomicU64,
    root_dir_read_done: bool,
}

impl SftpSession {
    fn next_handle(&self) -> String {
        let id = self.handle_counter.fetch_add(1, Ordering::SeqCst);
        format!("handle_{}", id)
    }

    fn status_ok(id: u32) -> Status {
        Status {
            id,
            status_code: StatusCode::Ok,
            error_message: "Ok".to_string(),
            language_tag: "en-US".to_string(),
        }
    }
}

fn sanitize_path(path: &str) -> Option<std::path::PathBuf> {
    let path = path.trim_start_matches('/');
    if path.is_empty() || path == "." {
        return Some(std::path::PathBuf::from(UPLOAD_DIR));
    }
    let full = std::path::PathBuf::from(UPLOAD_DIR).join(path);
    tracing::debug!("sanitize_path: path={}, full={}", path, full.display());
    if full.exists() {
        let canonical = full.canonicalize().ok()?;
        return if canonical.starts_with(UPLOAD_DIR) { Some(canonical) } else { None };
    }
    if let Some(parent) = full.parent() {
        tracing::debug!("sanitize_path: parent={}, exists={}", parent.display(), parent.exists());
        let _ = std::fs::create_dir_all(parent);
        if parent.exists() {
            let parent_canon = parent.canonicalize().ok()?;
            tracing::debug!("sanitize_path: parent_canon={}", parent_canon.display());
            return if parent_canon.starts_with(UPLOAD_DIR) { Some(full) } else { None };
        }
    }
    tracing::warn!("sanitize_path: returning None for {}", path);
    None
}

fn process_image(path: &std::path::Path) {
    let img = match image::open(path) {
        Ok(i) => i,
        Err(_) => {
            tracing::warn!("Uploaded file is not a valid image, deleting: {}", path.display());
            let _ = std::fs::remove_file(path);
            return;
        }
    };

    let (w, h) = img.dimensions();
    if w <= MAX_IMAGE_DIM && h <= MAX_IMAGE_DIM {
        return;
    }

    let ratio = (MAX_IMAGE_DIM as f32) / (w.max(h) as f32);
    let new_w = (w as f32 * ratio).round() as u32;
    let new_h = (h as f32 * ratio).round() as u32;

    let resized = img.resize(new_w, new_h, image::imageops::FilterType::Lanczos3);

    let jpeg_path = path.with_extension("jpg");
    let mut buf = Vec::new();
    let mut cursor = std::io::Cursor::new(&mut buf);
    if resized.write_to(&mut cursor, image::ImageFormat::Jpeg).is_ok() {
        let _ = std::fs::write(&jpeg_path, &buf);
        if jpeg_path != path {
            let _ = std::fs::remove_file(path);
        }
        tracing::info!("Image processed: {} -> {} ({}x{}, {} bytes)",
            path.display(), jpeg_path.display(), new_w, new_h, buf.len());
    }
}

pub fn list_uploaded_images() -> Vec<(String, String)> {
    let _ = std::fs::create_dir_all(UPLOAD_DIR);
    let mut images = Vec::new();
    if let Ok(entries) = std::fs::read_dir(UPLOAD_DIR) {
        for entry in entries.flatten() {
            let path = entry.path();
            if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                if matches!(ext.to_lowercase().as_str(), "jpg" | "jpeg" | "png" | "gif" | "webp") {
                    if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                        let size = path.metadata().map(|m| m.len()).unwrap_or(0);
                        images.push((name.to_string(), format!("{:.1} KB", size as f64 / 1024.0)));
                    }
                }
            }
        }
    }
    images.sort_by(|a, b| b.0.cmp(&a.0));
    images
}

fn attrs_from_meta(meta: &std::fs::Metadata) -> FileAttributes {
    FileAttributes::from(meta)
}

impl russh_sftp::server::Handler for SftpSession {
    type Error = StatusCode;

    fn unimplemented(&self) -> Self::Error {
        StatusCode::OpUnsupported
    }

    async fn init(
        &mut self,
        version: u32,
        extensions: HashMap<String, String>,
    ) -> Result<Version, Self::Error> {
        if self.version.is_some() {
            tracing::warn!("duplicate SSH_FXP_VERSION packet");
            return Err(StatusCode::ConnectionLost);
        }
        self.version = Some(version);
        tracing::info!("SFTP version: {:?}, extensions: {:?}", self.version, extensions);
        Ok(Version::new())
    }

    async fn open(
        &mut self,
        id: u32,
        filename: String,
        pflags: OpenFlags,
        _attrs: FileAttributes,
    ) -> Result<Handle, Self::Error> {
        let _ = std::fs::create_dir_all(UPLOAD_DIR);

        let p = sanitize_path(&filename).ok_or(StatusCode::PermissionDenied)?;

        let write = pflags.contains(OpenFlags::WRITE);
        let read = pflags.contains(OpenFlags::READ);

        let file = if write {
            std::fs::File::create(&p).map_err(|_| StatusCode::Failure)?
        } else if read {
            std::fs::File::open(&p).map_err(|_| StatusCode::NoSuchFile)?
        } else {
            std::fs::File::open(&p)
                .or_else(|_| std::fs::File::create(&p))
                .map_err(|_| StatusCode::Failure)?
        };

        let handle = self.next_handle();
        self.open_files.insert(handle.clone(), OpenFile {
            path: p,
            file: Arc::new(Mutex::new(file)),
            is_write: write,
        });

        Ok(Handle { id, handle })
    }

    async fn close(
        &mut self,
        id: u32,
        handle: String,
    ) -> Result<Status, Self::Error> {
        if let Some(open_file) = self.open_files.remove(&handle) {
            if open_file.is_write {
                let path = open_file.path.clone();
                tokio::task::spawn_blocking(move || {
                    process_image(&path);
                });
            }
        }
        Ok(Self::status_ok(id))
    }

    async fn read(
        &mut self,
        id: u32,
        handle: String,
        offset: u64,
        len: u32,
    ) -> Result<Data, Self::Error> {
        use std::io::{Read, Seek};
        let open_file = self.open_files.get(&handle).ok_or(StatusCode::BadMessage)?;
        let mut file = open_file.file.lock().unwrap();
        file.seek(std::io::SeekFrom::Start(offset)).map_err(|_| StatusCode::Failure)?;
        let mut buf = vec![0u8; len as usize];
        let n = file.read(&mut buf).map_err(|_| StatusCode::Failure)?;
        buf.truncate(n);
        Ok(Data { id, data: buf })
    }

    async fn write(
        &mut self,
        id: u32,
        handle: String,
        offset: u64,
        data: Vec<u8>,
    ) -> Result<Status, Self::Error> {
        use std::io::{Seek, Write};
        let open_file = self.open_files.get(&handle).ok_or(StatusCode::BadMessage)?;
        let mut file = open_file.file.lock().unwrap();
        file.seek(std::io::SeekFrom::Start(offset)).map_err(|_| StatusCode::Failure)?;

        let current_pos = offset + data.len() as u64;
        if current_pos > MAX_IMAGE_SIZE {
            return Err(StatusCode::Failure);
        }

        file.write_all(&data).map_err(|_| StatusCode::Failure)?;
        Ok(Self::status_ok(id))
    }

    async fn lstat(
        &mut self,
        id: u32,
        path: String,
    ) -> Result<Attrs, Self::Error> {
        let p = sanitize_path(&path).ok_or(StatusCode::PermissionDenied)?;
        let meta = std::fs::metadata(&p).map_err(|_| StatusCode::NoSuchFile)?;
        Ok(Attrs {
            id,
            attrs: attrs_from_meta(&meta),
        })
    }

    async fn fstat(
        &mut self,
        id: u32,
        handle: String,
    ) -> Result<Attrs, Self::Error> {
        let open_file = self.open_files.get(&handle).ok_or(StatusCode::BadMessage)?;
        let file = open_file.file.lock().unwrap();
        let meta = file.metadata().map_err(|_| StatusCode::Failure)?;
        Ok(Attrs {
            id,
            attrs: attrs_from_meta(&meta),
        })
    }

    async fn opendir(
        &mut self,
        id: u32,
        path: String,
    ) -> Result<Handle, Self::Error> {
        let p = sanitize_path(&path).ok_or(StatusCode::PermissionDenied)?;
        let _ = std::fs::read_dir(&p).map_err(|_| StatusCode::NoSuchFile)?;

        let handle = self.next_handle();
        self.open_dirs.insert(handle.clone(), p);
        self.root_dir_read_done = false;

        Ok(Handle { id, handle })
    }

    async fn readdir(
        &mut self,
        id: u32,
        handle: String,
    ) -> Result<Name, Self::Error> {
        let dir_path = self.open_dirs.get(&handle).ok_or(StatusCode::BadMessage)?.clone();

        let entries = std::fs::read_dir(&dir_path).map_err(|_| StatusCode::NoSuchFile)?;
        let mut files = Vec::new();
        for entry in entries.flatten() {
            if let Ok(meta) = entry.metadata() {
                let name = entry.file_name().to_string_lossy().to_string();
                let attrs = attrs_from_meta(&meta);
                files.push(File::new(&name, attrs));
            }
        }

        if files.is_empty() {
            return Err(StatusCode::Eof);
        }

        Ok(Name { id, files })
    }

    async fn remove(
        &mut self,
        id: u32,
        filename: String,
    ) -> Result<Status, Self::Error> {
        let p = sanitize_path(&filename).ok_or(StatusCode::PermissionDenied)?;
        std::fs::remove_file(&p).map_err(|_| StatusCode::Failure)?;
        Ok(Self::status_ok(id))
    }

    async fn mkdir(
        &mut self,
        id: u32,
        path: String,
        _attrs: FileAttributes,
    ) -> Result<Status, Self::Error> {
        let p = sanitize_path(&path).ok_or(StatusCode::PermissionDenied)?;
        std::fs::create_dir(&p).map_err(|_| StatusCode::Failure)?;
        Ok(Self::status_ok(id))
    }

    async fn rmdir(
        &mut self,
        id: u32,
        path: String,
    ) -> Result<Status, Self::Error> {
        let p = sanitize_path(&path).ok_or(StatusCode::PermissionDenied)?;
        std::fs::remove_dir(&p).map_err(|_| StatusCode::Failure)?;
        Ok(Self::status_ok(id))
    }

    async fn realpath(
        &mut self,
        id: u32,
        path: String,
    ) -> Result<Name, Self::Error> {
        let p = sanitize_path(&path).unwrap_or_else(|| std::path::PathBuf::from(UPLOAD_DIR));
        Ok(Name {
            id,
            files: vec![File::dummy(p.to_string_lossy().to_string())],
        })
    }

    async fn stat(
        &mut self,
        id: u32,
        path: String,
    ) -> Result<Attrs, Self::Error> {
        let p = sanitize_path(&path).ok_or(StatusCode::PermissionDenied)?;
        let meta = std::fs::metadata(&p).map_err(|_| StatusCode::NoSuchFile)?;
        Ok(Attrs {
            id,
            attrs: attrs_from_meta(&meta),
        })
    }

    async fn rename(
        &mut self,
        id: u32,
        oldpath: String,
        newpath: String,
    ) -> Result<Status, Self::Error> {
        let old_p = sanitize_path(&oldpath).ok_or(StatusCode::PermissionDenied)?;
        let new_p = sanitize_path(&newpath).ok_or(StatusCode::PermissionDenied)?;
        std::fs::rename(&old_p, &new_p).map_err(|_| StatusCode::Failure)?;
        Ok(Self::status_ok(id))
    }
}

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
            clients: Arc::new(Mutex::new(HashMap::new())),
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
    clients: Arc<Mutex<HashMap<ChannelId, Channel<Msg>>>>,
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
        {
            let mut clients = self.clients.lock().unwrap();
            clients.insert(channel.id(), channel);
        }
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

    async fn subsystem_request(
        &mut self,
        channel_id: ChannelId,
        name: &str,
        session: &mut Session,
    ) -> Result<(), Self::Error> {
        if name == "sftp" {
            let channel = {
                let mut clients = self.clients.lock().unwrap();
                clients.remove(&channel_id).unwrap()
            };
            let sftp = SftpSession::default();
            session.channel_success(channel_id);
            russh_sftp::server::run(channel.into_stream(), sftp).await;
        } else {
            session.channel_failure(channel_id);
        }
        Ok(())
    }
}

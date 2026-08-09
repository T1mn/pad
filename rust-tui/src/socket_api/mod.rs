mod cli {
    use super::client::send_request;
    use super::model::ApiRequest;
    use std::error::Error;

    pub fn run_args(args: impl IntoIterator<Item = String>) -> Result<(), Box<dyn Error>> {
        let args: Vec<String> = args.into_iter().collect();
        let request = match args.first().map(String::as_str) {
            Some("request") => {
                let raw = args.get(1).ok_or("missing request json")?;
                serde_json::from_str::<ApiRequest>(raw)?
            }
            Some("status") => ApiRequest {
                action: "status".into(),
                ..ApiRequest::default()
            },
            Some("inbox") => ApiRequest {
                action: "inbox".into(),
                ..ApiRequest::default()
            },
            Some(other) => return Err(format!("unknown socket-api command: {other}").into()),
            None => {
                return Err("usage: pad __internal socket-api status|inbox|request <json>".into())
            }
        };
        let response = send_request(&request)?;
        println!("{}", serde_json::to_string_pretty(&response)?);
        Ok(())
    }
}
pub(crate) mod client {
    use super::model::{ApiRequest, ApiResponse};
    use std::io::{self, BufRead, BufReader, Write};
    use std::os::unix::net::UnixStream;

    pub fn send_request(request: &ApiRequest) -> io::Result<ApiResponse> {
        let mut stream = UnixStream::connect(crate::paths::api_socket_path())?;
        let encoded = serde_json::to_string(request)?;
        stream.write_all(encoded.as_bytes())?;
        stream.write_all(b"\n")?;
        let mut line = String::new();
        BufReader::new(stream).read_line(&mut line)?;
        serde_json::from_str::<ApiResponse>(&line).map_err(|err| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("invalid API response: {err}"),
            )
        })
    }
}
pub(crate) mod handler {
    mod core {
        use super::super::model::{ApiRequest, ApiResponse};
        use serde_json::json;

        pub(super) fn status_response() -> ApiResponse {
            ApiResponse::ok(
                "ok",
                Some(json!({
                    "runtime": "native",
                    "online": crate::runtime_status::read_status(&crate::paths::pad_status_path())
                        .is_some_and(|status| crate::runtime_status::process_alive(status.pid)),
                })),
            )
        }

        pub(super) fn inbox_response() -> ApiResponse {
            let inbox = crate::notification_inbox::load();
            ApiResponse::ok(
                "ok",
                Some(json!({
                    "unread": inbox.unread_count(),
                    "entries": inbox.entries,
                })),
            )
        }

        pub(super) fn mark_read_response(request: ApiRequest) -> ApiResponse {
            let Some(id) = request.id.as_deref() else {
                return ApiResponse::err("missing id");
            };
            match crate::notification_inbox::mark_read(id) {
                Ok(changed) => ApiResponse::ok("ok", Some(json!({ "changed": changed }))),
                Err(err) => ApiResponse::err(format!("mark_read failed: {err}")),
            }
        }

        pub(super) fn prompt_response(request: ApiRequest) -> ApiResponse {
            let Some(pane_id) = request.pane_id.as_deref() else {
                return ApiResponse::err("missing pane_id");
            };
            let Some(prompt) = request.prompt.as_deref() else {
                return ApiResponse::err("missing prompt");
            };
            if request.dry_run {
                return ApiResponse::ok(
                    "dry_run",
                    Some(json!({ "pane_id": pane_id, "prompt_len": prompt.chars().count() })),
                );
            }
            ApiResponse::err(format!(
                "native pane {pane_id} is owned by the PAD UI; prompt dispatch requires the in-app terminal"
            ))
        }
    }
    mod remote {
        use super::super::model::{ApiRequest, ApiResponse};
        use serde_json::json;

        pub(super) fn browser_open_response(request: ApiRequest) -> ApiResponse {
            let Some(url) = request.url.as_deref() else {
                return ApiResponse::err("missing url");
            };
            if request.dry_run {
                return match crate::browser_remote::browser_open_command(url) {
                    Ok(command) => ApiResponse::ok(
                        "dry_run",
                        Some(json!({ "program": command.program, "args": command.args })),
                    ),
                    Err(err) => ApiResponse::err(format!("browser command failed: {err}")),
                };
            }
            match crate::browser_remote::open_browser_url(url) {
                Ok(()) => ApiResponse::ok("opened", None),
                Err(err) => ApiResponse::err(format!("browser open failed: {err}")),
            }
        }

        pub(crate) fn remote_exec_command(
            request: &ApiRequest,
        ) -> Result<Vec<String>, ApiResponse> {
            let Some(host) = request.host.as_deref() else {
                return Err(ApiResponse::err("missing host"));
            };
            let Some(command) = request.command.as_deref() else {
                return Err(ApiResponse::err("missing command"));
            };
            crate::browser_remote::remote_ssh_command(
                &crate::browser_remote::RemoteCommandRequest {
                    host: host.to_string(),
                    cwd: request.cwd.clone(),
                    command: command.to_string(),
                },
            )
            .map_err(|err| ApiResponse::err(format!("invalid host: {err}")))
        }

        pub(super) fn remote_exec_response(request: ApiRequest) -> ApiResponse {
            let ssh = match remote_exec_command(&request) {
                Ok(ssh) => ssh,
                Err(response) => return response,
            };
            if request.dry_run {
                return ApiResponse::ok("dry_run", Some(json!({ "command": ssh })));
            }
            ApiResponse::err("live remote exec requires the async socket server")
        }
    }

    use super::model::{ApiRequest, ApiResponse};
    use core::{inbox_response, mark_read_response, prompt_response, status_response};
    pub(crate) use remote::remote_exec_command;
    use remote::{browser_open_response, remote_exec_response};

    pub fn handle_request(request: ApiRequest) -> ApiResponse {
        match request.action.as_str() {
            "status" => status_response(),
            "inbox" => inbox_response(),
            "mark_read" => mark_read_response(request),
            "prompt" => prompt_response(request),
            "browser_open" => browser_open_response(request),
            "remote_exec" => remote_exec_response(request),
            other => ApiResponse::err(format!("unknown action: {other}")),
        }
    }
}
pub(crate) mod model {
    use serde::{Deserialize, Serialize};
    use serde_json::Value;

    #[derive(Clone, Debug, Default, Serialize, Deserialize)]
    pub struct ApiRequest {
        pub action: String,
        #[serde(default)]
        pub pane_id: Option<String>,
        #[serde(default)]
        pub prompt: Option<String>,
        #[serde(default)]
        pub id: Option<String>,
        #[serde(default)]
        pub name: Option<String>,
        #[serde(default)]
        pub session_id: Option<String>,
        #[serde(default)]
        pub url: Option<String>,
        #[serde(default)]
        pub host: Option<String>,
        #[serde(default)]
        pub cwd: Option<String>,
        #[serde(default)]
        pub command: Option<String>,
        #[serde(default)]
        pub dry_run: bool,
    }

    #[derive(Clone, Debug, Serialize, Deserialize)]
    pub struct ApiResponse {
        pub ok: bool,
        pub message: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub data: Option<Value>,
    }

    impl ApiResponse {
        pub fn ok(message: impl Into<String>, data: Option<Value>) -> Self {
            Self {
                ok: true,
                message: message.into(),
                data,
            }
        }

        pub fn err(message: impl Into<String>) -> Self {
            Self {
                ok: false,
                message: message.into(),
                data: None,
            }
        }
    }
}
pub(crate) mod peer {
    use std::io;
    use std::os::fd::{AsRawFd, RawFd};

    /// socket API 能执行 ssh / 向原生终端注入 prompt / 拉起进程，所以只接受和本进程同一个
    /// uid 的本地连接；root 也不特殊放行（root 本来就能绕过一切，无需在这里开口子）。
    pub(super) fn peer_uid_is_allowed(peer_uid: u32, owner_uid: u32) -> bool {
        peer_uid == owner_uid
    }

    pub(super) fn current_uid() -> u32 {
        // SAFETY: geteuid 无参数、无副作用，总是成功。
        unsafe { libc::geteuid() as u32 }
    }

    /// 校验通过返回对端 uid，否则返回错误；调用方必须直接关闭连接。
    pub(crate) fn authorize_peer(stream: &impl AsRawFd) -> io::Result<u32> {
        let owner_uid = current_uid();
        let peer_uid = peer_uid(stream.as_raw_fd())?;
        if !peer_uid_is_allowed(peer_uid, owner_uid) {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                format!("peer uid {peer_uid} != owner uid {owner_uid}"),
            ));
        }
        Ok(peer_uid)
    }

    #[cfg(target_os = "linux")]
    fn peer_uid(fd: RawFd) -> io::Result<u32> {
        let mut cred = libc::ucred {
            pid: 0,
            uid: 0,
            gid: 0,
        };
        let mut len = std::mem::size_of::<libc::ucred>() as libc::socklen_t;
        // SAFETY: cred/len 都是本地变量，长度与 SO_PEERCRED 期望的结构体一致。
        let rc = unsafe {
            libc::getsockopt(
                fd,
                libc::SOL_SOCKET,
                libc::SO_PEERCRED,
                std::ptr::addr_of_mut!(cred).cast(),
                &mut len,
            )
        };
        if rc != 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(cred.uid)
    }

    #[cfg(not(target_os = "linux"))]
    fn peer_uid(fd: RawFd) -> io::Result<u32> {
        let mut uid: libc::uid_t = 0;
        let mut gid: libc::gid_t = 0;
        // SAFETY: 两个出参都是本地变量，getpeereid 只写入它们。
        let rc = unsafe { libc::getpeereid(fd, &mut uid, &mut gid) };
        if rc != 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(uid as u32)
    }
}
mod server;
pub(crate) mod socket_file {
    use std::io;
    use std::os::unix::fs::PermissionsExt;
    use std::os::unix::net::{UnixListener as StdUnixListener, UnixStream as StdUnixStream};
    use std::path::{Path, PathBuf};

    const SOCKET_MODE: u32 = 0o600;
    const SOCKET_DIR_MODE: u32 = 0o700;

    pub(crate) fn socket_is_live(socket_path: &Path) -> bool {
        StdUnixStream::connect(socket_path).is_ok()
    }

    /// 同步绑定 socket 并交回 listener：所在目录收到 0700，socket 文件 0600。
    ///
    /// 走 "bind 临时名 -> chmod 0600 -> link 到正式名"，没有选另外两种做法：
    /// - "bind 正式名后再 chmod" 有一段 0755 的窗口；
    /// - "bind 前翻 umask" 是进程级副作用，pad 有一堆 tokio 线程在并发建文件，会误伤它们；
    /// - 用 link 而不是 rename，因为 rename 会原子覆盖掉另一个实例正在监听的 socket，
    ///   而 link 碰到已存在的正式名直接 EEXIST，这一步本身就是所有权判据，
    ///   不需要先 exists() 再动手，也就没有 TOCTOU。
    pub(crate) fn bind_private_listener(socket_path: &Path) -> io::Result<StdUnixListener> {
        harden_socket_dir(socket_path.parent().unwrap_or_else(|| Path::new(".")))?;
        match bind_via_staging(socket_path) {
            Err(err) if link_unsupported(&err) => bind_in_place(socket_path),
            result => result,
        }
    }

    fn bind_via_staging(socket_path: &Path) -> io::Result<StdUnixListener> {
        let staging = staging_path(socket_path);
        let _ = std::fs::remove_file(&staging);
        let listener = StdUnixListener::bind(&staging)?;
        let published = publish_socket(&staging, socket_path);
        // 正式名是硬链接，临时名用完就删；listener 仍然指向同一个 inode。
        let _ = std::fs::remove_file(&staging);
        published?;
        listener.set_nonblocking(true)?;
        Ok(listener)
    }

    /// 少数文件系统不给 socket 建硬链接时的退路：直接 bind 正式名再 chmod。
    /// 此时 `~/.pad` 已经是 0700，窗口期内别的用户也走不进这个目录。
    fn bind_in_place(socket_path: &Path) -> io::Result<StdUnixListener> {
        let listener = match StdUnixListener::bind(socket_path) {
            Ok(listener) => listener,
            Err(err) if err.kind() == io::ErrorKind::AddrInUse => {
                reclaim_stale_socket(socket_path)?;
                StdUnixListener::bind(socket_path)?
            }
            Err(err) => return Err(err),
        };
        set_mode(socket_path, SOCKET_MODE)?;
        listener.set_nonblocking(true)?;
        Ok(listener)
    }

    fn publish_socket(staging: &Path, socket_path: &Path) -> io::Result<()> {
        set_mode(staging, SOCKET_MODE)?;
        match std::fs::hard_link(staging, socket_path) {
            Err(err) if err.kind() == io::ErrorKind::AlreadyExists => {
                reclaim_stale_socket(socket_path)?;
                std::fs::hard_link(staging, socket_path)
            }
            result => result,
        }
    }

    fn reclaim_stale_socket(socket_path: &Path) -> io::Result<()> {
        if socket_is_live(socket_path) {
            return Err(io::Error::new(
                io::ErrorKind::AlreadyExists,
                format!(
                    "pad Unix socket already active at {}",
                    socket_path.display()
                ),
            ));
        }
        match std::fs::remove_file(socket_path) {
            Ok(()) => Ok(()),
            Err(err) if err.kind() == io::ErrorKind::NotFound => Ok(()),
            Err(err) => Err(err),
        }
    }

    /// 临时名带 pid，避免两个 pad 实例（或同进程的两个 socket）互相踩掉对方的 staging。
    fn staging_path(socket_path: &Path) -> PathBuf {
        let stem = socket_path
            .file_stem()
            .and_then(|stem| stem.to_str())
            .unwrap_or("pad-socket");
        socket_path.with_file_name(format!("{stem}.{}.tmp", std::process::id()))
    }

    fn link_unsupported(err: &io::Error) -> bool {
        // The staging name is longer than the published socket name. macOS
        // reports EINVAL/InvalidInput when only the staging name exceeds
        // SUN_LEN, so fall back to binding the final name directly.
        err.kind() == io::ErrorKind::Unsupported
            || err.kind() == io::ErrorKind::InvalidInput
            || matches!(
                err.raw_os_error(),
                Some(libc::EPERM) | Some(libc::EOPNOTSUPP) | Some(libc::EMLINK)
            )
    }

    fn set_mode(path: &Path, mode: u32) -> io::Result<()> {
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(mode))
    }

    /// Linux 上 $HOME 默认 0755，`~/.pad` 必须自己收到 0700，否则同机用户能走进来 connect。
    fn harden_socket_dir(dir: &Path) -> io::Result<()> {
        std::fs::create_dir_all(dir)?;
        let mode = std::fs::metadata(dir)?.permissions().mode() & 0o777;
        if mode & 0o077 != 0 {
            set_mode(dir, SOCKET_DIR_MODE)?;
        }
        Ok(())
    }
}

pub use cli::run_args;
pub use server::{start_api_listener, ApiReceiver};

#[cfg(test)]
pub(crate) mod tests;

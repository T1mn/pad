use super::*;

pub(super) enum ReaderEvent {
    Output(Vec<u8>),
    Closed(Result<(), String>),
}

pub(super) struct PendingInput {
    bytes: Vec<u8>,
    offset: usize,
}

pub(super) struct ReaderWorker {
    handle: Option<JoinHandle<()>>,
    cancelled: Arc<AtomicBool>,
}

impl ReaderWorker {
    pub(super) fn cancel(&self) {
        self.cancelled.store(true, Ordering::Release);
    }

    pub(super) fn is_finished(&self) -> bool {
        self.handle.as_ref().is_none_or(JoinHandle::is_finished)
    }

    pub(super) fn is_joined(&self) -> bool {
        self.handle.is_none()
    }

    pub(super) fn join_finished(&mut self) -> Result<(), &'static str> {
        let Some(handle) = self.handle.take() else {
            return Ok(());
        };
        debug_assert!(handle.is_finished());
        handle.join().map_err(|_| "reader thread panicked")
    }
}

impl Drop for ReaderWorker {
    fn drop(&mut self) {
        self.cancel();
        let deadline = Instant::now() + READER_JOIN_GRACE;
        while self
            .handle
            .as_ref()
            .is_some_and(|handle| !handle.is_finished())
            && Instant::now() < deadline
        {
            thread::sleep(CONTROL_POLL_INTERVAL);
        }
        if self.is_finished() {
            let _ = self.join_finished();
        }
    }
}

pub(super) fn spawn_reader(
    id: &TransportId,
    mut reader: Box<dyn Read + Send>,
    events: SyncSender<ReaderEvent>,
) -> io::Result<ReaderWorker> {
    let cancelled = Arc::new(AtomicBool::new(false));
    let reader_cancelled = Arc::clone(&cancelled);
    let handle = thread::Builder::new()
        .name(reader_thread_name(id))
        .spawn(move || {
            let mut buffer = vec![0; READ_BUFFER_SIZE];
            while !reader_cancelled.load(Ordering::Acquire) {
                match reader.read(&mut buffer) {
                    Ok(0) => {
                        send_reader_event(&events, ReaderEvent::Closed(Ok(())), &reader_cancelled);
                        return;
                    }
                    Ok(read) => {
                        if !send_reader_event(
                            &events,
                            ReaderEvent::Output(buffer[..read].to_vec()),
                            &reader_cancelled,
                        ) {
                            return;
                        }
                    }
                    Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
                    Err(error) if error.kind() == io::ErrorKind::WouldBlock => {
                        thread::sleep(CONTROL_POLL_INTERVAL);
                    }
                    Err(error) if pty_read_is_eof(&error) => {
                        send_reader_event(&events, ReaderEvent::Closed(Ok(())), &reader_cancelled);
                        return;
                    }
                    Err(error) => {
                        send_reader_event(
                            &events,
                            ReaderEvent::Closed(Err(error.to_string())),
                            &reader_cancelled,
                        );
                        return;
                    }
                }
            }
        })?;
    Ok(ReaderWorker {
        handle: Some(handle),
        cancelled,
    })
}

pub(super) fn pty_read_is_eof(error: &io::Error) -> bool {
    #[cfg(unix)]
    {
        error.raw_os_error() == Some(libc::EIO)
    }

    #[cfg(not(unix))]
    {
        let _ = error;
        false
    }
}

pub(super) fn send_reader_event(
    events: &SyncSender<ReaderEvent>,
    mut event: ReaderEvent,
    cancelled: &AtomicBool,
) -> bool {
    loop {
        if cancelled.load(Ordering::Acquire) {
            return false;
        }
        match events.try_send(event) {
            Ok(()) => return true,
            Err(TrySendError::Full(returned)) => {
                event = returned;
                thread::sleep(CONTROL_POLL_INTERVAL);
            }
            Err(TrySendError::Disconnected(_)) => return false,
        }
    }
}

pub(super) fn reader_thread_name(id: &TransportId) -> String {
    let suffix: String = id
        .as_str()
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || matches!(character, '-' | '_') {
                character
            } else {
                '_'
            }
        })
        .take(24)
        .collect();
    format!("pad-pty-reader-{suffix}")
}

pub(super) fn drain_reader_events(
    read_events: &Receiver<ReaderEvent>,
    pending_output: &mut VecDeque<Vec<u8>>,
    reader_closed: &mut Option<Result<(), String>>,
) {
    for _ in 0..READER_DRAIN_BUDGET {
        if pending_output.len() >= PENDING_OUTPUT_CAPACITY {
            return;
        }
        match read_events.try_recv() {
            Ok(ReaderEvent::Output(bytes)) => pending_output.push_back(bytes),
            Ok(ReaderEvent::Closed(result)) => {
                *reader_closed = Some(result);
                return;
            }
            Err(TryRecvError::Empty | TryRecvError::Disconnected) => return,
        }
    }
}

/// Keeps the PTY readable while a child is being signalled and reaped.
///
/// A process can be blocked inside a terminal write when its output buffer is
/// full.  If the transport stops draining at that exact point, even a pending
/// fatal signal may not be observed promptly on some kernels.  Preserve the
/// bounded tail that fits in `pending_output`, but discard overflow so forced
/// shutdown can always make progress.
pub(super) fn drain_reader_events_dropping_overflow(
    read_events: &Receiver<ReaderEvent>,
    pending_output: &mut VecDeque<Vec<u8>>,
    reader_closed: &mut Option<Result<(), String>>,
) {
    for _ in 0..READ_QUEUE_CAPACITY {
        match read_events.try_recv() {
            Ok(ReaderEvent::Output(bytes)) => {
                if pending_output.len() < PENDING_OUTPUT_CAPACITY {
                    pending_output.push_back(bytes);
                }
            }
            Ok(ReaderEvent::Closed(result)) => {
                *reader_closed = Some(result);
                return;
            }
            Err(TryRecvError::Empty | TryRecvError::Disconnected) => return,
        }
    }
}

pub(super) fn flush_transport_events(
    transport_events: &SyncSender<TransportEvent>,
    pending_resize: &mut Option<TerminalSize>,
    pending_output: &mut VecDeque<Vec<u8>>,
) -> Result<(), &'static str> {
    if let Some(size) = pending_resize.take() {
        match transport_events.try_send(TransportEvent::ResizeApplied(size)) {
            Ok(()) => {}
            Err(TrySendError::Full(_)) => {
                *pending_resize = Some(size);
                return Ok(());
            }
            Err(TrySendError::Disconnected(_)) => return Err("receiver disconnected"),
        }
    }

    for _ in 0..OUTPUT_FORWARD_BUDGET {
        let Some(bytes) = pending_output.pop_front() else {
            break;
        };
        match transport_events.try_send(TransportEvent::Output(bytes)) {
            Ok(()) => {}
            Err(TrySendError::Full(TransportEvent::Output(bytes))) => {
                pending_output.push_front(bytes);
                break;
            }
            Err(TrySendError::Disconnected(_)) => return Err("receiver disconnected"),
            Err(TrySendError::Full(_)) => unreachable!("output event changed variant"),
        }
    }
    Ok(())
}

pub(super) fn drain_commands(
    commands: &Receiver<TransportCommand>,
    pending_input: &mut VecDeque<PendingInput>,
    pending_input_bytes: &mut usize,
    pending_resize: &mut Option<TerminalSize>,
    master: &dyn MasterPty,
    shutdown_requested: &mut bool,
) -> Result<(), String> {
    for _ in 0..COMMAND_DRAIN_BUDGET {
        match commands.try_recv() {
            Ok(TransportCommand::Input(bytes)) if !*shutdown_requested => {
                let new_size = pending_input_bytes
                    .checked_add(bytes.len())
                    .ok_or_else(|| "pending PTY input size overflowed".to_string())?;
                if new_size > MAX_PENDING_INPUT_BYTES {
                    *shutdown_requested = true;
                    pending_input.clear();
                    *pending_input_bytes = 0;
                    return Err(format!(
                        "pending PTY input exceeded {MAX_PENDING_INPUT_BYTES} bytes"
                    ));
                }
                *pending_input_bytes = new_size;
                pending_input.push_back(PendingInput { bytes, offset: 0 });
            }
            Ok(TransportCommand::Input(_)) => {}
            Ok(TransportCommand::Resize(size)) if !*shutdown_requested => {
                master
                    .resize(pty_size(size))
                    .map_err(|error| error.to_string())?;
                *pending_resize = Some(size);
            }
            Ok(TransportCommand::Resize(_)) => {}
            Ok(TransportCommand::Shutdown) => *shutdown_requested = true,
            Err(TryRecvError::Empty) => break,
            Err(TryRecvError::Disconnected) => {
                *shutdown_requested = true;
                break;
            }
        }
    }
    Ok(())
}

pub(super) fn write_pending_input(
    writer: &mut dyn Write,
    pending_input: &mut VecDeque<PendingInput>,
    pending_input_bytes: &mut usize,
) -> io::Result<()> {
    let mut budget = WRITE_BUDGET;
    while budget > 0 {
        let Some(input) = pending_input.front_mut() else {
            break;
        };
        if input.offset == input.bytes.len() {
            pending_input.pop_front();
            continue;
        }
        let remaining = &input.bytes[input.offset..];
        let write_len = remaining.len().min(budget);
        match writer.write(&remaining[..write_len]) {
            Ok(0) => return Err(io::ErrorKind::WriteZero.into()),
            Ok(written) => {
                input.offset += written;
                *pending_input_bytes = pending_input_bytes.saturating_sub(written);
                budget -= written;
                if input.offset == input.bytes.len() {
                    pending_input.pop_front();
                }
            }
            Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
            Err(error) if error.kind() == io::ErrorKind::WouldBlock => break,
            Err(error) => return Err(error),
        }
    }
    Ok(())
}

#[cfg(unix)]
pub(super) fn set_master_nonblocking(master: &dyn MasterPty) -> io::Result<()> {
    let file_descriptor = master
        .as_raw_fd()
        .ok_or_else(|| io::Error::new(io::ErrorKind::Unsupported, "PTY has no file descriptor"))?;
    let flags = unsafe { libc::fcntl(file_descriptor, libc::F_GETFL) };
    if flags == -1 {
        return Err(io::Error::last_os_error());
    }
    if unsafe { libc::fcntl(file_descriptor, libc::F_SETFL, flags | libc::O_NONBLOCK) } == -1 {
        return Err(io::Error::last_os_error());
    }
    Ok(())
}

#[cfg(not(unix))]
pub(super) fn set_master_nonblocking(_master: &dyn MasterPty) -> io::Result<()> {
    Ok(())
}

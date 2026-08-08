use super::*;

pub(super) struct ChildGuard {
    child: Box<dyn Child + Send + Sync>,
    reaped: bool,
    #[cfg(unix)]
    child_pid: Option<libc::pid_t>,
}

impl ChildGuard {
    pub(super) fn new(child: Box<dyn Child + Send + Sync>) -> Self {
        #[cfg(unix)]
        let child_pid = child
            .process_id()
            .and_then(|pid| libc::pid_t::try_from(pid).ok())
            .filter(|pid| is_safe_child_pid(*pid));
        Self {
            child,
            reaped: false,
            #[cfg(unix)]
            child_pid,
        }
    }

    pub(super) fn try_wait(&mut self) -> io::Result<Option<ExitStatus>> {
        if self.reaped {
            return Ok(None);
        }
        let status = self.child.try_wait()?;
        if status.is_some() {
            self.reaped = true;
        }
        Ok(status)
    }

    pub(super) fn terminate(
        &mut self,
        master: &dyn MasterPty,
        mut service_output: impl FnMut(),
    ) -> io::Result<ExitStatus> {
        #[cfg(unix)]
        {
            let mut last_signal_error = match self.try_wait() {
                Ok(Some(status)) => return Ok(status),
                Ok(None) => None,
                Err(error) => Some(error),
            };
            for signal in [libc::SIGHUP, libc::SIGTERM, libc::SIGKILL] {
                // Foreground process groups can change when a shell starts a
                // job, so resolve them for every escalation stage. Always
                // signal the owned child PID as well; that prevents a shell
                // which moved out of the foreground group from escaping.
                if let Some(process_group) = safe_process_group(master) {
                    if let Err(error) = signal_process_group(process_group, signal) {
                        last_signal_error = Some(error);
                    }
                }
                if let Some(child_pid) = self.child_pid {
                    if let Err(error) = signal_process(child_pid, signal) {
                        last_signal_error = Some(error);
                    }
                }
                if signal == libc::SIGKILL {
                    if let Err(error) = self.child.kill() {
                        last_signal_error = Some(error);
                    }
                }
                let grace = if signal == libc::SIGKILL {
                    KILL_GRACE
                } else {
                    SIGNAL_GRACE
                };
                match self.wait_until_with(Instant::now() + grace, &mut service_output) {
                    Ok(Some(status)) => return Ok(status),
                    Ok(None) => {}
                    Err(error) => last_signal_error = Some(error),
                }
            }
            let detail = last_signal_error.map_or_else(
                || "child survived SIGHUP, SIGTERM, and SIGKILL deadlines".to_string(),
                |error| format!("child was not reaped; last signal error: {error}"),
            );
            Err(io::Error::new(io::ErrorKind::TimedOut, detail))
        }

        #[cfg(not(unix))]
        {
            match self.try_wait() {
                Ok(Some(status)) => return Ok(status),
                Ok(None) | Err(_) => {}
            }
            let kill_error = self.child.kill().err();
            match self.wait_until(Instant::now() + Duration::from_millis(500)) {
                Ok(Some(status)) => Ok(status),
                Ok(None) | Err(_) => Err(kill_error.unwrap_or_else(|| {
                    io::Error::new(
                        io::ErrorKind::TimedOut,
                        "child was not reaped before the kill deadline",
                    )
                })),
            }
        }
    }

    fn wait_until(&mut self, deadline: Instant) -> io::Result<Option<ExitStatus>> {
        self.wait_until_with(deadline, || {})
    }

    fn wait_until_with(
        &mut self,
        deadline: Instant,
        mut service: impl FnMut(),
    ) -> io::Result<Option<ExitStatus>> {
        loop {
            service();
            if let Some(status) = self.try_wait()? {
                return Ok(Some(status));
            }
            if Instant::now() >= deadline {
                return Ok(None);
            }
            thread::sleep(Duration::from_millis(10));
        }
    }
}

impl Drop for ChildGuard {
    fn drop(&mut self) {
        if self.reaped {
            return;
        }

        #[cfg(unix)]
        {
            // Setup can fail before the controller owns all PTY handles. The
            // captured PID is the only trustworthy target in that path.
            for signal in [libc::SIGHUP, libc::SIGTERM, libc::SIGKILL] {
                if let Some(child_pid) = self.child_pid {
                    let _ = signal_process(child_pid, signal);
                } else if signal == libc::SIGKILL {
                    let _ = self.child.kill();
                }
                if self
                    .wait_until(Instant::now() + SIGNAL_GRACE)
                    .ok()
                    .flatten()
                    .is_some()
                {
                    return;
                }
            }
        }

        #[cfg(not(unix))]
        {
            let _ = self.child.kill();
            let _ = self.wait_until(Instant::now() + Duration::from_millis(500));
        }
    }
}

#[cfg(unix)]
fn is_safe_child_pid(pid: libc::pid_t) -> bool {
    pid > 1 && pid != unsafe { libc::getpid() }
}

#[cfg(unix)]
fn safe_process_group(master: &dyn MasterPty) -> Option<libc::pid_t> {
    let process_group = master.process_group_leader()?;
    let own_process_group = unsafe { libc::getpgrp() };
    (process_group > 1 && process_group != own_process_group).then_some(process_group)
}

#[cfg(unix)]
fn signal_process_group(process_group: libc::pid_t, signal: libc::c_int) -> io::Result<()> {
    let result = unsafe { libc::kill(-process_group, signal) };
    if result == 0 {
        return Ok(());
    }
    let error = io::Error::last_os_error();
    if error.raw_os_error() == Some(libc::ESRCH) {
        Ok(())
    } else {
        Err(error)
    }
}

#[cfg(unix)]
pub(super) fn signal_process(pid: libc::pid_t, signal: libc::c_int) -> io::Result<()> {
    let result = unsafe { libc::kill(pid, signal) };
    if result == 0 {
        return Ok(());
    }
    let error = io::Error::last_os_error();
    if error.raw_os_error() == Some(libc::ESRCH) {
        Ok(())
    } else {
        Err(error)
    }
}

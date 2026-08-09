use std::any::Any;
use std::collections::hash_map::DefaultHasher;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::sync::mpsc::{self, Receiver, SyncSender};
use std::thread::{self, JoinHandle};

use super::model::TerminalEngineEvent;
use super::{
    EngineId, EngineRegistry, PaneId, TerminalEngine, TerminalError, TerminalScroll, TerminalSize,
    TerminalSnapshot,
};
use crate::panic_boundary::catch_isolated_unwind;

const DEFAULT_QUEUE_CAPACITY: usize = 256;

enum EngineCommand {
    Open {
        pane_id: PaneId,
        engine_id: EngineId,
        size: TerminalSize,
        reply: SyncSender<Result<(), TerminalError>>,
    },
    Feed {
        pane_id: PaneId,
        bytes: Vec<u8>,
        reply: SyncSender<Result<(), TerminalError>>,
    },
    Resize {
        pane_id: PaneId,
        size: TerminalSize,
        reply: SyncSender<Result<(), TerminalError>>,
    },
    Scroll {
        pane_id: PaneId,
        scroll: TerminalScroll,
        reply: SyncSender<Result<(), TerminalError>>,
    },
    Snapshot {
        pane_id: PaneId,
        reply: SyncSender<Result<TerminalSnapshot, TerminalError>>,
    },
    DrainEvents {
        pane_id: PaneId,
        reply: SyncSender<Result<Vec<TerminalEngineEvent>, TerminalError>>,
    },
    Close {
        pane_id: PaneId,
        reply: SyncSender<Result<(), TerminalError>>,
    },
}

pub struct EngineRuntime {
    senders: Vec<SyncSender<EngineCommand>>,
    workers: Vec<JoinHandle<()>>,
}

impl EngineRuntime {
    pub fn start(worker_count: usize, registry: EngineRegistry) -> Result<Self, TerminalError> {
        Self::start_with_queue_capacity(worker_count, DEFAULT_QUEUE_CAPACITY, registry)
    }

    fn start_with_queue_capacity(
        worker_count: usize,
        queue_capacity: usize,
        registry: EngineRegistry,
    ) -> Result<Self, TerminalError> {
        if worker_count == 0 {
            return Err(TerminalError::new(
                "terminal engine runtime needs at least one worker",
            ));
        }

        let mut senders = Vec::with_capacity(worker_count);
        let mut workers: Vec<JoinHandle<()>> = Vec::with_capacity(worker_count);
        for index in 0..worker_count {
            let (sender, receiver) = mpsc::sync_channel(queue_capacity);
            let worker_registry = registry.clone();
            let handle = match thread::Builder::new()
                .name(format!("pad-terminal-engine-{index}"))
                .spawn(move || run_worker(worker_registry, receiver))
            {
                Ok(handle) => handle,
                Err(error) => {
                    // Disconnect and join workers already started in this
                    // attempt instead of leaving detached threads behind.
                    drop(sender);
                    senders.clear();
                    for worker in workers.drain(..) {
                        let _ = worker.join();
                    }
                    return Err(TerminalError::new(format!(
                        "failed to start terminal engine worker: {error}"
                    )));
                }
            };
            senders.push(sender);
            workers.push(handle);
        }

        Ok(Self { senders, workers })
    }

    pub fn recommended_worker_count() -> usize {
        thread::available_parallelism()
            .map(usize::from)
            .unwrap_or(1)
            .clamp(1, 4)
    }

    pub fn open(
        &self,
        pane_id: PaneId,
        engine_id: EngineId,
        size: TerminalSize,
    ) -> Result<(), TerminalError> {
        let (reply, result) = mpsc::sync_channel(1);
        self.send(
            &pane_id,
            EngineCommand::Open {
                pane_id: pane_id.clone(),
                engine_id,
                size,
                reply,
            },
        )?;
        receive_reply(result)
    }

    pub fn feed(&self, pane_id: &PaneId, bytes: Vec<u8>) -> Result<(), TerminalError> {
        let (reply, result) = mpsc::sync_channel(1);
        self.send(
            pane_id,
            EngineCommand::Feed {
                pane_id: pane_id.clone(),
                bytes,
                reply,
            },
        )?;
        receive_reply(result)
    }

    pub fn resize(&self, pane_id: &PaneId, size: TerminalSize) -> Result<(), TerminalError> {
        let (reply, result) = mpsc::sync_channel(1);
        self.send(
            pane_id,
            EngineCommand::Resize {
                pane_id: pane_id.clone(),
                size,
                reply,
            },
        )?;
        receive_reply(result)
    }

    pub fn scroll(&self, pane_id: &PaneId, scroll: TerminalScroll) -> Result<(), TerminalError> {
        let (reply, result) = mpsc::sync_channel(1);
        self.send(
            pane_id,
            EngineCommand::Scroll {
                pane_id: pane_id.clone(),
                scroll,
                reply,
            },
        )?;
        receive_reply(result)
    }

    pub fn snapshot(&self, pane_id: &PaneId) -> Result<TerminalSnapshot, TerminalError> {
        let (reply, result) = mpsc::sync_channel(1);
        self.send(
            pane_id,
            EngineCommand::Snapshot {
                pane_id: pane_id.clone(),
                reply,
            },
        )?;
        receive_reply(result)
    }

    pub fn drain_events(
        &self,
        pane_id: &PaneId,
    ) -> Result<Vec<TerminalEngineEvent>, TerminalError> {
        let (reply, result) = mpsc::sync_channel(1);
        self.send(
            pane_id,
            EngineCommand::DrainEvents {
                pane_id: pane_id.clone(),
                reply,
            },
        )?;
        receive_reply(result)
    }

    pub fn close(&self, pane_id: &PaneId) -> Result<(), TerminalError> {
        let (reply, result) = mpsc::sync_channel(1);
        self.send(
            pane_id,
            EngineCommand::Close {
                pane_id: pane_id.clone(),
                reply,
            },
        )?;
        receive_reply(result)
    }

    fn send(&self, pane_id: &PaneId, command: EngineCommand) -> Result<(), TerminalError> {
        self.senders[self.shard_index(pane_id)]
            .send(command)
            .map_err(|_| TerminalError::new("terminal engine worker stopped"))
    }

    pub(super) fn shard_index(&self, pane_id: &PaneId) -> usize {
        let mut hasher = DefaultHasher::new();
        pane_id.hash(&mut hasher);
        (hasher.finish() as usize) % self.senders.len()
    }
}

impl Drop for EngineRuntime {
    fn drop(&mut self) {
        // Disconnect every queue before joining. Workers drain commands that
        // were already accepted, then exit when `recv` observes disconnect.
        // This also avoids trying to enqueue shutdown behind a full queue.
        self.senders.clear();
        for worker in self.workers.drain(..) {
            let _ = worker.join();
        }
    }
}

fn receive_reply<T>(receiver: Receiver<Result<T, TerminalError>>) -> Result<T, TerminalError> {
    receiver
        .recv()
        .map_err(|_| TerminalError::new("terminal engine worker dropped its reply"))?
}

fn run_worker(registry: EngineRegistry, receiver: Receiver<EngineCommand>) {
    let mut engines: HashMap<PaneId, Box<dyn TerminalEngine>> = HashMap::new();
    while let Ok(command) = receiver.recv() {
        match command {
            EngineCommand::Open {
                pane_id,
                engine_id,
                size,
                reply,
            } => {
                let result = if engines.contains_key(&pane_id) {
                    Err(TerminalError::new(format!(
                        "terminal pane '{pane_id}' is already open"
                    )))
                } else {
                    match catch_isolated_unwind(|| registry.create(&engine_id, size)) {
                        Ok(result) => result.map(|engine| {
                            engines.insert(pane_id.clone(), engine);
                        }),
                        Err(payload) => Err(engine_panic_error(
                            &pane_id,
                            &format!("create '{engine_id}'"),
                            payload.as_ref(),
                        )),
                    }
                };
                let _ = reply.send(result);
            }
            EngineCommand::Feed {
                pane_id,
                bytes,
                reply,
            } => {
                let result =
                    call_engine(&mut engines, &pane_id, "feed", |engine| engine.feed(&bytes));
                let _ = reply.send(result);
            }
            EngineCommand::Resize {
                pane_id,
                size,
                reply,
            } => {
                let result = call_engine(&mut engines, &pane_id, "resize", |engine| {
                    engine.resize(size)
                });
                let _ = reply.send(result);
            }
            EngineCommand::Scroll {
                pane_id,
                scroll,
                reply,
            } => {
                let result = call_engine(&mut engines, &pane_id, "scroll", |engine| {
                    engine.scroll(scroll)
                });
                let _ = reply.send(result);
            }
            EngineCommand::Snapshot { pane_id, reply } => {
                let result = call_engine(&mut engines, &pane_id, "snapshot", |engine| {
                    Ok(engine.snapshot())
                });
                let _ = reply.send(result);
            }
            EngineCommand::DrainEvents { pane_id, reply } => {
                let result = call_engine(&mut engines, &pane_id, "drain events", |engine| {
                    Ok(engine.drain_events())
                });
                let _ = reply.send(result);
            }
            EngineCommand::Close { pane_id, reply } => {
                let result = close_engine(&mut engines, &pane_id);
                let _ = reply.send(result);
            }
        }
    }

    // A panicking destructor must not prevent the remaining panes on this
    // shard from being destroyed or turn graceful runtime shutdown into an
    // unwind through `EngineRuntime::drop`.
    for (_, engine) in engines.drain() {
        let _ = drop_engine(engine);
    }
}

fn pane_not_open(pane_id: &PaneId) -> TerminalError {
    TerminalError::new(format!("terminal pane '{pane_id}' is not open"))
}

fn call_engine<T>(
    engines: &mut HashMap<PaneId, Box<dyn TerminalEngine>>,
    pane_id: &PaneId,
    operation: &str,
    call: impl FnOnce(&mut dyn TerminalEngine) -> Result<T, TerminalError>,
) -> Result<T, TerminalError> {
    let outcome = {
        let engine = engines
            .get_mut(pane_id)
            .ok_or_else(|| pane_not_open(pane_id))?;
        catch_isolated_unwind(|| call(engine.as_mut()))
    };

    match outcome {
        Ok(result) => result,
        Err(payload) => {
            let operation_error = engine_panic_error(pane_id, operation, payload.as_ref());
            if let Some(engine) = engines.remove(pane_id) {
                if let Err(drop_payload) = drop_engine(engine) {
                    return Err(TerminalError::new(format!(
                        "{operation_error}; cleanup panicked: {}",
                        panic_message(drop_payload.as_ref())
                    )));
                }
            }
            Err(operation_error)
        }
    }
}

fn close_engine(
    engines: &mut HashMap<PaneId, Box<dyn TerminalEngine>>,
    pane_id: &PaneId,
) -> Result<(), TerminalError> {
    let engine = engines
        .remove(pane_id)
        .ok_or_else(|| pane_not_open(pane_id))?;
    drop_engine(engine).map_err(|payload| engine_panic_error(pane_id, "close", payload.as_ref()))
}

fn drop_engine(engine: Box<dyn TerminalEngine>) -> Result<(), Box<dyn Any + Send + 'static>> {
    catch_isolated_unwind(|| drop(engine))
}

fn engine_panic_error(
    pane_id: &PaneId,
    operation: &str,
    payload: &(dyn Any + Send),
) -> TerminalError {
    TerminalError::new(format!(
        "terminal pane '{pane_id}' engine {operation} panicked: {}",
        panic_message(payload)
    ))
}

fn panic_message(payload: &(dyn Any + Send)) -> String {
    if let Some(message) = payload.downcast_ref::<String>() {
        message.clone()
    } else if let Some(message) = payload.downcast_ref::<&'static str>() {
        (*message).to_string()
    } else {
        "non-string panic payload".to_string()
    }
}

#[cfg(test)]
#[path = "worker_tests.rs"]
pub(crate) mod tests;

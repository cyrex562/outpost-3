//! The DB/session actor.
//!
//! `harsh-core`'s `WorldDatabase` is synchronous (`rusqlite`, single connection)
//! and `GMController` borrows it for the session lifetime. To serialize access
//! and avoid a self-referential `(db, controller)` struct, a dedicated blocking
//! thread owns the active world and a long-lived controller, processing jobs
//! from an mpsc channel. Async handlers send a job + oneshot reply and await it.
//!
//! Game events produced by player input are published to a broadcast channel;
//! each `/ws` connection subscribes and forwards them.

use std::path::PathBuf;

use serde_json::{json, Value};
use tokio::sync::{broadcast, mpsc, oneshot};

use harsh_core::character_creation::CharacterDraftSubmission;
use harsh_core::db::WorldDatabase;
use harsh_core::events::GameEvent;
use harsh_core::gm::GMController;
use harsh_core::public_api::CreateWorldRequest;

use crate::worldsvc::{
    build_controller, create_and_place_character, create_world, current_world_json, delete_world,
    open_world, world_filename, ActorPaths,
};

type Reply<T> = oneshot::Sender<T>;

/// A read closure executed against the active world's database.
pub type DbRead = Box<dyn FnOnce(&WorldDatabase) -> Result<Value, String> + Send>;

/// Work items processed by the session actor.
pub enum SessionJob {
    CreateWorld {
        req: CreateWorldRequest,
        reply: Reply<Result<Value, String>>,
    },
    LoadWorld {
        file: String,
        reply: Reply<Result<Value, String>>,
    },
    DeleteWorld {
        file: String,
        reply: Reply<Result<Value, String>>,
    },
    CurrentWorld {
        reply: Reply<Value>,
    },
    CreateCharacter {
        submission: Box<CharacterDraftSubmission>,
        reply: Reply<Result<Value, String>>,
    },
    PlayerInput {
        text: String,
        reply: Reply<()>,
    },
    /// Re-read the loaded world's `ir_records` into the live controller (after
    /// content is stored), so authored IR goes live without a reload.
    RefreshContent {
        reply: Reply<()>,
    },
    InitialNarration {
        reply: Reply<Vec<GameEvent>>,
    },
    Read {
        job: DbRead,
        reply: Reply<Result<Value, String>>,
    },
}

/// Cheaply-cloneable handle to the session actor.
#[derive(Clone)]
pub struct SessionHandle {
    tx: mpsc::Sender<SessionJob>,
    /// Broadcast of game events; `/ws` connections subscribe to this.
    pub events: broadcast::Sender<GameEvent>,
}

const CLOSED: &str = "session actor is not available";

impl SessionHandle {
    async fn send(&self, job: SessionJob) -> Result<(), String> {
        self.tx.send(job).await.map_err(|_| CLOSED.to_string())
    }

    pub async fn create_world(&self, req: CreateWorldRequest) -> Result<Value, String> {
        let (reply, rx) = oneshot::channel();
        self.send(SessionJob::CreateWorld { req, reply }).await?;
        rx.await.map_err(|_| CLOSED.to_string())?
    }

    pub async fn load_world(&self, file: String) -> Result<Value, String> {
        let (reply, rx) = oneshot::channel();
        self.send(SessionJob::LoadWorld { file, reply }).await?;
        rx.await.map_err(|_| CLOSED.to_string())?
    }

    pub async fn delete_world(&self, file: String) -> Result<Value, String> {
        let (reply, rx) = oneshot::channel();
        self.send(SessionJob::DeleteWorld { file, reply }).await?;
        rx.await.map_err(|_| CLOSED.to_string())?
    }

    pub async fn current_world(&self) -> Result<Value, String> {
        let (reply, rx) = oneshot::channel();
        self.send(SessionJob::CurrentWorld { reply }).await?;
        rx.await.map_err(|_| CLOSED.to_string())
    }

    pub async fn create_character(
        &self,
        submission: CharacterDraftSubmission,
    ) -> Result<Value, String> {
        let (reply, rx) = oneshot::channel();
        self.send(SessionJob::CreateCharacter {
            submission: Box::new(submission),
            reply,
        })
        .await?;
        rx.await.map_err(|_| CLOSED.to_string())?
    }

    pub async fn player_input(&self, text: String) -> Result<(), String> {
        let (reply, rx) = oneshot::channel();
        self.send(SessionJob::PlayerInput { text, reply }).await?;
        rx.await.map_err(|_| CLOSED.to_string())
    }

    /// Ask the live controller to re-read the world's IR records.
    pub async fn refresh_content(&self) -> Result<(), String> {
        let (reply, rx) = oneshot::channel();
        self.send(SessionJob::RefreshContent { reply }).await?;
        rx.await.map_err(|_| CLOSED.to_string())
    }

    pub async fn initial_narration(&self) -> Result<Vec<GameEvent>, String> {
        let (reply, rx) = oneshot::channel();
        self.send(SessionJob::InitialNarration { reply }).await?;
        rx.await.map_err(|_| CLOSED.to_string())
    }

    /// Run a read closure against the active world's database.
    pub async fn read<F>(&self, f: F) -> Result<Value, String>
    where
        F: FnOnce(&WorldDatabase) -> Result<Value, String> + Send + 'static,
    {
        let (reply, rx) = oneshot::channel();
        self.send(SessionJob::Read {
            job: Box::new(f),
            reply,
        })
        .await?;
        rx.await.map_err(|_| CLOSED.to_string())?
    }
}

/// Spawn the session actor thread and return a handle to it.
pub fn spawn(paths: ActorPaths) -> SessionHandle {
    let (tx, rx) = mpsc::channel::<SessionJob>(64);
    let (events, _) = broadcast::channel::<GameEvent>(256);
    let events_thread = events.clone();
    std::thread::Builder::new()
        .name("harsh-session".into())
        .spawn(move || actor_loop(rx, events_thread, paths))
        .expect("failed to spawn session actor thread");
    SessionHandle { tx, events }
}

/// Outcome of an active-world session loop.
enum Flow {
    /// The channel closed; stop the actor.
    Shutdown,
    /// Switch to a different world (load).
    Switch(WorldDatabase),
    /// Drop the active world and return to the world-less state.
    Unload,
    /// Drop the active world, then delete its file (it was deleted while loaded).
    UnloadThenDelete(PathBuf),
}

fn actor_loop(
    mut rx: mpsc::Receiver<SessionJob>,
    events: broadcast::Sender<GameEvent>,
    paths: ActorPaths,
) {
    let mut active: Option<WorldDatabase> = None;
    loop {
        if let Some(db) = active.take() {
            let flow = run_world_session(&db, &mut rx, &events, &paths);
            // Close the connection before any file operations or re-activation.
            drop(db);
            match flow {
                Flow::Shutdown => return,
                Flow::Switch(new_db) => active = Some(new_db),
                Flow::Unload => {}
                Flow::UnloadThenDelete(path) => {
                    let _ = std::fs::remove_file(path);
                }
            }
        } else {
            match rx.blocking_recv() {
                None => return,
                Some(job) => {
                    if let Some(db) = handle_worldless(job, &paths) {
                        active = Some(db);
                    }
                }
            }
        }
    }
}

/// Handle a job when no world is loaded. Returns `Some(db)` to activate a world.
fn handle_worldless(job: SessionJob, paths: &ActorPaths) -> Option<WorldDatabase> {
    match job {
        SessionJob::CreateWorld { req, reply } => {
            // Create + generate the file but do NOT auto-load: the frontend
            // issues an explicit `/api/worlds/load` afterwards.
            match create_world(paths, &req) {
                Ok((db, info)) => {
                    drop(db);
                    let _ = reply.send(Ok(info));
                }
                Err(e) => {
                    let _ = reply.send(Err(e));
                }
            }
            None
        }
        SessionJob::LoadWorld { file, reply } => match open_world(paths, &file) {
            Ok((db, info)) => {
                let _ = reply.send(Ok(info));
                Some(db)
            }
            Err(e) => {
                let _ = reply.send(Err(e));
                None
            }
        },
        SessionJob::DeleteWorld { file, reply } => {
            let _ = reply.send(delete_world(paths, &file));
            None
        }
        SessionJob::CurrentWorld { reply } => {
            let _ = reply.send(json!({}));
            None
        }
        SessionJob::CreateCharacter { reply, .. } => {
            let _ = reply.send(Err("no world loaded".into()));
            None
        }
        SessionJob::PlayerInput { reply, .. } => {
            let _ = reply.send(());
            None
        }
        SessionJob::RefreshContent { reply } => {
            let _ = reply.send(());
            None
        }
        SessionJob::InitialNarration { reply } => {
            let _ = reply.send(vec![]);
            None
        }
        SessionJob::Read { reply, .. } => {
            let _ = reply.send(Err("no world loaded".into()));
            None
        }
    }
}

/// Drive an active world until the channel closes or the world switches.
fn run_world_session(
    db: &WorldDatabase,
    rx: &mut mpsc::Receiver<SessionJob>,
    events: &broadcast::Sender<GameEvent>,
    paths: &ActorPaths,
) -> Flow {
    let mut controller = match build_controller(db, paths) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("session: failed to build controller: {e}");
            // Park serving reads + world switches until the world changes.
            return park_until_switch(db, rx, paths);
        }
    };

    loop {
        let Some(job) = rx.blocking_recv() else {
            return Flow::Shutdown;
        };
        match job {
            SessionJob::PlayerInput { text, reply } => {
                for event in controller.handle_input(&text) {
                    let _ = events.send(event);
                }
                let _ = reply.send(());
            }
            SessionJob::InitialNarration { reply } => {
                let _ = reply.send(controller.get_initial_narration());
            }
            SessionJob::RefreshContent { reply } => {
                controller.reload_runtime_content();
                let _ = reply.send(());
            }
            SessionJob::CreateCharacter { submission, reply } => {
                match create_and_place_character(db, &submission) {
                    Ok(info) => {
                        // Re-derive scene state so the controller advances to
                        // exploration now that a character exists.
                        controller.initialize();
                        // The form flow has no chat round-trip, so mirror the
                        // chat scene's transition events over the broadcast so a
                        // connected client advances to exploration.
                        broadcast_creation(events, &mut controller, &info);
                        let _ = reply.send(Ok(info));
                    }
                    Err(e) => {
                        let _ = reply.send(Err(e));
                    }
                }
            }
            SessionJob::Read { job, reply } => {
                let _ = reply.send(job(db));
            }
            SessionJob::CurrentWorld { reply } => {
                let _ = reply.send(current_world_json(db));
            }
            SessionJob::CreateWorld { req, reply } => {
                // Create the file without switching to it (frontend loads it
                // explicitly afterwards).
                match create_world(paths, &req) {
                    Ok((new_db, info)) => {
                        drop(new_db);
                        let _ = reply.send(Ok(info));
                    }
                    Err(e) => {
                        let _ = reply.send(Err(e));
                    }
                }
            }
            SessionJob::LoadWorld { file, reply } => match open_world(paths, &file) {
                Ok((new_db, info)) => {
                    let _ = reply.send(Ok(info));
                    return Flow::Switch(new_db);
                }
                Err(e) => {
                    let _ = reply.send(Err(e));
                }
            },
            SessionJob::DeleteWorld { file, reply } => {
                // Deleting the currently-loaded world must unload it first so the
                // sqlite connection is closed before the file is removed.
                if world_filename(db) == file {
                    let path = paths.worlds_dir.join(&file);
                    let _ = reply.send(Ok(json!({ "ok": true })));
                    return Flow::UnloadThenDelete(path);
                }
                let _ = reply.send(delete_world(paths, &file));
            }
        }
    }
}

/// Broadcast the events a freshly form-created character needs to enter play:
/// the "created" narration, a `character.created` event, a `gm.scene_change`,
/// and the new scene's opening narration. Mirrors the chat scene's
/// `persist_character` output so the frontend transitions identically.
fn broadcast_creation(
    events: &broadcast::Sender<GameEvent>,
    controller: &mut GMController<'_>,
    info: &Value,
) {
    let tick = controller.tick();
    let name = info.get("name").and_then(Value::as_str).unwrap_or("");
    let char_id = info.get("id").and_then(Value::as_str).unwrap_or("");

    let obj = |v: Value| v.as_object().cloned().unwrap_or_default();
    let _ = events.send(
        GameEvent::new(
            tick,
            "gm.narrate",
            obj(json!({
                "text": format!("Character \"{name}\" created! Your adventure begins\u{2026}")
            })),
        )
        .with_source("gm"),
    );
    let _ = events.send(
        GameEvent::new(tick, "character.created", obj(json!({ "character_id": char_id })))
            .with_source("gm"),
    );
    let _ = events.send(
        GameEvent::new(tick, "gm.scene_change", obj(json!({ "to": "exploration" })))
            .with_source("gm"),
    );
    for event in controller.get_initial_narration() {
        let _ = events.send(event);
    }
}

/// Fallback loop when the controller can't be built (e.g. missing content):
/// still serves world reads and create/load switches, but rejects gameplay.
fn park_until_switch(
    db: &WorldDatabase,
    rx: &mut mpsc::Receiver<SessionJob>,
    paths: &ActorPaths,
) -> Flow {
    loop {
        let Some(job) = rx.blocking_recv() else {
            return Flow::Shutdown;
        };
        match job {
            SessionJob::Read { job, reply } => {
                let _ = reply.send(job(db));
            }
            SessionJob::CurrentWorld { reply } => {
                let _ = reply.send(current_world_json(db));
            }
            SessionJob::CreateWorld { req, reply } => {
                match create_world(paths, &req) {
                    Ok((new_db, info)) => {
                        drop(new_db);
                        let _ = reply.send(Ok(info));
                    }
                    Err(e) => {
                        let _ = reply.send(Err(e));
                    }
                }
            }
            SessionJob::LoadWorld { file, reply } => match open_world(paths, &file) {
                Ok((new_db, info)) => {
                    let _ = reply.send(Ok(info));
                    return Flow::Switch(new_db);
                }
                Err(e) => {
                    let _ = reply.send(Err(e));
                }
            },
            SessionJob::DeleteWorld { file, reply } => {
                if world_filename(db) == file {
                    let path = paths.worlds_dir.join(&file);
                    let _ = reply.send(Ok(json!({ "ok": true })));
                    return Flow::UnloadThenDelete(path);
                }
                let _ = reply.send(delete_world(paths, &file));
            }
            SessionJob::CreateCharacter { reply, .. } => {
                let _ = reply.send(Err("world unavailable".into()));
            }
            SessionJob::PlayerInput { reply, .. } => {
                let _ = reply.send(());
            }
            SessionJob::RefreshContent { reply } => {
                let _ = reply.send(());
            }
            SessionJob::InitialNarration { reply } => {
                let _ = reply.send(vec![]);
            }
        }
    }
}

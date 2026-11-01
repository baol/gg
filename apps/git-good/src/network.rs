//! Serialized network operations with bounded progress and cooperative cancellation.
use eframe::egui;
use gg_engine::sync::{
    CloneReceipt, FetchReceipt, Operation, Progress, PushReceipt, UpstreamRelation,
};
use std::{
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
};

#[derive(Debug)]
pub enum Outcome {
    Fetch(FetchReceipt),
    /// The fetch half of Pull, with how the branch then stands against its upstream.
    Pull {
        fetch: FetchReceipt,
        relation: Option<UpstreamRelation>,
    },
    Push(PushReceipt),
    Clone(CloneReceipt),
    /// A remote branch was deleted; the message names it.
    DeleteRemote(String),
}

pub enum Message {
    Progress(Progress),
    Done(Result<Outcome, String>),
}

pub struct Worker {
    pub operation: Operation,
    receiver: mpsc::Receiver<Message>,
    cancel: Arc<AtomicBool>,
    #[cfg(test)]
    _keep_alive: Option<mpsc::Sender<Message>>,
}

impl Worker {
    pub fn fetch(
        ctx: &egui::Context,
        path: PathBuf,
        remote: Option<String>,
        prune: bool,
    ) -> Result<Self, String> {
        start(ctx, Operation::Fetch, move |cancel, progress| {
            gg_git::sync::fetch(&path, remote.as_deref(), prune, false, &cancel, progress)
                .map(Outcome::Fetch)
        })
    }

    /// Fetch `remote`, then read how `branch` (a full ref name) stands against
    /// its upstream, so Pull integrates only when there is something to take.
    pub fn pull(
        ctx: &egui::Context,
        path: PathBuf,
        remote: String,
        branch: String,
        prune: bool,
    ) -> Result<Self, String> {
        start(ctx, Operation::Fetch, move |cancel, progress| {
            let fetch = gg_git::sync::fetch(&path, Some(&remote), prune, false, &cancel, progress)?;
            let relation = gg_git::sync::upstream_relation(&path, Some(&branch))?;
            Ok(Outcome::Pull { fetch, relation })
        })
    }

    pub fn push(
        ctx: &egui::Context,
        path: PathBuf,
        remote: Option<String>,
        branch: Option<String>,
        set_upstream: bool,
        force_with_lease: Option<String>,
    ) -> Result<Self, String> {
        start(ctx, Operation::Push, move |cancel, progress| {
            gg_git::sync::push(
                &path,
                remote.as_deref(),
                branch.as_deref(),
                gg_git::sync::PushOptions {
                    set_upstream,
                    force_with_lease,
                },
                false,
                &cancel,
                progress,
            )
            .map(Outcome::Push)
        })
    }

    pub fn delete_remote(
        ctx: &egui::Context,
        path: PathBuf,
        remote: String,
        branch: String,
        expected: String,
    ) -> Result<Self, String> {
        start(ctx, Operation::Push, move |cancel, progress| {
            gg_git::sync::delete_remote_branch(
                &path, &remote, &branch, &expected, false, &cancel, progress,
            )
            .map(Outcome::DeleteRemote)
        })
    }

    pub fn clone(
        ctx: &egui::Context,
        url: String,
        destination: Option<PathBuf>,
    ) -> Result<Self, String> {
        start(ctx, Operation::Clone, move |cancel, progress| {
            gg_git::sync::clone(&url, destination.as_deref(), false, &cancel, progress)
                .map(Outcome::Clone)
        })
    }

    pub fn try_recv(&self) -> Result<Message, mpsc::TryRecvError> {
        self.receiver.try_recv()
    }

    pub fn cancel(&self) {
        self.cancel.store(true, Ordering::Relaxed);
    }

    pub fn is_cancelled(&self) -> bool {
        self.cancel.load(Ordering::Relaxed)
    }

    #[cfg(test)]
    pub(crate) fn idle(operation: Operation) -> Self {
        let (sender, receiver) = mpsc::channel();
        Self {
            operation,
            receiver,
            cancel: Arc::new(AtomicBool::new(false)),
            _keep_alive: Some(sender),
        }
    }
}

fn start(
    ctx: &egui::Context,
    operation: Operation,
    operation_fn: impl FnOnce(Arc<AtomicBool>, &mut dyn FnMut(Progress)) -> Result<Outcome, String>
    + Send
    + 'static,
) -> Result<Worker, String> {
    let (sender, receiver) = mpsc::sync_channel(32);
    let cancel = Arc::new(AtomicBool::new(false));
    let worker_cancel = Arc::clone(&cancel);
    let repaint = ctx.clone();
    std::thread::Builder::new()
        .name(format!("gg-{}", operation.as_str()))
        .spawn(move || {
            let progress_sender = sender.clone();
            let progress_repaint = repaint.clone();
            let mut progress = move |update: Progress| {
                let _ = progress_sender.try_send(Message::Progress(update));
                progress_repaint.request_repaint();
            };
            let result = operation_fn(worker_cancel, &mut progress);
            let _ = sender.send(Message::Done(result));
            repaint.request_repaint();
        })
        .map_err(|e| format!("Start {} operation: {e}", operation.as_str()))?;
    Ok(Worker {
        operation,
        receiver,
        cancel,
        #[cfg(test)]
        _keep_alive: None,
    })
}

//! Serialized network operations with bounded progress and cooperative cancellation.
use eframe::egui;
use gg_engine::sync::{CloneReceipt, FetchReceipt, Operation, Progress, PushReceipt};
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
    Push(PushReceipt),
    Clone(CloneReceipt),
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

    pub fn push(
        ctx: &egui::Context,
        path: PathBuf,
        remote: Option<String>,
        branch: Option<String>,
        set_upstream: bool,
    ) -> Result<Self, String> {
        start(ctx, Operation::Push, move |cancel, progress| {
            gg_git::sync::push(
                &path,
                remote.as_deref(),
                branch.as_deref(),
                set_upstream,
                false,
                &cancel,
                progress,
            )
            .map(Outcome::Push)
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

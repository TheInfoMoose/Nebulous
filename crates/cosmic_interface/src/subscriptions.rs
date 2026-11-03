use std::hash::{Hash, Hasher};
use std::path::PathBuf;
use std::sync::mpsc::RecvTimeoutError;
use std::time::Duration;

use backend_core::{GitSyncMode, TrackerStorage, TrackerWatcher};
use cosmic::iced::futures::SinkExt;
use cosmic::iced::{stream, Subscription};

use crate::app::{Message, NebulousApp};

#[derive(Debug, Clone)]
struct WatcherSubscriptionId {
    tracker_dir: PathBuf,
}

impl Hash for WatcherSubscriptionId {
    fn hash<H: Hasher>(&self, state: &mut H) {
        "nebulous-tracker-notify-watcher".hash(state);
        self.tracker_dir.hash(state);
    }
}

/// Builds reactive background subscriptions for the Nebulous GUI:
/// 1. `.tracker/` filesystem watcher (`notify`) that reconciles indexes and refreshes the GUI when
///    external CLI commands (`neb bug`, `neb close`) or Git operations mutate `.tracker/`.
/// 2. Active session timer 1-second tick when `.tracker/.timer.toml` is active.
/// 3. Scheduled Git sync periodic tick when `project.toml` `[git].sync_mode == "scheduled"`.
pub fn subscription(app: &NebulousApp) -> Subscription<Message> {
    let mut subs = Vec::with_capacity(3);

    // 1. Live `.tracker/` notify filesystem watcher subscription
    subs.push(tracker_watcher_subscription(app.storage.clone()));

    // 2. Active session timer 1-second tick
    if app.active_timer.is_some() {
        subs.push(cosmic::iced::time::every(Duration::from_secs(1)).map(|_| Message::TimerTick));
    }

    // 3. Scheduled Git sync periodic tick when `sync_mode == Scheduled`
    if app.index.project.git.sync_mode == GitSyncMode::Scheduled {
        let interval_mins = u64::from(app.index.project.git.sync_interval_minutes.max(1));
        subs.push(
            cosmic::iced::time::every(Duration::from_secs(interval_mins.saturating_mul(60)))
                .map(|_| Message::ScheduledGitSyncTick),
        );
    }

    Subscription::batch(subs)
}

/// Creates a `Subscription` that runs [`TrackerWatcher`] against `storage.tracker_dir()` and emits
/// [`Message::TrackerFilesChanged`] whenever files inside `.tracker/` are created, modified, or removed.
pub fn tracker_watcher_subscription(storage: TrackerStorage) -> Subscription<Message> {
    let sub_id = WatcherSubscriptionId {
        tracker_dir: storage.tracker_dir().to_path_buf(),
    };

    Subscription::run_with(sub_id, build_watcher_stream)
}

fn build_watcher_stream(
    sub_id: &WatcherSubscriptionId,
) -> impl cosmic::iced::futures::Stream<Item = Message> {
    let tracker_dir = sub_id.tracker_dir.clone();
    stream::channel(
        32,
        move |mut output: cosmic::iced::futures::channel::mpsc::Sender<Message>| async move {
            let storage = TrackerStorage::from_tracker_dir(tracker_dir);
            let (mut async_tx, mut async_rx) =
                cosmic::iced::futures::channel::mpsc::channel::<Vec<PathBuf>>(32);

            std::thread::spawn(move || {
                let Ok((_watcher, rx)) = TrackerWatcher::start(storage) else {
                    return;
                };

                loop {
                    match rx.recv_timeout(Duration::from_millis(500)) {
                        Ok(event) => {
                            if async_tx.try_send(event.changed_paths).is_err() {
                                break;
                            }
                        }
                        Err(RecvTimeoutError::Timeout) => {
                            if async_tx.is_closed() {
                                break;
                            }
                        }
                        Err(RecvTimeoutError::Disconnected) => break,
                    }
                }
            });

            use cosmic::iced::futures::StreamExt;
            while let Some(changed_paths) = async_rx.next().await {
                if output
                    .send(Message::TrackerFilesChanged(changed_paths))
                    .await
                    .is_err()
                {
                    break;
                }
            }
        },
    )
}

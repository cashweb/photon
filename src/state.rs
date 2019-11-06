use std::sync::{
    atomic::{AtomicU32, AtomicUsize, Ordering},
    RwLock,
};

use crossbeam::{
    queue::{ArrayQueue, PushError},
    utils::CachePadded,
};
use futures::channel::oneshot;

const RESUME_QUEUE_SIZE: usize = 2048;

/// Represents the current state of the service
#[derive(Copy, Clone)]
pub enum State {
    Syncing,
    Active,
    ReOrgPending,
    ReOrg,
}

pub struct StateMananger {
    /// State of the service
    state: RwLock<State>,
    /// Stores resume channel senders
    resume_queue: ArrayQueue<oneshot::Sender<bool>>,
    /// Stores the number of active requests
    active_counter: CachePadded<AtomicUsize>,
    /// Stores the current sync position
    /// 1 + the last block height sync'd
    sync_position: CachePadded<AtomicU32>,
    // Broadcast queue to for headers
}

impl Default for StateMananger {
    fn default() -> Self {
        StateMananger {
            state: RwLock::new(State::Syncing),
            resume_queue: ArrayQueue::new(RESUME_QUEUE_SIZE),
            active_counter: CachePadded::new(AtomicUsize::new(0)),
            sync_position: CachePadded::new(AtomicU32::new(0)),
        }
    }
}

impl StateMananger {
    /// Create new resume channel, expelling oldest if capacity is full
    fn new_resume_channel(&self) -> oneshot::Receiver<bool> {
        // Init new resume channel
        let (mut send, recv) = oneshot::channel::<bool>();

        // Keep trying to push until space
        while let Err(PushError(err_send)) = self.resume_queue.push(send) {
            send = err_send;
            // Expel oldest from resume queue with false
            if let Ok(expelled) = self.resume_queue.pop() {
                expelled.send(false).unwrap();
            }
        }
        recv
    }

    /// Check whether service can process a request in current state
    /// Returns a oneshot receiver representing readiness
    /// Receiving true through the channel implies receiver to should process request
    /// Receiving false through the channel implies receiver should reject request
    /// Receiving None indicates that the request should be immediately processed
    pub fn try_barrier(&self) -> Option<oneshot::Receiver<bool>> {
        let state_read = self.state.read().unwrap();
        match *state_read {
            State::Syncing => Some(self.new_resume_channel()),
            State::Active => {
                // Increment active request counter
                self.active_counter.fetch_add(1, Ordering::SeqCst);
                None
            }
            State::ReOrgPending => Some(self.new_resume_channel()),
            State::ReOrg => Some(self.new_resume_channel()),
        }
    }

    /// Signal to state manager that request has completed
    pub fn signal_completion(&self) {
        let state_read = self.state.read().unwrap();
        match *state_read {
            State::Active => {
                self.active_counter.fetch_sub(1, Ordering::SeqCst);
            }
            State::ReOrgPending => {
                let active_count = self.active_counter.fetch_sub(1, Ordering::SeqCst);
                drop(state_read);

                // When active count gets to 0 then transition to reorganisation state
                if active_count == 0 {
                    // State transition
                    self.transition(State::ReOrg)
                }
            }
            _ => {
                // Request cannot be finished in syncing or reorganisation states
            }
        }
    }

    /// Transition to a new state
    pub fn transition(&self, new_state: State) {
        let mut state_write = self.state.write().unwrap();

        let old_state = state_write.clone();

        match (old_state, new_state) {
            (State::ReOrg, State::Active) => {
                while let Ok(sender) = self.resume_queue.pop() {
                    sender.send(true).unwrap();
                }
            }
            _ => (),
        }
        *state_write = new_state;
    }

    /// Increment sync position
    pub fn increment_sync_position(&self) -> u32 {
        self.sync_position.fetch_add(1, Ordering::SeqCst)
    }

    /// Sync position
    pub fn set_sync_position(&self, position: u32) {
        self.sync_position.store(position, Ordering::SeqCst);
    }

    /// Get sync position
    pub fn sync_position(&self) -> u32 {
        self.sync_position.load(Ordering::SeqCst)
    }
}

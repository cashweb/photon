use std::sync::{
    atomic::{AtomicUsize, Ordering},
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
}

impl Default for StateMananger {
    fn default() -> Self {
        StateMananger {
            state: RwLock::new(State::Syncing),
            resume_queue: ArrayQueue::new(RESUME_QUEUE_SIZE),
            active_counter: CachePadded::new(AtomicUsize::new(0)),
        }
    }
}

impl StateMananger {
    /// Check whether service can process a request in current state
    /// Returns a oneshot receiver representing readiness
    /// Receiving true through the channel implies receiver to should process request
    /// Receiving false through the channel implies receiver should reject request
    /// Receiving None indicates that the request should be immediately processed
    pub fn accepting(&self) -> Option<oneshot::Receiver<bool>> {
        let state_read = self.state.read().unwrap();
        match *state_read {
            State::Syncing => {
                // Init new resume channel
                let (mut send, recv) = oneshot::channel::<bool>();

                // Keep trying to push until space
                while let Err(PushError(err_send)) = self.resume_queue.push(send) {
                    send = err_send;
                    // Expell oldest from resume queue with false
                    if let Ok(expelled) = self.resume_queue.pop() {
                        expelled.send(false).unwrap();
                    }
                }
                Some(recv)
            }
            State::Active => {
                // Increment active request counter
                self.active_counter.fetch_add(1, Ordering::Relaxed);
                None
            }
            State::ReOrgPending => {
                // Init new resume channel
                let (mut send, recv) = oneshot::channel::<bool>();

                // Keep trying to push until space
                while let Err(PushError(err_send)) = self.resume_queue.push(send) {
                    send = err_send;
                    // Expell oldest from resume queue with false
                    if let Ok(expelled) = self.resume_queue.pop() {
                        expelled.send(false).unwrap();
                    }
                }

                Some(recv)
            }
            State::ReOrg => {
                // Init new resume channel
                let (mut send, recv) = oneshot::channel::<bool>();

                // Keep trying to push until space
                while let Err(PushError(err_send)) = self.resume_queue.push(send) {
                    send = err_send;
                    // Expell oldest from resume queue with false
                    if let Ok(expelled) = self.resume_queue.pop() {
                        expelled.send(false).unwrap();
                    }
                }

                Some(recv)
            }
        }
    }

    pub fn finished(&self) {
        let state_read = self.state.read().unwrap();
        match *state_read {
            State::Active => {
                self.active_counter.fetch_sub(1, Ordering::Relaxed);
            }
            State::ReOrgPending => {
                let active_count = self.active_counter.fetch_sub(1, Ordering::Relaxed);
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
}

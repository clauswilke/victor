// Copyright 2016 Amanieu d'Antras
//
// Licensed under the Apache License, Version 2.0, <LICENSE-APACHE or
// http://apache.org/licenses/LICENSE-2.0> or the MIT license <LICENSE-MIT or
// http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.

use std::sync::atomic::{AtomicUsize as AtomicU8, ATOMIC_USIZE_INIT as ATOMIC_U8_INIT, Ordering};
use parking_lot_core::{self, UnparkResult, SpinWait, UnparkToken, DEFAULT_PARK_TOKEN};

type U8 = usize;

// UnparkToken used to indicate that that the target thread should attempt to
// lock the mutex again as soon as it is unparked.
pub const TOKEN_NORMAL: UnparkToken = UnparkToken(0);

// UnparkToken used to indicate that the mutex is being handed off to the target
// thread directly without unlocking it.
pub const TOKEN_HANDOFF: UnparkToken = UnparkToken(1);

const LOCKED_BIT: U8 = 1;
const PARKED_BIT: U8 = 2;

pub struct RawMutex {
    state: AtomicU8,
}

pub const RAW_MUTEX_INIT: RawMutex = RawMutex {
    state: ATOMIC_U8_INIT,
};

impl RawMutex {
    #[inline]
    pub fn lock(&self) {
        if self.state
            .compare_exchange_weak(0, LOCKED_BIT, Ordering::Acquire, Ordering::Relaxed)
            .is_ok() {
            return;
        }
        self.lock_slow();
    }

    #[inline]
    pub fn unlock(&self) {
        if self.state
            .compare_exchange_weak(LOCKED_BIT, 0, Ordering::Release, Ordering::Relaxed)
            .is_ok() {
            return;
        }
        self.unlock_slow(false);
    }

    #[cold]
    #[inline(never)]
    fn lock_slow(&self) {
        let mut spinwait = SpinWait::new();
        let mut state = self.state.load(Ordering::Relaxed);
        loop {
            // Grab the lock if it isn't locked, even if there is a queue on it
            if state & LOCKED_BIT == 0 {
                match self.state
                    .compare_exchange_weak(state,
                                           state | LOCKED_BIT,
                                           Ordering::Acquire,
                                           Ordering::Relaxed) {
                    Ok(_) => return,
                    Err(x) => state = x,
                }
                continue;
            }

            // If there is no queue, try spinning a few times
            if state & PARKED_BIT == 0 && spinwait.spin() {
                state = self.state.load(Ordering::Relaxed);
                continue;
            }

            // Set the parked bit
            if state & PARKED_BIT == 0 {
                if let Err(x) = self.state.compare_exchange_weak(state,
                                                                 state | PARKED_BIT,
                                                                 Ordering::Relaxed,
                                                                 Ordering::Relaxed) {
                    state = x;
                    continue;
                }
            }

            // Park our thread until we are woken up by an unlock
            unsafe {
                let addr = self as *const _ as usize;
                let validate = || self.state.load(Ordering::Relaxed) == LOCKED_BIT | PARKED_BIT;
                let before_sleep = || {};
                let timed_out = |_, _| unreachable!();
                if parking_lot_core::park(addr,
                                          validate,
                                          before_sleep,
                                          timed_out,
                                          DEFAULT_PARK_TOKEN,
                                          None) == Some(TOKEN_HANDOFF) {
                    // The thread that unparked us passed the lock on to us
                    // directly without unlocking it.
                    return;
                }
            }

            // Loop back and try locking again
            spinwait.reset();
            state = self.state.load(Ordering::Relaxed);
        }
    }

    #[cold]
    #[inline(never)]
    fn unlock_slow(&self, force_fair: bool) {
        // Unlock directly if there are no parked threads
        if self.state
            .compare_exchange(LOCKED_BIT, 0, Ordering::Release, Ordering::Relaxed)
            .is_ok() {
            return;
        }

        // Unpark one thread and leave the parked bit set if there might
        // still be parked threads on this address.
        unsafe {
            let addr = self as *const _ as usize;
            let callback = |result: UnparkResult| {
                // If we are using a fair unlock then we should keep the
                // mutex locked and hand it off to the unparked thread.
                if result.unparked_threads != 0 && (force_fair || result.be_fair) {
                    // Clear the parked bit if there are no more parked
                    // threads.
                    if !result.have_more_threads {
                        self.state.store(LOCKED_BIT, Ordering::Relaxed);
                    }
                    return TOKEN_HANDOFF;
                }

                // Clear the locked bit, and the parked bit as well if there
                // are no more parked threads.
                if result.have_more_threads {
                    self.state.store(PARKED_BIT, Ordering::Release);
                } else {
                    self.state.store(0, Ordering::Release);
                }
                TOKEN_NORMAL
            };
            parking_lot_core::unpark_one(addr, callback);
        }
    }
}
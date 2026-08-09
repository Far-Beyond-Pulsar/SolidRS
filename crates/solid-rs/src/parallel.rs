//! Optional multi-threaded map helpers.
//!
//! This module is the single place where format crates opt into parallelism.
//! It is backed by `rayon` behind the `parallel` feature; when that feature is
//! off every helper degrades to a plain serial iteration, so callers get
//! deterministic, identical output in either case.
//!
//! Thread count is derived from [`LoadOptions::num_threads`](crate::traits::LoadOptions::num_threads)
//! / [`SaveOptions::num_threads`](crate::traits::SaveOptions::num_threads):
//!
//! | `num_threads` | Behaviour                                             |
//! |---------------|-------------------------------------------------------|
//! | `None`        | Auto — use the rayon global pool (feature on) or serial (feature off) |
//! | `Some(1)`     | Always serial — the determinism escape hatch           |
//! | `Some(n > 1)` | A dedicated pool of `n` worker threads                 |
//!
//! Parallel mapping is **order-preserving**: the returned `Vec` is in the same
//! order as the input slice regardless of how many threads ran, so results are
//! identical to a serial map.

#[cfg(feature = "parallel")]
use std::sync::Arc;

/// How to run parallel work for one load/save operation.
///
/// Construct it once per operation (from `num_threads`) and reuse it for every
/// inner map so a requested worker pool is built a single time, not per item.
#[derive(Debug)]
pub struct Parallelism {
    #[cfg(feature = "parallel")]
    pool: Option<Arc<rayon::ThreadPool>>,
}

impl Default for Parallelism {
    fn default() -> Self {
        Self::from_num_threads(None)
    }
}

impl Parallelism {
    /// Builds a parallelism plan from a `num_threads` option.
    pub fn from_num_threads(num_threads: Option<usize>) -> Self {
        #[cfg(feature = "parallel")]
        {
            let pool = match num_threads {
                // `Some(1)` forces serial — handled by `map`.
                Some(1) | Some(0) | None => None,
                Some(n) => Some(Arc::new(
                    rayon::ThreadPoolBuilder::new()
                        .num_threads(n)
                        .build()
                        .expect("failed to build rayon thread pool"),
                )),
            };
            Self { pool }
        }
        #[cfg(not(feature = "parallel"))]
        {
            let _ = num_threads;
            Self {}
        }
    }

    /// Returns `true` when a worker pool is available and parallel mapping
    /// will actually be used.
    pub fn is_parallel(&self) -> bool {
        #[cfg(feature = "parallel")]
        {
            self.pool.is_some() || rayon::current_num_threads() > 1
        }
        #[cfg(not(feature = "parallel"))]
        {
            false
        }
    }

    /// Maps `items` through `f`, in input order.
    ///
    /// Runs in parallel when the `parallel` feature is enabled and this
    /// [`Parallelism`] was not created with `Some(1)`. Otherwise it is a plain
    /// serial `map` with identical output.
    pub fn map<A, B>(&self, items: &[A], f: impl Fn(&A) -> B + Sync + Send) -> Vec<B>
    where
        A: Sync,
        B: Send,
    {
        if items.len() <= 1 {
            return items.iter().map(f).collect();
        }
        #[cfg(feature = "parallel")]
        {
            use rayon::prelude::*;
            match &self.pool {
                Some(pool) => pool.install(|| items.par_iter().map(f).collect()),
                None => items.par_iter().map(f).collect(),
            }
        }
        #[cfg(not(feature = "parallel"))]
        {
            items.iter().map(f).collect()
        }
    }
}

//! A small parallel map over a slice: results come back in input order, so output does not
//! depend on scheduling.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread;

/// `f` applied to every item on up to `jobs` threads (the calling thread included). If a thread
/// cannot be started the others do its share.
///
/// # Errors
/// When a worker thread panicked (a bug; reported, never re-raised).
pub fn map<T: Sync, R: Send>(items: &[T], jobs: usize, f: impl Fn(&T) -> R + Sync) -> Result<Vec<R>, String> {
    let next = AtomicUsize::new(0);
    let work = || {
        let mut done = Vec::new();
        loop {
            let i = next.fetch_add(1, Ordering::Relaxed);
            let Some(item) = items.get(i) else { break };
            done.push((i, f(item)));
        }
        done
    };
    let mut results: Vec<(usize, R)> = thread::scope(|s| {
        let handles: Vec<_> = (1..jobs.max(1)).filter_map(|_| thread::Builder::new().spawn_scoped(s, work).ok()).collect();
        let mut all = work();
        for h in handles {
            all.extend(h.join().map_err(|_| "a worker thread failed".to_owned())?);
        }
        Ok::<_, String>(all)
    })?;
    results.sort_by_key(|(i, _)| *i);
    Ok(results.into_iter().map(|(_, r)| r).collect())
}

/// The default number of threads.
pub fn default_jobs() -> usize {
    thread::available_parallelism().map_or(1, std::num::NonZeroUsize::get)
}

#[cfg(test)]
mod tests {
    #[test]
    fn keeps_order() {
        let items: Vec<u32> = (0..1000).collect();
        let out = super::map(&items, 8, |x| x * 2).unwrap();
        assert_eq!(out, items.iter().map(|x| x * 2).collect::<Vec<_>>());
        assert_eq!(super::map(&items, 0, |x| *x).unwrap().len(), 1000);
    }
}

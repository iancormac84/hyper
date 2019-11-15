use std::fmt;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use crate::body::{Payload, Body};
use crate::proto::h2::server::H2Stream;
use crate::server::conn::spawn_all::{NewSvcTask, Watcher};
use crate::service::HttpService;

pub trait H2Exec<F, B: Payload>: Clone {
    fn execute_h2stream(&self, fut: H2Stream<F, B>);
}

pub trait NewSvcExec<I, N, S: HttpService<Body>, E, W: Watcher<I, S, E>>: Clone {
    fn execute_new_svc(&self, fut: NewSvcTask<I, N, S, E, W>);
}

pub type BoxFuture = Pin<Box<dyn Future<Output=()> + Send>>;

// Either the user provides an executor for background tasks, or we use
// `tokio::spawn`.
#[derive(Clone)]
pub enum Exec {
    Default,
    Executor(Arc<dyn Fn(BoxFuture) + Send + Sync>),
}

// ===== impl Exec =====

impl Exec {
    pub(crate) fn execute<F>(&self, fut: F)
    where
        F: Future<Output=()> + Send + 'static,
    {
        match *self {
            Exec::Default => {
                #[cfg(feature = "tcp")]
                {
                    tokio::spawn(fut);
                }
                #[cfg(not(feature = "tcp"))]
                {
                    // If no runtime, we need an executor!
                    panic!("executor must be set")
                }
            },
            Exec::Executor(ref e) => {
                e(Box::pin(fut));
            },
        }
    }
}

impl fmt::Debug for Exec {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Exec::Default => f.write_str("Exec::Default"),
            Exec::Executor(..) => f.write_str("Exec::Custom"),
        }
    }
}


impl<F, B> H2Exec<F, B> for Exec
where
    H2Stream<F, B>: Future<Output = ()> + Send + 'static,
    B: Payload,
{
    fn execute_h2stream(&self, fut: H2Stream<F, B>) {
        self.execute(fut)
    }
}

impl<I, N, S, E, W> NewSvcExec<I, N, S, E, W> for Exec
where
    NewSvcTask<I, N, S, E, W>: Future<Output=()> + Send + 'static,
    S: HttpService<Body>,
    W: Watcher<I, S, E>,
{
    fn execute_new_svc(&self, fut: NewSvcTask<I, N, S, E, W>) {
        self.execute(fut)
    }
}

// ==== impl Executor =====

impl<E, F, B> H2Exec<F, B> for E
where
    E: Fn(H2Stream<F, B>) + Clone,
    H2Stream<F, B>: Future<Output=()>,
    B: Payload,
{
    fn execute_h2stream(&self, fut: H2Stream<F, B>) {
        self(fut);
    }
}

impl<I, N, S, E, W> NewSvcExec<I, N, S, E, W> for E
where
    E: Fn(NewSvcTask<I, N, S, E, W>) + Clone,
    NewSvcTask<I, N, S, E, W>: Future<Output=()>,
    S: HttpService<Body>,
    W: Watcher<I, S, E>,
{
    fn execute_new_svc(&self, fut: NewSvcTask<I, N, S, E, W>) {
        self(fut);
    }
}

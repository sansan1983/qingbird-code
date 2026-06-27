use qbird_code_models::EflowError;
use std::future::Future;
use std::sync::Arc;
use tokio::sync::Semaphore;

pub struct SubagentPool {
    semaphore: Arc<Semaphore>,
}

impl SubagentPool {
    pub fn new(size: usize) -> Self {
        Self {
            semaphore: Arc::new(Semaphore::new(size.max(1))),
        }
    }

    pub fn size(&self) -> usize {
        self.semaphore.available_permits()
    }

    pub fn semaphore(&self) -> Arc<Semaphore> {
        self.semaphore.clone()
    }
}

pub async fn execute_parallel<F, T>(tasks: Vec<F>) -> Vec<Result<T, EflowError>>
where
    F: Future<Output = Result<T, EflowError>> + Send + 'static,
    T: Send + 'static,
{
    use futures_util::stream::{FuturesUnordered, StreamExt};
    let mut futures: FuturesUnordered<tokio::task::JoinHandle<Result<T, EflowError>>> =
        FuturesUnordered::new();
    for task in tasks {
        futures.push(tokio::spawn(task));
    }
    let mut results = Vec::new();
    while let Some(result) = futures.next().await {
        match result {
            Ok(Ok(val)) => results.push(Ok(val)),
            Ok(Err(e)) => results.push(Err(e)),
            Err(join_err) => results.push(Err(EflowError::Internal(format!(
                "Task panicked: {}",
                join_err
            )))),
        }
    }
    results
}

pub async fn execute_parallel_limited<F, T>(
    pool: &SubagentPool,
    tasks: Vec<F>,
) -> Vec<Result<T, EflowError>>
where
    F: Future<Output = Result<T, EflowError>> + Send + 'static,
    T: Send + 'static,
{
    let sem = pool.semaphore();
    let mut handles = Vec::with_capacity(tasks.len());
    for task in tasks {
        let permit = sem.clone().acquire_owned().await.unwrap();
        handles.push(tokio::spawn(async move {
            let result = task.await;
            drop(permit);
            result
        }));
    }

    let mut results = Vec::new();
    for handle in handles {
        match handle.await {
            Ok(Ok(val)) => results.push(Ok(val)),
            Ok(Err(e)) => results.push(Err(e)),
            Err(join_err) => results.push(Err(EflowError::Internal(format!(
                "Task panicked: {}",
                join_err
            )))),
        }
    }
    results
}

use qbird_code_models::EflowError;
use std::future::Future;

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

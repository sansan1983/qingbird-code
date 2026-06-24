use std::sync::Arc;
use std::time::Instant;

use qingbird_code::capability::pool::SubagentPool;
use qingbird_code::common::types::Role;

#[tokio::test]
async fn pool_dispatches_concurrently_within_capacity() {
    // v1.1 M10.5 Task C6: Arc<SubagentPool> 在多 task 间共享，
    // 4 worker 并发派发 10 个任务，验证 dispatch 全部成功 + 时延合理
    let pool = Arc::new(SubagentPool::start(4));
    let start = Instant::now();
    let mut handles = vec![];
    for _ in 0..10 {
        let p = pool.clone();
        handles.push(tokio::spawn(async move {
            p.dispatch_for_role(Role::CodeAssistant).await
        }));
    }
    let mut count = 0;
    for h in handles {
        if h.await.unwrap().is_ok() {
            count += 1;
        }
    }
    let elapsed = start.elapsed();
    assert_eq!(count, 10);
    // 4 worker 并发，10 个任务应在 3 轮内完成（验证并发）
    assert!(elapsed.as_millis() < 1000, "took {:?}", elapsed);
    pool.shutdown().await;
}

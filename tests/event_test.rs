rust_i18n::i18n!("locales", fallback = "en-US");

use qingbird_code::infrastructure::event::{Event, EventChannel};
use uuid::Uuid;

#[tokio::test]
async fn test_publish_and_receive() {
    let channel = EventChannel::new();
    let mut rx = channel.subscribe();

    let task_id = Uuid::new_v4();
    channel.publish(Event::TaskStarted {
        task_id,
        description: "test task".into(),
    });

    let received = rx.recv().await.unwrap();
    match received {
        Event::TaskStarted {
            task_id: id,
            description,
        } => {
            assert_eq!(id, task_id);
            assert_eq!(description, "test task");
        }
        _ => panic!("expected TaskStarted"),
    }
}

#[tokio::test]
async fn test_multiple_subscribers() {
    let channel = EventChannel::new();
    let mut rx1 = channel.subscribe();
    let mut rx2 = channel.subscribe();

    channel.publish(Event::SystemShutdown);

    let e1 = rx1.recv().await.unwrap();
    let e2 = rx2.recv().await.unwrap();
    assert!(matches!(e1, Event::SystemShutdown));
    assert!(matches!(e2, Event::SystemShutdown));
}

#[tokio::test]
async fn test_publish_with_no_subscribers_does_not_panic() {
    let channel = EventChannel::new();
    // No subscribers — should not panic, just silently dropped
    channel.publish(Event::SystemShutdown);
    // Late subscriber should not see the dropped event
    let mut rx = channel.subscribe();
    let result = tokio::time::timeout(std::time::Duration::from_millis(50), rx.recv()).await;
    assert!(result.is_err(), "expected timeout, got {:?}", result);
}

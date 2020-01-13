use monza::Buffer;
use tracing_subscriber::{EnvFilter, FmtSubscriber};

fn main() {
    let subscriber = FmtSubscriber::builder()
    .with_env_filter(EnvFilter::from_default_env())
    .finish();
    let _ = tracing::dispatcher::set_global_default(tracing::Dispatch::new(subscriber));

    let buf = Buffer::new(256);

    buf.push(10);
    assert_eq!(buf.pop(), Some(10));
}

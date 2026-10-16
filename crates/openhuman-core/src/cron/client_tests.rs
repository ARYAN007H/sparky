use super::*;
use std::time::Instant;

#[tokio::test]
async fn timing_probe() {
    let t = Instant::now();
    let _ = crate::modules::runtime::runtime().await.unwrap();
    eprintln!("runtime: {:?}", t.elapsed());
    let t = Instant::now();
    crate::modules::tinyflows::fixture_attach_for_probe().await.unwrap();
    eprintln!("attach: {:?}", t.elapsed());
}

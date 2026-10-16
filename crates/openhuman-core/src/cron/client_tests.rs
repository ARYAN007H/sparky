use super::*;
use std::time::Instant;

#[tokio::test]
async fn timing_probe() {
    let config = Config::default();
    let t = Instant::now();
    crate::modules::tinyflows::proxy(&config).await.unwrap();
    eprintln!("attach: {:?}", t.elapsed());
    let t = Instant::now();
    let normalized = normalize_expression(&config, "0 9 * * 1-5").await.unwrap();
    eprintln!("first call: {:?} -> {normalized}", t.elapsed());
    let t = Instant::now();
    normalize_expression(&config, "0 9 * * 1-5").await.unwrap();
    eprintln!("second call: {:?}", t.elapsed());
}

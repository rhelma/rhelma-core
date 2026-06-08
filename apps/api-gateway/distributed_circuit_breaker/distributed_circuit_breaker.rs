rust
use redis::AsyncCommands;
use std::time::Duration;

pub struct DistributedCircuitBreaker {
    redis_url: String,
    service_name: String,
    failure_threshold: u32,
    recovery_timeout: Duration,
}

impl DistributedCircuitBreaker {
    pub fn new(redis_url: String, service_name: String, failure_threshold: u32, recovery_timeout: Duration) -> Self {
        DistributedCircuitBreaker {
            redis_url,
            service_name,
            failure_threshold,
            recovery_timeout,
        }
    }

    pub async fn is_open(&self) -> bool {
        let client = redis::Client::open(self.redis_url.clone()).expect("Invalid Redis URL");
        let mut con = client.get_async_connection().await.unwrap();

        // Get the failure count for the service
        let failures: u32 = con.get(&self.service_name).await.unwrap_or(0);

        failures >= self.failure_threshold
    }

    pub async fn record_failure(&self) {
        let client = redis::Client::open(self.redis_url.clone()).expect("Invalid Redis URL");
        let mut con = client.get_async_connection().await.unwrap();

        // Increment the failure count for the service
        let _: () = con.incr(&self.service_name, 1).await.unwrap();
    }

    pub async fn reset(&self) {
        let client = redis::Client::open(self.redis_url.clone()).expect("Invalid Redis URL");
        let mut con = client.get_async_connection().await.unwrap();

        // Reset the failure count for the service
        let _: () = con.del(&self.service_name).await.unwrap();
    }
}

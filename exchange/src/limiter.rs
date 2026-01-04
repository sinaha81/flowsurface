use crate::adapter::AdapterError;

use reqwest::{Client, Method, Response};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::time::{Duration, Instant};
use std::sync::{RwLock, LazyLock};

use std::sync::atomic::Ordering;

pub fn get_client() -> Client {
    PROXY_MANAGER.read().unwrap().get_best_instance().map(|i| i.client.clone()).unwrap_or_else(|| Client::new())
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum ProxyProtocol {
    #[default]
    Auto,
    Http,
    Https,
    Socks4,
    Socks5,
}

impl ProxyProtocol {
    pub fn all() -> &'static [Self] {
        &[Self::Auto, Self::Http, Self::Https, Self::Socks4, Self::Socks5]
    }
}

impl std::fmt::Display for ProxyProtocol {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProxyProtocol::Auto => write!(f, "auto"),
            ProxyProtocol::Http => write!(f, "http"),
            ProxyProtocol::Https => write!(f, "https"),
            ProxyProtocol::Socks4 => write!(f, "socks4"),
            ProxyProtocol::Socks5 => write!(f, "socks5"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CustomProxy {
    pub protocol: ProxyProtocol,
    pub host: String,
    pub port: u16,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct CustomProxyOuter {
    #[serde(rename = "Custom")]
    inner: CustomProxy,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct CustomProxyVec {
    #[serde(rename = "Custom")]
    inner: Vec<CustomProxy>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub enum ProxyConfig {
    System,
    None,
    Custom(Vec<CustomProxy>),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
enum ProxyConfigDe {
    String(String),
    SingleTagged(CustomProxyOuter),
    MultiTagged(CustomProxyVec),
    MultiUntagged(Vec<CustomProxy>),
    SingleUntagged(CustomProxy),
}

impl<'de> Deserialize<'de> for ProxyConfig {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let de = ProxyConfigDe::deserialize(deserializer)?;
        match de {
            ProxyConfigDe::String(s) => match s.as_str() {
                "System" => Ok(ProxyConfig::System),
                "None" => Ok(ProxyConfig::None),
                _ => Err(serde::de::Error::custom(format!("Invalid ProxyConfig string: {}", s))),
            },
            ProxyConfigDe::SingleTagged(s) => Ok(ProxyConfig::Custom(vec![s.inner])),
            ProxyConfigDe::MultiTagged(m) => Ok(ProxyConfig::Custom(m.inner)),
            ProxyConfigDe::MultiUntagged(v) => Ok(ProxyConfig::Custom(v)),
            ProxyConfigDe::SingleUntagged(p) => Ok(ProxyConfig::Custom(vec![p])),
        }
    }
}

impl Default for ProxyConfig {
    fn default() -> Self {
        Self::System
    }
}

use std::sync::atomic::AtomicUsize;
use std::sync::Arc;

pub struct ProxyHealth {
    pub is_healthy: bool,
    pub last_success: Option<Instant>,
    pub failure_count: usize,
    pub avg_latency: Duration,
}

impl Default for ProxyHealth {
    fn default() -> Self {
        Self {
            is_healthy: true,
            last_success: None,
            failure_count: 0,
            avg_latency: Duration::from_millis(0),
        }
    }
}

pub struct ProxyInstance {
    pub config: Option<CustomProxy>, // None means System or Direct
    pub client: Client,
    pub health: Arc<RwLock<ProxyHealth>>,
}

pub struct ProxyManager {
    pub instances: Vec<ProxyInstance>,
    pub rotation_index: AtomicUsize,
}

impl ProxyManager {
    pub fn new(config: ProxyConfig) -> Result<Self, AdapterError> {
        let mut instances = Vec::new();

        match config {
            ProxyConfig::System => {
                instances.push(ProxyInstance {
                    config: None,
                    client: Client::builder().build().map_err(AdapterError::FetchError)?,
                    health: Arc::new(RwLock::new(ProxyHealth::default())),
                });
            }
            ProxyConfig::None => {
                instances.push(ProxyInstance {
                    config: None,
                    client: Client::builder()
                        .no_proxy()
                        .build()
                        .map_err(AdapterError::FetchError)?,
                    health: Arc::new(RwLock::new(ProxyHealth::default())),
                });
            }
            ProxyConfig::Custom(proxies) => {
                for custom in proxies {
                    let url = if custom.protocol == ProxyProtocol::Auto {
                        if custom.host.contains("://") {
                            custom.host.clone()
                        } else if custom.port == 1080
                            || custom.port == 1081
                            || custom.port == 7890
                            || custom.port == 7891
                        {
                            format!("socks5://{}:{}", custom.host, custom.port)
                        } else {
                            format!("http://{}:{}", custom.host, custom.port)
                        }
                    } else {
                        format!("{}://{}:{}", custom.protocol, custom.host, custom.port)
                    };

                    let proxy = reqwest::Proxy::all(&url)
                        .map_err(|e| AdapterError::InvalidRequest(e.to_string()))?;
                    let client = Client::builder()
                        .proxy(proxy)
                        .timeout(Duration::from_secs(10))
                        .build()
                        .map_err(AdapterError::FetchError)?;

                    instances.push(ProxyInstance {
                        config: Some(custom),
                        client,
                        health: Arc::new(RwLock::new(ProxyHealth::default())),
                    });
                }
            }
        }

        Ok(Self {
            instances,
            rotation_index: AtomicUsize::new(0),
        })
    }

    pub fn get_best_instance(&self) -> Option<&ProxyInstance> {
        if self.instances.is_empty() {
            return None;
        }

        let start_idx = self
            .rotation_index
            .fetch_add(1, Ordering::Relaxed) % self.instances.len();
        
        // Try to find a healthy instance starting from rotation_index
        for i in 0..self.instances.len() {
            let idx = (start_idx + i) % self.instances.len();
            let inst = &self.instances[idx];
            if inst.health.read().unwrap().is_healthy {
                return Some(inst);
            }
        }

        // If all unhealthy, return the one with least failures or just the first one
        self.instances.get(start_idx)
    }

    // We could add a background task here to re-enable healthy proxies
}

pub static PROXY_MANAGER: LazyLock<RwLock<ProxyManager>> = LazyLock::new(|| {
    RwLock::new(ProxyManager::new(ProxyConfig::System).unwrap())
});

pub fn set_global_proxy(config: ProxyConfig) -> Result<(), AdapterError> {
    let manager = ProxyManager::new(config)?;
    
    // Acquire write lock and replace the manager
    if let Ok(mut guard) = PROXY_MANAGER.write() {
        *guard = manager;
    } else {
        return Err(AdapterError::WebsocketError("Failed to acquire lock on ProxyManager".to_string()));
    }

    // Spawn a background health checker if there are custom proxies
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(60));
        loop {
            interval.tick().await;
            
            let instances_to_check: Vec<_> = {
                let manager_guard = PROXY_MANAGER.read().unwrap();
                manager_guard.instances.iter()
                    .filter(|inst| !inst.health.read().unwrap().is_healthy)
                    .map(|inst| (inst.client.clone(), inst.health.clone()))
                    .collect()
            };

            for (client, health_tracker) in instances_to_check {
                log::info!("Performing background health check for proxy...");
                let test_url = "https://www.google.com";
                let start = Instant::now();
                match client.get(test_url).timeout(Duration::from_secs(5)).send().await {
                    Ok(_) => {
                        let latency = start.elapsed();
                        let mut h = health_tracker.write().unwrap();
                        h.is_healthy = true;
                        h.failure_count = 0;
                        h.last_success = Some(Instant::now());
                        h.avg_latency = latency;
                        log::info!("Proxy recovered and is now healthy");
                    }
                    Err(_) => {
                        log::debug!("Proxy still unhealthy");
                    }
                }
            }
        }
    });

    Ok(())
}

pub trait RateLimiter: Send + Sync {
    /// Prepare for a request with given weight. Returns wait time if needed.
    fn prepare_request(&mut self, weight: usize) -> Option<Duration>;

    /// Update the limiter with response data (e.g., rate limit headers)
    fn update_from_response(&mut self, response: &Response, weight: usize);

    /// Check if response indicates rate limiting and should exit
    fn should_exit_on_response(&self, response: &Response) -> bool;
}

pub async fn http_request_with_limiter<L: RateLimiter>(
    url: &str,
    limiter: &tokio::sync::Mutex<L>,
    weight: usize,
    method: Option<Method>,
    json_body: Option<&Value>,
) -> Result<String, AdapterError> {
    let method = method.unwrap_or(Method::GET);

    let mut limiter_guard = limiter.lock().await;

    if let Some(wait_time) = limiter_guard.prepare_request(weight) {
        log::warn!("Rate limit hit for: {url}. Waiting for {:?}", wait_time);
        tokio::time::sleep(wait_time).await;
    }

    let max_retries = 5;
    let mut last_error = None;

    for attempt in 0..max_retries {
        let (client, health_tracker, proxy_info) = {
            let manager = PROXY_MANAGER.read().unwrap();
            let instance = match manager.get_best_instance() {
                Some(i) => i,
                None => {
                    return Err(AdapterError::AllProxiesFailed("No healthy proxy instances available".to_string()));
                }
            };
            let proxy_info = instance.config.as_ref()
                .map(|c| format!("{}:{}", c.host, c.port))
                .unwrap_or_else(|| "System/Direct".to_string());
            (instance.client.clone(), instance.health.clone(), proxy_info)
        };

        let mut request_builder = client.request(method.clone(), url)
            .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
            .header("Accept", "application/json");
        if let Some(body) = json_body {
            request_builder = request_builder.json(body);
        }

        let start = Instant::now();
        match request_builder.send().await {
            Ok(response) => {
                let latency = start.elapsed();
                
                // Update health on success
                if let Ok(mut h) = health_tracker.write() {
                    h.is_healthy = true;
                    h.last_success = Some(Instant::now());
                    h.failure_count = 0;
                    // Exponential moving average for latency
                    let alpha = 0.2;
                    h.avg_latency = Duration::from_secs_f32(
                        h.avg_latency.as_secs_f32() * (1.0 - alpha) + latency.as_secs_f32() * alpha
                    );
                }

                if limiter_guard.should_exit_on_response(&response) {
                    let status = response.status();
                    log::error!(
                        "HTTP error {} for: {}. (This may be a rate limit, geo-block, or other access issue.)",
                        status,
                        url
                    );
                    // If it's a 451 (Geo-blocked) or similar, maybe mark proxy as bad for this site?
                    // For now we just return error
                }

                limiter_guard.update_from_response(&response, weight);
                let body = response.text().await.map_err(AdapterError::FetchError)?;

                // Detect HTML responses which usually mean the proxy is blocked/challenged
                let trimmed = body.trim();
                if trimmed.starts_with("<!") || trimmed.to_lowercase().starts_with("<html") {
                    log::warn!(
                        "Received HTML response for {} via {}. Proxy might be blocked. Retrying...",
                        url, proxy_info
                    );
                    if let Ok(mut h) = health_tracker.write() {
                        h.failure_count += 1;
                        if h.failure_count >= 3 {
                            h.is_healthy = false;
                        }
                    }
                    last_error = Some(AdapterError::ParseError(format!("Received HTML instead of JSON via {}: {}", proxy_info, &trimmed[..trimmed.len().min(50)])));
                    tokio::time::sleep(Duration::from_millis(500)).await;
                    continue;
                }

                return Ok(body);
            }
            Err(e) => {
                log::error!("Attempt {} failed for {}: {}", attempt + 1, url, e);
                
                // Update health on failure
                if let Ok(mut h) = health_tracker.write() {
                    h.failure_count += 1;
                    if h.failure_count >= 3 {
                        h.is_healthy = false;
                        log::warn!("Proxy marked as unhealthy due to repeated failures");
                    }
                }
                
                last_error = Some(AdapterError::FetchError(e));
                
                // Small delay before retry with (hopefully) another proxy
                tokio::time::sleep(Duration::from_millis(500)).await;
            }
        }
    }

    Err(AdapterError::AllProxiesFailed(format!("Failed after {} attempts. Last error: {:?}", max_retries, last_error)))
}

pub async fn http_parse_with_limiter<L, V>(
    url: &str,
    limiter: &tokio::sync::Mutex<L>,
    weight: usize,
    method: Option<Method>,
    json_body: Option<&Value>,
) -> Result<V, AdapterError>
where
    L: RateLimiter,
    V: serde::de::DeserializeOwned,
{
    let method = method.unwrap_or(Method::GET);

    let body = http_request_with_limiter(url, limiter, weight, Some(method), json_body).await?;
    let trimmed = body.trim();

    let body_preview = |body: &str, n: usize| {
        let trimmed = body.trim();
        let mut preview = trimmed.chars().take(n).collect::<String>();
        if trimmed.len() > n {
            preview.push('…');
        }
        preview
    };

    if trimmed.is_empty() {
        let msg = format!("Empty response body | url={url}");
        log::error!("{}", msg);
        return Err(AdapterError::ParseError(msg));
    }
    if trimmed.starts_with('<') {
        let msg = format!(
            "Non-JSON (HTML?) response | url={} | len={} | preview={:?}",
            url,
            body.len(),
            body_preview(&body, 200)
        );
        log::error!("{}", msg);
        return Err(AdapterError::ParseError(msg));
    }

    serde_json::from_str(&body).map_err(|e| {
        let msg = format!(
            "JSON parse failed: {} | url={} | response_len={} | preview={:?}",
            e,
            url,
            body.len(),
            body_preview(&body, 200)
        );
        log::error!("{}", msg);
        AdapterError::ParseError(msg)
    })
}

/// Limiter for a fixed window rate
pub struct FixedWindowBucket {
    max_tokens: usize,
    available_tokens: usize,
    last_refill: Instant,
    refill_rate: Duration,
}

impl FixedWindowBucket {
    pub fn new(max_tokens: usize, refill_rate: Duration) -> Self {
        Self {
            max_tokens,
            available_tokens: max_tokens,
            last_refill: Instant::now(),
            refill_rate,
        }
    }

    fn refill(&mut self) {
        if let Ok(current_time) = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH)
        {
            let now = Instant::now();
            let period_seconds = self.refill_rate.as_secs();
            let seconds_in_current_period = current_time.as_secs() % period_seconds;

            let elapsed = now.duration_since(self.last_refill);
            if elapsed >= self.refill_rate || seconds_in_current_period < 1 {
                self.available_tokens = self.max_tokens;
                self.last_refill = now;
            }
        }
    }

    pub fn calculate_wait_time(&mut self, tokens: usize) -> Option<Duration> {
        self.refill();

        if self.available_tokens >= tokens {
            self.available_tokens -= tokens;
            return None;
        }

        let wait_time = self
            .refill_rate
            .saturating_sub(Instant::now().duration_since(self.last_refill));
        Some(wait_time)
    }

    pub fn consume_tokens(&mut self, tokens: usize) {
        self.refill();
        self.available_tokens -= tokens.min(self.available_tokens);
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DynamicLimitReason {
    HeaderRate,
    FixedWindowRate,
}

/// Limiter that can be used when source reports the rate-limit usage
///
/// Can fallback to fixed window bucket
pub struct DynamicBucket {
    max_weight: usize,
    current_used_weight: usize,
    last_updated: Instant,
    refill_rate: Duration,
    fallback_bucket: FixedWindowBucket,
}

impl DynamicBucket {
    pub fn new(max_weight: usize, refill_rate: Duration) -> Self {
        Self {
            max_weight,
            current_used_weight: 0,
            last_updated: Instant::now(),
            refill_rate,
            fallback_bucket: FixedWindowBucket::new(max_weight, refill_rate),
        }
    }

    pub fn update_weight(&mut self, new_weight: usize) {
        if new_weight > 0 {
            self.current_used_weight = new_weight;
            self.last_updated = Instant::now();
        }
    }

    pub fn prepare_request(
        &mut self,
        weight: usize,
    ) -> (Option<Duration>, Option<DynamicLimitReason>) {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_updated);

        if elapsed <= self.refill_rate && self.current_used_weight > 0 {
            self.prepare_with_header_data(weight)
        } else {
            self.prepare_with_fallback(weight)
        }
    }

    fn prepare_with_header_data(
        &self,
        weight: usize,
    ) -> (Option<Duration>, Option<DynamicLimitReason>) {
        let available = self.max_weight.saturating_sub(self.current_used_weight);

        if available >= weight {
            return (None, None);
        }

        let current_time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default();

        let period_seconds = self.refill_rate.as_secs();
        let seconds_in_period = current_time.as_secs() % period_seconds;
        let wait_time = Duration::from_secs(period_seconds - seconds_in_period)
            .saturating_add(Duration::from_millis(500));

        (Some(wait_time), Some(DynamicLimitReason::HeaderRate))
    }

    fn prepare_with_fallback(
        &mut self,
        weight: usize,
    ) -> (Option<Duration>, Option<DynamicLimitReason>) {
        match self.fallback_bucket.calculate_wait_time(weight) {
            None => (None, None),
            Some(wait_time) => (Some(wait_time), Some(DynamicLimitReason::FixedWindowRate)),
        }
    }
}

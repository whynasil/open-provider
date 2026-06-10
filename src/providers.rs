use std::sync::{Arc, Mutex};
use std::time::Instant;
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};

use crate::config::ProviderConfig;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderStatus {
    pub name: String,
    pub base_url: String,
    pub requires_key: bool,
    pub is_healthy: bool,
    pub last_check_at: Option<String>,
    pub response_time_ms: Option<u64>,
    pub model_count: Option<u64>,
    pub error_message: Option<String>,
}

/// Check a single provider and update the database
pub async fn check_provider(
    conn: &rusqlite::Connection,
    provider: &ProviderConfig,
) -> anyhow::Result<ProviderStatus> {
    let requires_key = provider.api_key_env.is_some();
    let start = Instant::now();

    let result = fetch_models(provider).await;
    let elapsed = start.elapsed().as_millis() as u64;
    let now = chrono::Utc::now().to_rfc3339();

    let (is_healthy, model_count, error_msg) = match result {
        Ok(count) => (true, Some(count as u64), None),
        Err(e) => (false, None, Some(e.to_string())),
    };

    // Upsert provider
    conn.execute(
        "INSERT INTO providers (name, base_url, models_endpoint, requires_key, last_check_at, is_healthy, response_time_ms, model_count, error_message)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
         ON CONFLICT(name) DO UPDATE SET
            last_check_at=excluded.last_check_at,
            is_healthy=excluded.is_healthy,
            response_time_ms=excluded.response_time_ms,
            model_count=excluded.model_count,
            error_message=excluded.error_message",
        params![
            provider.name,
            provider.base_url,
            provider.models_endpoint,
            requires_key as i32,
            now,
            is_healthy as i32,
            elapsed,
            model_count,
            error_msg,
        ],
    )?;

    // Insert check history
    conn.execute(
        "INSERT INTO checks (provider_name, checked_at, is_healthy, response_time_ms, model_count, error_message)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![
            provider.name,
            now,
            is_healthy as i32,
            elapsed,
            model_count,
            error_msg,
        ],
    )?;

    // If healthy, upsert models
    if let Ok(ref models) = fetch_models_detailed(provider).await {
        let pid = get_provider_id(conn, &provider.name)?;
        for m in models {
            conn.execute(
                "INSERT OR REPLACE INTO models (provider_id, model_id, created_at, owned_by, raw_data) VALUES (?1, ?2, ?3, ?4, ?5)",
                params![pid, m.id, m.created, m.owned_by, m.raw],
            )?;
        }
    }

    Ok(ProviderStatus {
        name: provider.name.clone(),
        base_url: provider.base_url.clone(),
        requires_key,
        is_healthy,
        last_check_at: Some(now),
        response_time_ms: Some(elapsed),
        model_count,
        error_message: error_msg,
    })
}

fn get_provider_id(conn: &rusqlite::Connection, name: &str) -> anyhow::Result<i64> {
    let id: i64 = conn.query_row(
        "SELECT id FROM providers WHERE name = ?1",
        params![name],
        |row| row.get(0),
    )?;
    Ok(id)
}

#[derive(Debug, Deserialize)]
struct OpenAIModel {
    id: String,
    created: Option<i64>,
    owned_by: Option<String>,
    #[serde(skip)]
    raw: Option<String>,
}

/// Public model ID struct for routes
#[derive(Debug, Clone)]
pub struct ModelId {
    pub id: String,
    pub created: Option<i64>,
    pub owned_by: Option<String>,
    pub raw_data: Option<String>,
}

pub fn format_unix(ts: i64) -> String {
    if let Some(dt) = chrono::DateTime::from_timestamp(ts, 0) {
        dt.format("%Y-%m-%d").to_string()
    } else {
        ts.to_string()
    }
}

/// Fetch model count only (fast check)
async fn fetch_models(provider: &ProviderConfig) -> anyhow::Result<usize> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()?;

    let mut url = format!("{}{}", provider.base_url, provider.models_endpoint);

    // Handle API key in URL (Google style)
    if let Some(env_var) = &provider.api_key_env {
        if url.contains("{API_KEY}") {
            let key = std::env::var(env_var).ok();
            url = url.replace("{API_KEY}", &key.unwrap_or_default());
        }
    }

    let mut req = client.get(&url);

    // Add custom headers
    for (k, v) in &provider.headers {
        req = req.header(k, v);
    }

    // Add API key auth header
    if let Some(env_var) = &provider.api_key_env {
        if !url.contains("{API_KEY}") {
            // Try env var first, then fallback to token file
            let key = std::env::var(env_var).ok().or_else(|| {
                let token_file = format!("/home/test/{}_token.txt", env_var.to_lowercase());
                std::fs::read_to_string(&token_file).ok().map(|s| s.trim().to_string())
            });
            if let Some(k) = key {
                req = req.header("Authorization", format!("Bearer {k}"));
            }
        }
    }

    // Special handling for Anthropic
    if provider.name == "Anthropic" || provider.name == "Google" {
        if let Some(env_var) = &provider.api_key_env {
            if let Ok(key) = std::env::var(env_var) {
                if provider.name == "Anthropic" {
                    req = req.header("x-api-key", &key);
                }
            }
        }
    }

    let resp = req.send().await?;
    let status = resp.status();
    let text = resp.text().await?;

    if !status.is_success() {
        anyhow::bail!("HTTP {status}: {text}");
    }

    // Parse JSON and count models
    let json: serde_json::Value = serde_json::from_str(&text)?;

    // Try common response shapes
    if let Some(arr) = json.get("data").and_then(|d| d.as_array()) {
        Ok(arr.len())
    } else if let Some(arr) = json.get("models").and_then(|d| d.as_array()) {
        Ok(arr.len())
    } else if let Some(arr) = json.as_array() {
        Ok(arr.len())
    } else {
        Ok(0)
    }
}

/// Fetch detailed model list
async fn fetch_models_detailed(provider: &ProviderConfig) -> anyhow::Result<Vec<OpenAIModel>> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()?;

    let mut url = format!("{}{}", provider.base_url, provider.models_endpoint);

    if let Some(env_var) = &provider.api_key_env {
        if url.contains("{API_KEY}") {
            let key = std::env::var(env_var).ok();
            url = url.replace("{API_KEY}", &key.unwrap_or_default());
        }
    }

    let mut req = client.get(&url);

    for (k, v) in &provider.headers {
        req = req.header(k, v);
    }

    if let Some(env_var) = &provider.api_key_env {
        if !url.contains("{API_KEY}") {
            // Try env var first, then fallback to token file
            let key = std::env::var(env_var).ok().or_else(|| {
                let token_file = format!("/home/test/{}_token.txt", env_var.to_lowercase());
                std::fs::read_to_string(&token_file).ok().map(|s| s.trim().to_string())
            });
            if let Some(k) = key {
                req = req.header("Authorization", format!("Bearer {k}"));
            }
        }
    }

    let resp = req.send().await?;
    let status = resp.status();
    let text = resp.text().await?;

    if !status.is_success() {
        anyhow::bail!("HTTP {status}: {text}");
    }

    let json: serde_json::Value = serde_json::from_str(&text)?;

    let models: Vec<OpenAIModel> = if let Some(arr) = json.get("data").and_then(|d| d.as_array()) {
        arr.iter()
            .filter_map(|m| {
                let id = m.get("id")?.as_str()?.to_string();
                let created = m.get("created").and_then(|v| v.as_i64());
                let owned_by = m.get("owned_by").and_then(|v| v.as_str()).map(String::from);
                let raw = serde_json::to_string(m).ok();
                Some(OpenAIModel {
                    id,
                    created,
                    owned_by,
                    raw,
                })
            })
            .collect()
    } else if let Some(arr) = json.get("models").and_then(|d| d.as_array()) {
        arr.iter()
            .filter_map(|m| {
                let id = m.get("name")
                    .or_else(|| m.get("id"))
                    .and_then(|v| v.as_str())?
                    .to_string();
                let raw = serde_json::to_string(m).ok();
                Some(OpenAIModel {
                    id,
                    created: None,
                    owned_by: None,
                    raw,
                })
            })
            .collect()
    } else {
        vec![]
    };

    Ok(models)
}

/// Fetch count + model list (for routes — no lock) — single HTTP request
pub async fn fetch_models_only(provider: &ProviderConfig) -> anyhow::Result<(usize, Vec<ModelId>)> {
    let detailed = fetch_models_detailed(provider).await?;
    let count = detailed.len();
    let models = detailed
        .into_iter()
        .map(|m| ModelId {
            id: m.id,
            created: m.created,
            owned_by: m.owned_by,
            raw_data: m.raw,
        })
        .collect();
    Ok((count, models))
}

/// Fetch and record — split: HTTP outside lock, DB write inside lock
pub async fn fetch_and_record(
    db: Arc<Mutex<Connection>>,
    provider: &ProviderConfig,
) -> anyhow::Result<ProviderStatus> {
    let requires_key = provider.api_key_env.is_some();
    let start = Instant::now();
    let result = fetch_models(provider).await;
    let elapsed = start.elapsed().as_millis() as u64;
    let now = chrono::Utc::now().to_rfc3339();

    let (is_healthy, model_count, error_msg) = match &result {
        Ok(count) => (true, Some(*count as u64), None),
        Err(e) => (false, None, Some(e.to_string())),
    };

    // Fetch details OUTSIDE lock
    let detailed = if is_healthy {
        fetch_models_detailed(provider).await.ok()
    } else {
        None
    };

    // Now lock and write
    let conn = db.lock().unwrap();
    conn.execute(
        "INSERT INTO providers (name, base_url, models_endpoint, requires_key, last_check_at, is_healthy, response_time_ms, model_count, error_message)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
         ON CONFLICT(name) DO UPDATE SET
            last_check_at=excluded.last_check_at,
            is_healthy=excluded.is_healthy,
            response_time_ms=excluded.response_time_ms,
            model_count=excluded.model_count,
            error_message=excluded.error_message",
        params![
            provider.name,
            provider.base_url,
            provider.models_endpoint,
            requires_key as i32,
            now,
            is_healthy as i32,
            elapsed,
            model_count,
            error_msg,
        ],
    )?;

    conn.execute(
        "INSERT INTO checks (provider_name, checked_at, is_healthy, response_time_ms, model_count, error_message)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![
            provider.name,
            now,
            is_healthy as i32,
            elapsed,
            model_count,
            error_msg,
        ],
    )?;

    if let Some(models) = &detailed {
        let pid: i64 = conn.query_row(
            "SELECT id FROM providers WHERE name = ?1",
            params![provider.name],
            |row| row.get(0),
        )?;
        for m in models {
            conn.execute(
                "INSERT OR REPLACE INTO models (provider_id, model_id, created_at, owned_by, raw_data) VALUES (?1, ?2, ?3, ?4, ?5)",
                params![pid, m.id, m.created, m.owned_by, m.raw],
            )?;
        }
    }

    Ok(ProviderStatus {
        name: provider.name.clone(),
        base_url: provider.base_url.clone(),
        requires_key,
        is_healthy,
        last_check_at: Some(now),
        response_time_ms: Some(elapsed),
        model_count,
        error_message: error_msg,
    })
}

/// Get current status of all providers from DB
pub fn get_all_status(conn: &rusqlite::Connection) -> anyhow::Result<Vec<ProviderStatus>> {
    let mut stmt = conn.prepare(
        "SELECT name, base_url, requires_key, last_check_at, is_healthy, response_time_ms, model_count, error_message
         FROM providers ORDER BY name",
    )?;

    let rows = stmt.query_map([], |row| {
        Ok(ProviderStatus {
            name: row.get(0)?,
            base_url: row.get(1)?,
            requires_key: row.get::<_, i32>(2)? != 0,
            is_healthy: row.get::<_, i32>(4)? != 0,
            last_check_at: row.get(3)?,
            response_time_ms: row.get(5)?,
            model_count: row.get(6)?,
            error_message: row.get(7)?,
        })
    })?;

    Ok(rows.filter_map(|r| r.ok()).collect())
}

/// Get all models for a specific provider from DB
pub fn get_models_for_provider(
    conn: &rusqlite::Connection,
    provider_name: &str,
) -> anyhow::Result<Vec<ModelId>> {
    let mut stmt = conn.prepare(
        "SELECT m.model_id, m.created_at, m.owned_by, m.raw_data
         FROM models m
         JOIN providers p ON p.id = m.provider_id
         WHERE p.name = ?1
         ORDER BY m.model_id",
    )?;

    let rows = stmt.query_map(params![provider_name], |row| {
        let created_val: Option<i64> = row.get::<_, Option<i64>>(1)
            .ok()
            .flatten()
            .or_else(|| {
                row.get::<_, String>(1)
                    .ok()
                    .and_then(|s: String| s.parse().ok())
            });
        let owned_by_val: Option<String> = row.get::<_, Option<String>>(2).ok().flatten();
        let raw_data_val: Option<String> = row.get::<_, Option<String>>(3).ok().flatten();
        Ok(ModelId {
            id: row.get(0)?,
            created: created_val,
            owned_by: owned_by_val,
            raw_data: raw_data_val,
        })
    })?;

    Ok(rows.filter_map(|r| r.ok()).collect())
}
pub fn get_recent_checks(conn: &rusqlite::Connection, limit: usize) -> anyhow::Result<Vec<serde_json::Value>> {
    let mut stmt = conn.prepare(
        "SELECT provider_name, checked_at, is_healthy, response_time_ms, model_count, error_message
         FROM checks ORDER BY id DESC LIMIT ?1",
    )?;

    let rows = stmt.query_map(params![limit as i64], |row| {
        Ok(serde_json::json!({
            "provider_name": row.get::<_, String>(0)?,
            "checked_at": row.get::<_, String>(1)?,
            "is_healthy": row.get::<_, i32>(2)? != 0,
            "response_time_ms": row.get::<_, Option<i64>>(3)?,
            "model_count": row.get::<_, Option<i64>>(4)?,
            "error_message": row.get::<_, Option<String>>(5)?,
        }))
    })?;

    Ok(rows.filter_map(|r| r.ok()).collect())
}

use std::sync::{Arc, Mutex};
use axum::{
    extract::{Path, State},
    response::Html,
    routing::get,
    Json, Router,
};
use rusqlite::Connection;

use crate::config::{Config, FreeRules};
use crate::providers::{self, ProviderStatus};

type Db = Arc<Mutex<Connection>>;
type AppState = Arc<(Db, Config)>;

pub fn app(db: Db, cfg: Config) -> Router {
    let state: AppState = Arc::new((db, cfg));
    Router::new()
        .route("/", get(dashboard))
        .route("/api/status", get(api_status))
        .route("/api/checks", get(api_checks))
        .route("/api/check-now", get(api_check_now))
        .route("/checks", get(all_checks))
        .route("/provider/{slug}", get(provider_models))
        .with_state(state)
}

fn make_slug(name: &str) -> String {
    name.to_lowercase().replace(' ', "-")
}

fn format_cell_value(key: &str, val: &str) -> String {
    // Keep tokens/context as raw numbers
    let _is_size_key = key.ends_with("length") || key.ends_with("tokens") || key.ends_with("context") || key == "max_completion_tokens";
    val.to_string()
}

async fn dashboard(State(state): State<AppState>) -> Html<String> {
    let (db, cfg) = &*state;
    let conn = db.lock().unwrap();
    let all_status = providers::get_all_status(&conn).unwrap_or_default();
    let disabled_names: std::collections::HashSet<String> = cfg
        .providers.iter().filter(|p| p.disabled).map(|p| p.name.clone()).collect();
    let statuses: Vec<ProviderStatus> = all_status
        .into_iter().filter(|s| !disabled_names.contains(&s.name)).collect();
    let checks = providers::get_recent_checks(&conn, 5).unwrap_or_default();
    drop(conn);
    let n = statuses.len();
    let card_html: String = statuses.iter().map(|s| {
        let slug = make_slug(&s.name);
        let health_class = if s.is_healthy { "healthy" } else { "down" };
        let mc = s.model_count.map(|n| format!("{}", n)).unwrap_or_else(|| "—".into());
        let rt = s.response_time_ms.map(|n| format!("{}ms", n)).unwrap_or_else(|| "—".into());
        let lc = s.last_check_at.as_deref().unwrap_or("—");
        let key_badge = if s.requires_key {
            r#"<span class="card-badge"><svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="7.5" cy="15.5" r="5.5"/><path d="m21 2-9.6 9.6"/><path d="m15.5 7.5 3 3L22 7l-3-3"/></svg>API key</span>"#
        } else { "" };
        let err_html = s.error_message.as_deref().unwrap_or("");
        let err_div = if !err_html.is_empty() { format!(r#"<div class="card-error" title="{}">{}</div>"#, err_html, err_html) } else { String::new() };
        format!(r#"<a href="/provider/{slug}" class="card"><div class="card-top"><span class="card-name">{name}</span><span class="card-status {health_class}"></span></div><div class="card-details"><span>{mc} models · {rt}</span><span>Checked: {lc}</span></div>{key_badge}{err_div}</a>"#, name = s.name)
    }).collect();
    let check_rows: String = checks.iter().map(|c| {
        let name = c.get("provider_name").and_then(|v| v.as_str()).unwrap_or("-");
        let time = c.get("checked_at").and_then(|v| v.as_str()).unwrap_or("-");
        let healthy = c.get("is_healthy").and_then(|v| v.as_bool()).unwrap_or(false);
        let ms = c.get("response_time_ms").and_then(|v| v.as_i64()).map(|n| format!("{}ms", n)).unwrap_or_else(|| "—".into());
        let mc = c.get("model_count").and_then(|v| v.as_i64()).map(|n| format!("{}", n)).unwrap_or_else(|| "—".into());
        let err = c.get("error_message").and_then(|v| v.as_str()).unwrap_or("");
        let row_class = if healthy { "check-ok" } else { "check-fail" };
        let status_html = if healthy {
            r#"<span class="status-icon"><svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="hsl(142, 76%, 36%)" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round"><path d="M22 11.08V12a10 10 0 1 1-5.93-9.14"/><path d="m9 11 3 3L22 4"/></svg>Pass</span>"#
        } else {
            r#"<span class="status-icon"><svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="hsl(0, 62.8%, 50%)" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="10"/><path d="m15 9-6 6"/><path d="m9 9 6 6"/></svg>Fail</span>"#
        };
        format!(r#"<tr class="{row_class}"><td>{name}</td><td>{time}</td><td>{status_html}</td><td>{ms}</td><td>{mc}</td><td class="error-cell" title="{err}">{err}</td></tr>"#)
    }).collect();
    let html = format!(r##"<!DOCTYPE html>
<html lang="en">
<head><meta charset="UTF-8"><meta name="viewport" content="width=device-width,initial-scale=1.0">
<title>Open Provider — Model Catalog Monitor</title>
<link rel="preconnect" href="https://fonts.googleapis.com">
<link href="https://fonts.googleapis.com/css2?family=Inter:wght@400;500;600;700&display=swap" rel="stylesheet">
<style>
:root{{--background:240 10% 3.9%;--foreground:0 0% 98%;--card:240 10% 5.9%;--primary:217 91% 60%;--secondary:240 3.7% 15.9%;--secondary-foreground:0 0% 98%;--muted:240 3.7% 15.9%;--muted-foreground:240 5% 64.9%;--accent:240 3.7% 15.9%;--accent-foreground:0 0% 98%;--destructive:0 62.8% 30.6%;--success:142 76% 36%;--border:240 3.7% 15.9%;--ring:217 91% 60%;--radius:0.5rem}}
*{{margin:0;padding:0;box-sizing:border-box}}
body{{font-family:'Inter',-apple-system,BlinkMacSystemFont,sans-serif;background:hsl(var(--background));color:hsl(var(--foreground));padding:32px 24px;line-height:1.5;-webkit-font-smoothing:antialiased}}
h1{{font-size:28px;font-weight:700;letter-spacing:-0.02em;margin-bottom:4px}}
.subtitle{{color:hsl(var(--muted-foreground));font-size:14px;margin-bottom:32px;display:flex;align-items:center;gap:12px}}
.btn-icon{{display:inline-flex;align-items:center;gap:6px;font-size:13px;color:hsl(var(--muted-foreground));cursor:pointer;border:none;background:none;font-family:inherit;padding:4px 0;transition:color .15s}}
.btn-icon:hover{{color:hsl(var(--primary))}}.btn-icon svg{{width:14px;height:14px}}
.section-header{{display:flex;align-items:center;justify-content:space-between;margin-bottom:16px}}
.section-title{{font-size:16px;font-weight:600;letter-spacing:-0.01em}}
.grid{{display:grid;grid-template-columns:repeat(auto-fill,minmax(280px,1fr));gap:12px;margin-bottom:40px}}
.card{{background:hsl(var(--card));border:1px solid hsl(var(--border));border-radius:var(--radius);padding:16px 20px;text-decoration:none;color:inherit;display:block;transition:border-color .15s,background .15s}}
.card:hover{{border-color:hsl(var(--ring));background:hsl(var(--accent))}}
.card-top{{display:flex;align-items:center;justify-content:space-between;margin-bottom:12px}}
.card-name{{font-weight:600;font-size:15px}}
.card-status{{width:8px;height:8px;border-radius:50%;flex-shrink:0}}
.card-status.healthy{{background:hsl(var(--success));box-shadow:0 0 0 3px hsla(var(--success)/.2)}}
.card-status.down{{background:hsl(var(--destructive));box-shadow:0 0 0 3px hsla(var(--destructive)/.2)}}
.card-details{{font-size:12px;color:hsl(var(--muted-foreground));display:flex;flex-direction:column;gap:4px}}
.card-badge{{display:inline-flex;align-items:center;gap:5px;border-radius:calc(var(--radius) - 2px);padding:1px 8px;font-size:11px;font-weight:500;border:1px solid hsl(var(--border));background:hsl(var(--secondary));color:hsl(var(--secondary-foreground));width:fit-content;margin-top:4px}}
.card-badge svg{{width:11px;height:11px}}
.card-error{{margin-top:8px;font-size:11px;color:hsl(var(--destructive));background:hsla(var(--destructive)/.1);padding:6px 8px;border-radius:calc(var(--radius) - 2px);word-break:break-all;max-height:56px;overflow:hidden;line-height:1.3}}
.table-wrap{{background:hsl(var(--card));border:1px solid hsl(var(--border));border-radius:var(--radius);overflow:hidden}}
.table-wrap-inner{{overflow-x:auto}}
table{{width:100%;border-collapse:collapse;font-size:13px}}
th,td{{padding:10px 14px;text-align:left;border-bottom:1px solid hsl(var(--border));white-space:nowrap}}
th{{color:hsl(var(--muted-foreground));font-weight:500;font-size:11px;text-transform:uppercase;letter-spacing:.05em}}
tr:last-child td{{border-bottom:none}}
.check-ok td:first-child{{color:hsl(var(--success));font-weight:500}}
.check-fail td:first-child{{color:hsl(var(--destructive));font-weight:500}}
.status-icon{{display:inline-flex;align-items:center;gap:5px}}
.status-icon svg{{width:14px;height:14px;flex-shrink:0}}
.error-cell{{max-width:220px;overflow:hidden;text-overflow:ellipsis;color:hsl(var(--destructive));font-size:11px}}
@media(max-width:640px){{body{{padding:20px 12px}}.grid{{grid-template-columns:1fr}}table{{font-size:11px}}th,td{{padding:8px 10px}}}}
</style></head>
<body>
<h1>Open Provider</h1>
<div class="subtitle">Monitoring {n} providers<button class="btn-icon" onclick="location.reload()"><svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M3 12a9 9 0 0 1 9-9 9.75 9.75 0 0 1 6.74 2.74L21 8"/><path d="M21 3v5h-5"/><path d="M21 12a9 9 0 0 1-9 9 9.75 9.75 0 0 1-6.74-2.74L3 16"/><path d="M3 21v-5h5"/></svg>Refresh</button></div>
<div class="section-header"><span class="section-title">Providers</span></div>
<div class="grid">{card_html}</div>
<div class="section-header"><span class="section-title">Recent Checks</span><a href="/checks" class="btn-icon" style="text-decoration:none">View all →</a></div>
<div class="table-wrap"><div class="table-wrap-inner"><table><thead><tr><th>Provider</th><th>Time</th><th>Status</th><th>Latency</th><th>Models</th><th>Error</th></tr></thead>
<tbody>{check_rows}</tbody></table></div></div>
</body></html>"##);
    Html(html)
}

async fn api_status(State(state): State<AppState>) -> Json<Vec<ProviderStatus>> {
    let (db, cfg) = &*state;
    let disabled_names: std::collections::HashSet<String> = cfg.providers.iter().filter(|p| p.disabled).map(|p| p.name.clone()).collect();
    let conn = db.lock().unwrap();
    let result = providers::get_all_status(&conn).unwrap_or_default();
    let filtered: Vec<_> = result.into_iter().filter(|s| !disabled_names.contains(&s.name)).collect();
    Json(filtered)
}

async fn api_checks(State(state): State<AppState>) -> Json<Vec<serde_json::Value>> {
    let (db, _cfg) = &*state;
    let conn = db.lock().unwrap();
    Json(providers::get_recent_checks(&conn, 200).unwrap_or_default())
}

async fn all_checks(State(state): State<AppState>) -> Html<String> {
    let (db, _cfg) = &*state;
    let conn = db.lock().unwrap();
    let checks = providers::get_recent_checks(&conn, 200).unwrap_or_default();
    drop(conn);
    let rows: String = checks.iter().map(|c| {
        let name = c.get("provider_name").and_then(|v| v.as_str()).unwrap_or("-");
        let time = c.get("checked_at").and_then(|v| v.as_str()).unwrap_or("-");
        let healthy = c.get("is_healthy").and_then(|v| v.as_bool()).unwrap_or(false);
        let ms = c.get("response_time_ms").and_then(|v| v.as_i64()).map(|n| format!("{}ms", n)).unwrap_or_else(|| "—".into());
        let mc = c.get("model_count").and_then(|v| v.as_i64()).map(|n| format!("{}", n)).unwrap_or_else(|| "—".into());
        let err = c.get("error_message").and_then(|v| v.as_str()).unwrap_or("");
        let row_class = if healthy { "check-ok" } else { "check-fail" };
        let status_html = if healthy {
            r##"<span class="status-icon"><svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="hsl(142, 76%, 36%)" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round"><path d="M22 11.08V12a10 10 0 1 1-5.93-9.14"/><path d="m9 11 3 3L22 4"/></svg>Pass</span>"##
        } else {
            r##"<span class="status-icon"><svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="hsl(0, 62.8%, 50%)" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="10"/><path d="m15 9-6 6"/><path d="m9 9 6 6"/></svg>Fail</span>"##
        };
        format!(r##"<tr class="{row_class}"><td>{name}</td><td>{time}</td><td>{status_html}</td><td>{ms}</td><td>{mc}</td><td class="error-cell" title="{err}">{err}</td></tr>"##)
    }).collect();
    let html = format!(r###"<!DOCTYPE html>
<html lang="en">
<head><meta charset="UTF-8"><meta name="viewport" content="width=device-width,initial-scale=1.0">
<title>All Checks — Open Provider</title>
<link rel="preconnect" href="https://fonts.googleapis.com">
<link href="https://fonts.googleapis.com/css2?family=Inter:wght@400;500;600;700&display=swap" rel="stylesheet">
<style>
:root{{--background:240 10% 3.9%;--foreground:0 0% 98%;--card:240 10% 5.9%;--secondary:240 3.7% 15.9%;--secondary-foreground:0 0% 98%;--muted:240 3.7% 15.9%;--muted-foreground:240 5% 64.9%;--accent:240 3.7% 15.9%;--accent-foreground:0 0% 98%;--destructive:0 62.8% 30.6%;--success:142 76% 36%;--border:240 3.7% 15.9%;--ring:217 91% 60%;--primary:217 91% 60%;--primary-foreground:0 0% 98%;--radius:0.5rem}}
*{{margin:0;padding:0;box-sizing:border-box}}
body{{font-family:'Inter',-apple-system,BlinkMacSystemFont,sans-serif;background:hsl(var(--background));color:hsl(var(--foreground));margin:0 auto;max-width:1200px;padding:32px 24px;line-height:1.5;-webkit-font-smoothing:antialiased}}
h1{{font-size:24px;font-weight:700;letter-spacing:-0.02em;margin-bottom:20px}}
.back{{color:hsl(var(--muted-foreground));text-decoration:none;font-size:13px;display:inline-flex;align-items:center;gap:4px;margin-bottom:20px;transition:color .15s;font-weight:500}}
.back:hover{{color:hsl(var(--primary))}}
.table-wrap{{background:hsl(var(--card));border:1px solid hsl(var(--border));border-radius:var(--radius);overflow:hidden}}
.table-wrap-inner{{overflow-x:auto}}
table{{width:100%;border-collapse:collapse;font-size:13px}}
th,td{{padding:10px 14px;text-align:left;border-bottom:1px solid hsl(var(--border));white-space:nowrap}}
th{{color:hsl(var(--muted-foreground));font-weight:500;font-size:11px;text-transform:uppercase;letter-spacing:.05em;background:hsl(var(--card))}}
tr:last-child td{{border-bottom:none}}
.check-ok td:first-child{{color:hsl(var(--success));font-weight:500}}
.check-fail td:first-child{{color:hsl(var(--destructive));font-weight:500}}
.status-icon{{display:inline-flex;align-items:center;gap:5px}}
.status-icon svg{{width:14px;height:14px;flex-shrink:0}}
.error-cell{{max-width:220px;overflow:hidden;text-overflow:ellipsis;color:hsl(var(--destructive));font-size:11px}}
</style></head>
<body>
<a class="back" href="/"><svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="m15 18-6-6 6-6"/></svg>Back to dashboard</a>
<h1>All Checks</h1>
<div class="table-wrap"><div class="table-wrap-inner">
<table><thead><tr><th>Provider</th><th>Time</th><th>Status</th><th>Latency</th><th>Models</th><th>Error</th></tr></thead>
<tbody>{rows}</tbody></table></div></div>
</body></html>"###);
    Html(html)
}

async fn provider_models(Path(slug): Path<String>, State(state): State<AppState>) -> Html<String> {
    let (db, cfg) = &*state;
    let conn = db.lock().unwrap();
    let all_status = providers::get_all_status(&conn).unwrap_or_default();
    let provider_name = all_status.iter().find(|s| make_slug(&s.name) == slug).map(|s| s.name.clone()).unwrap_or_else(|| slug.clone());
    let models = providers::get_models_for_provider(&conn, &provider_name).unwrap_or_default();
    let provider = all_status.into_iter().find(|s| s.name == provider_name);
    let free_rules = cfg.providers.iter().find(|p| p.name == provider_name).map(|p| p.free_rules.clone()).unwrap_or_default();
    drop(conn);
    let mut extra_keys_set = std::collections::BTreeSet::new();
    let mut parsed_models: Vec<(String, serde_json::Value)> = Vec::new();
    for m in &models {
        let raw: serde_json::Value = m.raw_data.as_deref().and_then(|s| serde_json::from_str(s).ok()).unwrap_or(serde_json::Value::Null);
        if let Some(obj) = raw.as_object() {
            for k in obj.keys() {
                if k != "id" && k != "object" { extra_keys_set.insert(k.clone()); }
            }
        }
        parsed_models.push((m.id.clone(), raw));
    }
    fn is_free_model(id: &str, raw: &serde_json::Value, rules: &FreeRules) -> bool {
        if rules.id_suffix_colon_free && id.ends_with(":free") { return true; }
        if rules.id_contains_dash_free && id.contains("-free") { return true; }
        if rules.id_contains_free && id.to_lowercase().contains("free") { return true; }
        if rules.field_is_free && raw.get("isFree").and_then(|v| v.as_bool()).unwrap_or(false) { return true; }
        if rules.zero_pricing {
            if let Some(pricing) = raw.get("pricing").and_then(|v| v.as_object()) {
                if !pricing.is_empty() && pricing.values().all(|v| v.as_str().and_then(|s| s.parse::<f64>().ok()).unwrap_or(1.0) <= 0.0) { return true; }
            }
        }
        false
    }
    parsed_models.sort_by(|(id_a, raw_a), (id_b, raw_b)| {
        let a_free = is_free_model(id_a, raw_a, &free_rules);
        let b_free = is_free_model(id_b, raw_b, &free_rules);
        b_free.cmp(&a_free)
    });
    let free_count = parsed_models.iter().filter(|(id, raw)| is_free_model(id, raw, &free_rules)).count();
    let model_count = parsed_models.len();
    let model_rows: Vec<String> = parsed_models.iter().map(|(id, raw)| {
        let escaped_id = id.replace('"', "&quot;").replace('\'', "&#39;");
        let is_free = is_free_model(id, raw, &free_rules);
        let free_badge = if is_free { r#"<span class="free-badge">FREE</span>"# } else { "" };
        let tab_class = if is_free { "tab-free" } else { "tab-paid" };
        let mut cells = vec![format!(r#"<td><code class="mid" onclick="copyId('{}',event)" title="Click to copy">{}</code>{}</td>"#, escaped_id, escaped_id, free_badge)];
        if let Some(obj) = raw.as_object() {
            for k in obj.keys() {
                if k != "id" {
                    let s = match obj.get(k).unwrap() {
                        serde_json::Value::String(s) => s.clone(),
                        serde_json::Value::Number(n) => n.to_string(),
                        serde_json::Value::Bool(b) => b.to_string(),
                        other => format!("{}", other).replace('"', "&quot;"),
                    };
                    let display = format_cell_value(k, &s);
                    cells.push(format!(r#"<td class="col-muted">{}</td>"#, display));
                }
            }
        }
        format!(r#"<tr class="{}">{}</tr>"#, tab_class, cells.join(""))
    }).collect();
    let header_keys: Vec<String> = extra_keys_set.into_iter().collect();
    let mut headers = "<th>Model ID</th>".to_string();
    for k in &header_keys { headers.push_str(&format!("<th>{}</th>", k.replace('_', " "))); }
    let response_time = provider.as_ref().map(|p| format!("{}ms", p.response_time_ms.unwrap_or(0))).unwrap_or_else(|| "—".into());
    let is_healthy = provider.as_ref().map(|p| p.is_healthy).unwrap_or(false);
    let health_class = if is_healthy { "healthy" } else { "down" };
    let pc_ref = cfg.providers.iter().find(|pc| pc.name == provider_name);
    let provider_url = pc_ref.map(|pc| pc.base_url.clone()).unwrap_or_else(|| "#".into());
    let api_full_url = pc_ref.map(|pc| {
        format!("{}{}", pc.base_url.trim_end_matches('/'), pc.models_endpoint)
    }).unwrap_or_default();
    let api_url_html = if !api_full_url.is_empty() {
        let escaped = api_full_url.replace('"', "&quot;");
        format!(r#"<button class="copy-btn" onclick="copyId('{}',event)" title="Kopyala">📋</button><code class="api-url-code" onclick="copyId('{}',event)" title="Kopyalamak için tıkla">{}</code>"#, escaped, escaped, escaped)
    } else { String::new() };
    let needs_api_key = pc_ref.and_then(|pc| pc.api_key_env.as_ref()).is_some();
    let env_var = String::from("API_KEY");
    let api_key_badge = if needs_api_key {
        let env_name = pc_ref.and_then(|pc| pc.api_key_env.as_ref()).unwrap_or(&env_var);
        format!(r#"<span class="badge"><svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" width="10" height="10"><path d="M21 2l-2 2m-7.61 7.61a5.5 5.5 0 1 1-7.778 7.778 5.5 5.5 0 0 1 7.777-7.777zm0 0L15.5 7.5m0 0l3 3L22 7l-3-3m-3.5 3.5L19 4"/></svg>{} Required</span>"#, env_name)
    } else { String::new() };
    let html = format!(r###"<!DOCTYPE html>
<html lang="en">
<head><meta charset="UTF-8"><meta name="viewport" content="width=device-width,initial-scale=1.0">
<title>{name_f} — Models — Open Provider</title>
<link rel="preconnect" href="https://fonts.googleapis.com">
<link href="https://fonts.googleapis.com/css2?family=Inter:wght@400;500;600;700&display=swap" rel="stylesheet">
<style>
:root{{--background:240 10% 3.9%;--foreground:0 0% 98%;--card:240 10% 5.9%;--secondary:240 3.7% 15.9%;--secondary-foreground:0 0% 98%;--muted:240 3.7% 15.9%;--muted-foreground:240 5% 64.9%;--accent:240 3.7% 15.9%;--accent-foreground:0 0% 98%;--destructive:0 62.8% 30.6%;--success:142 76% 36%;--border:240 3.7% 15.9%;--ring:217 91% 60%;--primary:217 91% 60%;--primary-foreground:0 0% 98%;--radius:0.5rem}}
*{{margin:0;padding:0;box-sizing:border-box}}
body{{font-family:'Inter',-apple-system,BlinkMacSystemFont,sans-serif;background:hsl(var(--background));color:hsl(var(--foreground));padding:32px 24px;line-height:1.5;-webkit-font-smoothing:antialiased}}
h1{{font-size:28px;font-weight:700;letter-spacing:-0.02em;margin-bottom:4px}}
.back{{color:hsl(var(--muted-foreground));text-decoration:none;font-size:13px;display:inline-flex;align-items:center;gap:4px;margin-bottom:20px;transition:color .15s;font-weight:500}}
.back:hover{{color:hsl(var(--primary))}}
.meta{{font-size:14px;color:hsl(var(--muted-foreground));margin-bottom:16px;display:flex;align-items:center;gap:16px;flex-wrap:wrap}}
.status{{width:8px;height:8px;border-radius:50%;flex-shrink:0}}
.status.healthy{{background:hsl(var(--success));box-shadow:0 0 0 3px hsla(var(--success)/.2)}}
.status.down{{background:hsl(var(--destructive));box-shadow:0 0 0 3px hsla(var(--destructive)/.2)}}
.api-endpoint{{margin:4px 0 8px 0}}
.endpoint-row{{display:flex;align-items:center;gap:6px;font-size:13px;flex-wrap:wrap}}
.api-url-code{{background:hsl(var(--card));border:1px solid hsl(var(--border));border-radius:4px;padding:4px 8px;font-family:'SF Mono','Fira Code','Fira Mono',monospace;font-size:12px;color:hsl(var(--primary));cursor:pointer;user-select:all}}
.api-url-code:hover{{border-color:hsl(var(--ring))}}
.copy-btn{{background:hsl(var(--card));border:1px solid hsl(var(--border));border-radius:4px;padding:3px 6px;cursor:pointer;font-size:14px;display:inline-flex;align-items:center;transition:all .15s;flex-shrink:0}}
.copy-btn:hover{{color:hsl(var(--foreground));border-color:hsl(var(--ring));background:hsl(var(--accent))}}
.table-wrap{{background:hsl(var(--card));border:1px solid hsl(var(--border));border-radius:var(--radius);overflow:hidden}}
.table-inner{{overflow-x:auto}}
table{{width:100%;border-collapse:collapse;font-size:13px;white-space:nowrap}}
th,td{{padding:10px 14px;text-align:left;border-bottom:1px solid hsl(var(--border))}}
th{{color:hsl(var(--muted-foreground));font-weight:500;font-size:11px;text-transform:uppercase;letter-spacing:.05em;position:sticky;top:0;background:hsl(var(--card));z-index:1}}
tr:last-child td{{border-bottom:none}}
.col-muted{{color:hsl(var(--muted-foreground))}}
.mid{{color:hsl(var(--primary));cursor:pointer;user-select:all;font-size:12px;font-family:'SF Mono','Fira Code','Fira Mono',monospace;font-weight:500;padding:1px 4px;border-radius:3px;transition:background .1s}}
.mid:hover{{background:hsla(var(--primary)/.15)}}
.mid:active{{background:hsla(var(--primary)/.25)}}
.toast{{position:fixed;top:16px;left:50%;transform:translateX(-50%);background:hsl(var(--success));color:#fff;padding:8px 20px;border-radius:var(--radius);font-size:13px;font-weight:500;z-index:999;opacity:0;transition:opacity .2s;pointer-events:none}}
.toast.show{{opacity:1}}
.badge{{display:inline-flex;align-items:center;gap:5px;border-radius:calc(var(--radius) - 2px);padding:1px 8px;font-size:11px;font-weight:500;line-height:1.6;border:1px solid hsl(var(--border));background:hsl(var(--secondary));color:hsl(var(--secondary-foreground))}}
.provider-link{{display:inline-flex;align-items:center;gap:5px;color:hsl(var(--muted-foreground));text-decoration:none;font-size:13px;transition:color .15s;font-weight:500}}
.provider-link:hover{{color:hsl(var(--primary))}}
.provider-link svg{{flex-shrink:0}}
.free-badge{{display:inline-block;background:hsl(142,76%,36%);color:#fff;font-size:10px;font-weight:700;padding:1px 5px;border-radius:3px;margin-left:4px;vertical-align:middle;text-transform:uppercase;letter-spacing:.05em}}
.tabs{{display:flex;gap:4px;margin-bottom:16px;border-bottom:1px solid hsl(var(--border));padding-bottom:0}}
.tab-btn{{background:none;border:none;color:hsl(var(--muted-foreground));font-family:inherit;font-size:13px;font-weight:500;padding:8px 16px;cursor:pointer;border-bottom:2px solid transparent;margin-bottom:-1px;transition:all .15s}}
.tab-btn:hover{{color:hsl(var(--foreground))}}
.tab-btn.active{{color:hsl(var(--foreground));border-bottom-color:hsl(var(--primary))}}
.tab-count{{font-size:11px;color:hsl(var(--muted-foreground));font-weight:400;margin-left:4px}}
</style>
<script>function copyId(id,e){{e.stopPropagation();navigator.clipboard.writeText(id).then(()=>{{let t=document.getElementById('toast');t.textContent='Copied: '+id;t.classList.add('show');setTimeout(()=>t.classList.remove('show'),2000)}})}}function switchTab(t){{document.querySelectorAll('.tab-btn').forEach(b=>b.classList.remove('active'));event.target.classList.add('active');let all=document.querySelectorAll('tr.tab-free,tr.tab-paid');if(t==='free'){{all.forEach(r=>{{r.style.display=r.classList.contains('tab-free')?'table-row':'none'}})}}else{{all.forEach(r=>r.style.display='table-row')}}}}</script>
</head>
<body>
<div id="toast" class="toast"></div>
<a class="back" href="/"><svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="m15 18-6-6 6-6"/></svg>Back to dashboard</a>
<h1>{name_f}</h1>
<div class="meta">
<span class="status {health_class}"></span>
<span>{count} models · {latency}</span>
<a class="provider-link" href="{provider_url_f}" target="_blank" rel="noopener"><svg xmlns="http://www.w3.org/2000/svg" width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M18 13v6a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V8a2 2 0 0 1 2-2h6"/><polyline points="15 3 21 3 21 9"/><line x1="10" y1="14" x2="21" y2="3"/></svg>{provider_url_f}</a>
{api_key_badge}
</div>
<div class="api-endpoint">
    <div class="endpoint-row">{api_url_html}</div>
</div>
<div class="tabs">
    <button class="tab-btn active" onclick="switchTab('all')">All<span class="tab-count">{count}</span></button>
    <button class="tab-btn" onclick="switchTab('free')">Free<span class="tab-count">{free_count}</span></button>
</div>
<div class="table-wrap"><div class="table-inner">
<table><thead><tr>{headers}</tr></thead>
<tbody>{rows}</tbody></table></div></div></body></html>"###,
        rows = model_rows.join(""), headers = headers, count = model_count,
        provider_url_f = provider_url, name_f = provider_name,
        health_class = health_class, latency = response_time, free_count = free_count,
        api_key_badge = api_key_badge, api_url_html = api_url_html,
    );
    Html(html)
}

async fn api_check_now(State(state): State<AppState>) -> Json<Vec<ProviderStatus>> {
    let (db, cfg) = &*state;
    let providers_cfg = cfg.providers.clone();

    // Step 1: Do ALL HTTP calls without holding the DB lock
    struct CheckResult {
        provider_name: String,
        provider_base_url: String,
        requires_key: bool,
        is_healthy: bool,
        elapsed_ms: u64,
        model_count: Option<usize>,
        error_msg: Option<String>,
        models: Vec<providers::ModelId>,
    }

    let mut check_results = Vec::new();
    for p in &providers_cfg {
        if p.disabled { continue; }
        let start = std::time::Instant::now();
        let models_result = providers::fetch_models_only(p).await;
        let elapsed = start.elapsed().as_millis() as u64;

        let (is_healthy, model_count, error_msg, models) = match models_result {
            Ok((count, m)) => (true, Some(count), None, m),
            Err(e) => (false, None, Some(e.to_string()), vec![]),
        };

        check_results.push(CheckResult {
            provider_name: p.name.clone(),
            provider_base_url: p.base_url.clone(),
            requires_key: p.api_key_env.is_some(),
            is_healthy,
            elapsed_ms: elapsed,
            model_count,
            error_msg,
            models,
        });
    }

    // Step 2: Now lock the DB and write all results
    {
        let conn = db.lock().unwrap();
        let now = chrono::Utc::now().to_rfc3339();

        for cr in &check_results {
            // Upsert provider record
            let _ = conn.execute(
                "INSERT INTO providers (name, base_url, models_endpoint, requires_key, last_check_at, is_healthy, response_time_ms, model_count, error_message)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
                 ON CONFLICT(name) DO UPDATE SET
                    last_check_at=excluded.last_check_at,
                    is_healthy=excluded.is_healthy,
                    response_time_ms=excluded.response_time_ms,
                    model_count=excluded.model_count,
                    error_message=excluded.error_message",
                rusqlite::params![
                    cr.provider_name,
                    cr.provider_base_url,
                    "",
                    cr.requires_key as i32,
                    now,
                    cr.is_healthy as i32,
                    cr.elapsed_ms,
                    cr.model_count.map(|c| c as i64),
                    cr.error_msg,
                ],
            );

            // Store models if provider is healthy
            if cr.is_healthy {
                if let Ok(pid) = conn.query_row(
                    "SELECT id FROM providers WHERE name = ?1",
                    rusqlite::params![cr.provider_name],
                    |row| row.get::<_, i64>(0),
                ) {
                    for m in &cr.models {
                        let _ = conn.execute(
                            "INSERT OR REPLACE INTO models (provider_id, model_id, created_at, owned_by, raw_data)
                             VALUES (?1, ?2, ?3, ?4, ?5)",
                            rusqlite::params![pid, m.id, m.created, m.owned_by, m.raw_data],
                        );
                    }
                }
            }

            // Add check record
            let _ = conn.execute(
                "INSERT INTO checks (provider_name, checked_at, is_healthy, response_time_ms, model_count, error_message)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                rusqlite::params![
                    cr.provider_name,
                    now,
                    cr.is_healthy as i32,
                    cr.elapsed_ms,
                    cr.model_count.map(|c| c as i64),
                    cr.error_msg,
                ],
            );
        }
    }

    // Step 3: Read back and filter only enabled providers
    let results = {
        let conn = db.lock().unwrap();
        let disabled_names: std::collections::HashSet<String> = cfg
            .providers
            .iter()
            .filter(|p| p.disabled)
            .map(|p| p.name.clone())
            .collect();
        providers::get_all_status(&conn)
            .unwrap_or_default()
            .into_iter()
            .filter(|s| !disabled_names.contains(&s.name))
            .collect::<Vec<_>>()
    };

    Json(results)
}

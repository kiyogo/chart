//! Spec 016: Standalone "Chat với Vico AI" backend (separate from Enterprise AI).
//! Supports PostgreSQL conversation persistence, optional OpenAI-compatible endpoint,
//! and an honest built-in local tool/query engine when no external LLM key is set.

use axum::{
    Json, Router,
    extract::{Path, State},
    http::HeaderMap,
    routing::{delete, get},
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sqlx::Row;
use uuid::Uuid;

use crate::{
    auth::AppState,
    error::{AppError, AppResult},
    privacy::auth_user,
};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/conversations", get(list_conversations).post(create_conversation))
        .route("/conversations/{id}", delete(delete_conversation))
        .route("/conversations/{id}/messages", get(list_messages).post(send_message))
        .route("/config", get(get_config).put(update_config))
}

#[derive(Serialize)]
pub struct VicoConversation {
    pub id: Uuid,
    pub title: String,
    pub last_message: Option<String>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Serialize)]
pub struct VicoMessage {
    pub id: Uuid,
    pub conversation_id: Uuid,
    pub role: String,
    pub content: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Deserialize)]
pub struct CreateConvReq {
    pub title: Option<String>,
}

#[derive(Deserialize)]
pub struct SendMsgReq {
    pub content: String,
}

#[derive(Deserialize)]
pub struct UpdateConfigReq {
    pub api_endpoint: Option<String>,
    pub api_key: Option<String>,
    pub model_name: Option<String>,
}

fn validate_endpoint(url: &str) -> bool {
    (url.starts_with("https://") || url.starts_with("http://localhost") || url.starts_with("http://127.0.0.1"))
        && url.len() <= 300
        && !url.chars().any(|c| c.is_whitespace() || c.is_control())
}

async fn list_conversations(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> AppResult<Json<Vec<VicoConversation>>> {
    let uid = auth_user(&state, &headers).await?;
    let mut rows = sqlx::query(
        r#"
        SELECT c.id, c.title, c.updated_at,
               (SELECT m.content FROM vico_messages m WHERE m.conversation_id = c.id ORDER BY m.created_at DESC LIMIT 1) AS last_message
        FROM vico_conversations c
        WHERE c.user_id = $1
        ORDER BY c.updated_at DESC
        LIMIT 50
        "#,
    )
    .bind(uid)
    .fetch_all(&state.db)
    .await?;

    if rows.is_empty() {
        let new_id = Uuid::new_v4();
        sqlx::query("INSERT INTO vico_conversations (id, user_id, title) VALUES ($1, $2, $3)")
            .bind(new_id)
            .bind(uid)
            .bind("New chat")
            .execute(&state.db)
            .await?;
        rows = sqlx::query(
            r#"
            SELECT c.id, c.title, c.updated_at, NULL::text AS last_message
            FROM vico_conversations c WHERE c.id = $1
            "#,
        )
        .bind(new_id)
        .fetch_all(&state.db)
        .await?;
    }

    let list = rows
        .into_iter()
        .map(|r| VicoConversation {
            id: r.get("id"),
            title: r.get("title"),
            last_message: r.get("last_message"),
            updated_at: r.get("updated_at"),
        })
        .collect();
    Ok(Json(list))
}

async fn create_conversation(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(p): Json<CreateConvReq>,
) -> AppResult<Json<VicoConversation>> {
    let uid = auth_user(&state, &headers).await?;
    let title = p
        .title
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|s| s.chars().take(80).collect::<String>())
        .unwrap_or_else(|| "New chat".to_string());
    let id = Uuid::new_v4();
    let r = sqlx::query(
        "INSERT INTO vico_conversations (id, user_id, title) VALUES ($1, $2, $3) RETURNING id, title, updated_at",
    )
    .bind(id)
    .bind(uid)
    .bind(&title)
    .fetch_one(&state.db)
    .await?;

    Ok(Json(VicoConversation {
        id: r.get("id"),
        title: r.get("title"),
        last_message: None,
        updated_at: r.get("updated_at"),
    }))
}

async fn delete_conversation(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(conv_id): Path<Uuid>,
) -> AppResult<Json<Value>> {
    let uid = auth_user(&state, &headers).await?;
    let res = sqlx::query("DELETE FROM vico_conversations WHERE id = $1 AND user_id = $2")
        .bind(conv_id)
        .bind(uid)
        .execute(&state.db)
        .await?;
    if res.rows_affected() == 0 {
        return Err(AppError::NotFound("conversation_not_found"));
    }
    Ok(Json(json!({ "deleted": true })))
}

async fn list_messages(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(conv_id): Path<Uuid>,
) -> AppResult<Json<Vec<VicoMessage>>> {
    let uid = auth_user(&state, &headers).await?;
    let owner: Option<Uuid> = sqlx::query_scalar("SELECT user_id FROM vico_conversations WHERE id = $1")
        .bind(conv_id)
        .fetch_optional(&state.db)
        .await?;
    if owner != Some(uid) {
        return Err(AppError::NotFound("conversation_not_found"));
    }

    let rows = sqlx::query(
        "SELECT id, conversation_id, role, content, created_at FROM vico_messages WHERE conversation_id = $1 ORDER BY created_at ASC LIMIT 200",
    )
    .bind(conv_id)
    .fetch_all(&state.db)
    .await?;

    let msgs = rows
        .into_iter()
        .map(|r| VicoMessage {
            id: r.get("id"),
            conversation_id: r.get("conversation_id"),
            role: r.get("role"),
            content: r.get("content"),
            created_at: r.get("created_at"),
        })
        .collect();
    Ok(Json(msgs))
}

fn eval_simple_arithmetic(input: &str) -> Option<f64> {
    let clean: String = input
        .trim()
        .trim_start_matches('=')
        .chars()
        .filter(|c| !c.is_whitespace())
        .collect();
    if clean.is_empty()
        || !clean
            .chars()
            .all(|c| c.is_ascii_digit() || matches!(c, '+' | '-' | '*' | '/' | '.'))
        || !clean.chars().any(|c| matches!(c, '+' | '-' | '*' | '/'))
    {
        return None;
    }
    let mut total = 0.0;
    let mut op = '+';
    let mut buf = String::new();
    let eval_term = |t: &str| -> Option<f64> {
        let mut acc = None;
        let mut mop = '*';
        let mut num = String::new();
        for ch in t.chars().chain(std::iter::once('*')) {
            if matches!(ch, '*' | '/') {
                let n: f64 = num.parse().ok()?;
                num.clear();
                acc = Some(match acc {
                    None => n,
                    Some(a) => {
                        if mop == '*' {
                            a * n
                        } else if n == 0.0 {
                            return None;
                        } else {
                            a / n
                        }
                    }
                });
                mop = ch;
            } else {
                num.push(ch);
            }
        }
        acc
    };
    for (idx, ch) in clean.char_indices() {
        if (ch == '+' || ch == '-')
            && idx > 0
            && !matches!(clean.as_bytes()[idx - 1], b'*' | b'/' | b'+' | b'-')
        {
            let v = eval_term(&buf)?;
            buf.clear();
            total = if op == '+' { total + v } else { total - v };
            op = ch;
        } else {
            buf.push(ch);
        }
    }
    let last = eval_term(&buf)?;
    Some(if op == '+' { total + last } else { total - last })
}

async fn builtin_vico_reply(state: &AppState, uid: Uuid, prompt: &str) -> String {
    let trimmed = prompt.trim();
    let low = trimmed.to_lowercase();

    if let Some(res) = eval_simple_arithmetic(trimmed) {
        let fmt = if res.fract().abs() < 1e-9 {
            format!("{}", res.round() as i64)
        } else {
            format!("{res:.4}")
        };
        return format!("The result of calculation `{trimmed}` is **{fmt}**.");
    }

    if low.contains("e2ee") || low.contains("encryption") || low.contains("security") {
        return "ChatW uses end-to-end encryption (E2EE) with ECDH P-256 key pairs for session key exchange, ECDSA P-256 for identity signing, and AES-256-GCM for each message on your device. The server only forwards the encrypted packet and does not keep the private key.".to_string();
    }

    if low.contains("What time?") || low.contains("today") || low.contains("day") || low.contains("time") {
        let vn_now = Utc::now() + chrono::Duration::hours(7);
        return format!(
            "Currently according to Vietnam time (UTC+7) it is **{}** on **{}**.",
            vn_now.format("%H:%M:%S"),
            vn_now.format("%d/%m/%Y")
        );
    }

    if low.contains("summary") && trimmed.chars().count() > 40 {
        let body = trimmed
            .trim_start_matches("summary")
            .trim_start_matches("Summary")
            .trim_start_matches(':')
            .trim();
        let sentences: Vec<&str> = body
            .split(['.', '\n', '!', '?'])
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .collect();
        let take_n = sentences.len().min(3);
        let summary = sentences[..take_n].join(". ");
        return format!("Main idea ({take_n}/{} sentences):\n• {summary}", sentences.len());
    }

    let friends_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM friendships WHERE user_a = $1 OR user_b = $1",
    )
    .bind(uid)
    .fetch_one(&state.db)
    .await
    .unwrap_or(0);

    if low.contains("Hello") || low == "hi" || low == "hello" || low.contains("hello") {
        return format!(
            "Hello! I am **Vico AI**.\nYou can ask yourself to quickly calculate (e.g. `125 * 24`), see the date and time, learn about E2EE security, summarize long text (`summary: `), or click the settings icon in the upper corner to connect your own LLM API (OpenAI / Groq / Ollama / OpenRouter). You are currently connected to {friends_count} friends on ChatW."
        );
    }

    format!(
        "I have recorded the question: \"{trimmed}\".\nVico AI is currently running in local mode (supports quick calculations, viewing date and time, summarizing text and answering ChatW features). To use the full AI language model (GPT / Gemini / Llama / DeepSeek), click the Settings button (gear) in the upper corner of this chat screen to enter your Endpoint & API Key."
    )
}

async fn call_external_llm(
    state: &AppState,
    endpoint: &str,
    api_key: &str,
    model: &str,
    history: &[(String, String)],
) -> Option<String> {
    let url = if endpoint.ends_with("/chat/completions") {
        endpoint.to_string()
    } else {
        format!("{}/chat/completions", endpoint.trim_end_matches('/'))
    };
    let mut msgs = vec![json!({
        "role": "system",
        "content": "You are Vico AI, the smart, concise and polite AI assistant inside the ChatW messaging app. Reply in Vietnamese unless the user uses another language."
    })];
    for (r, c) in history.iter().rev().take(16).collect::<Vec<_>>().into_iter().rev() {
        msgs.push(json!({ "role": r, "content": c }));
    }

    let mut req = state
        .http_client
        .post(&url)
        .json(&json!({ "model": model, "messages": msgs, "temperature": 0.7 }));
    if !api_key.trim().is_empty() {
        req = req.bearer_auth(api_key.trim());
    }
    let resp = req.send().await.ok()?;
    if !resp.status().is_success() {
        return None;
    }
    let body: Value = resp.json().await.ok()?;
    body.pointer("/choices/0/message/content")
        .and_then(|v| v.as_str())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

async fn send_message(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(conv_id): Path<Uuid>,
    Json(p): Json<SendMsgReq>,
) -> AppResult<Json<Value>> {
    let uid = auth_user(&state, &headers).await?;
    let owner: Option<Uuid> = sqlx::query_scalar("SELECT user_id FROM vico_conversations WHERE id = $1")
        .bind(conv_id)
        .fetch_optional(&state.db)
        .await?;
    if owner != Some(uid) {
        return Err(AppError::NotFound("conversation_not_found"));
    }

    let text = p.content.trim().to_string();
    if text.is_empty() || text.chars().count() > 8000 {
        return Err(AppError::BadRequest("invalid_message_content"));
    }

    let u_row = sqlx::query(
        "INSERT INTO vico_messages (conversation_id, role, content) VALUES ($1, 'user', $2) RETURNING id, conversation_id, role, content, created_at",
    )
    .bind(conv_id)
    .bind(&text)
    .fetch_one(&state.db)
    .await?;

    let msg_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM vico_messages WHERE conversation_id = $1")
        .bind(conv_id)
        .fetch_one(&state.db)
        .await
        .unwrap_or(1);

    if msg_count == 1 {
        let short_title: String = text.chars().take(48).collect();
        let _ = sqlx::query("UPDATE vico_conversations SET title = $2, updated_at = now() WHERE id = $1")
            .bind(conv_id)
            .bind(short_title)
            .execute(&state.db)
            .await;
    } else {
        let _ = sqlx::query("UPDATE vico_conversations SET updated_at = now() WHERE id = $1")
            .bind(conv_id)
            .execute(&state.db)
            .await;
    }

    // Check user or server LLM config
    let cfg_row = sqlx::query_as::<_, (Option<String>, Option<String>, String)>(
        "SELECT api_endpoint, api_key, model_name FROM vico_user_configs WHERE user_id = $1",
    )
    .bind(uid)
    .fetch_optional(&state.db)
    .await?;

    let ep = cfg_row
        .as_ref()
        .and_then(|c| c.0.clone())
        .or_else(|| std::env::var("VICO_API_URL").ok().filter(|s| !s.trim().is_empty()));
    let key = cfg_row
        .as_ref()
        .and_then(|c| c.1.clone())
        .or_else(|| std::env::var("VICO_API_KEY").ok())
        .unwrap_or_default();
    let model = cfg_row
        .map(|c| c.2)
        .or_else(|| std::env::var("VICO_MODEL").ok())
        .unwrap_or_else(|| "gpt-4o-mini".to_string());

    let reply_text = if let Some(endpoint) = ep.filter(|u| validate_endpoint(u)) {
        let hist_rows = sqlx::query_as::<_, (String, String)>(
            "SELECT role, content FROM vico_messages WHERE conversation_id = $1 ORDER BY created_at DESC LIMIT 16",
        )
        .bind(conv_id)
        .fetch_all(&state.db)
        .await
        .unwrap_or_default();

        match call_external_llm(&state, &endpoint, &key, &model, &hist_rows).await {
            Some(ans) => ans,
            None => builtin_vico_reply(&state, uid, &text).await,
        }
    } else {
        builtin_vico_reply(&state, uid, &text).await
    };

    let a_row = sqlx::query(
        "INSERT INTO vico_messages (conversation_id, role, content) VALUES ($1, 'assistant', $2) RETURNING id, conversation_id, role, content, created_at",
    )
    .bind(conv_id)
    .bind(&reply_text)
    .fetch_one(&state.db)
    .await?;

    let _ = sqlx::query("UPDATE vico_conversations SET updated_at = now() WHERE id = $1")
        .bind(conv_id)
        .execute(&state.db)
        .await;

    Ok(Json(json!({
        "user_message": VicoMessage {
            id: u_row.get("id"),
            conversation_id: u_row.get("conversation_id"),
            role: u_row.get("role"),
            content: u_row.get("content"),
            created_at: u_row.get("created_at"),
        },
        "assistant_message": VicoMessage {
            id: a_row.get("id"),
            conversation_id: a_row.get("conversation_id"),
            role: a_row.get("role"),
            content: a_row.get("content"),
            created_at: a_row.get("created_at"),
        },
    })))
}

async fn get_config(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> AppResult<Json<Value>> {
    let uid = auth_user(&state, &headers).await?;
    let row = sqlx::query_as::<_, (Option<String>, Option<String>, String)>(
        "SELECT api_endpoint, api_key, model_name FROM vico_user_configs WHERE user_id = $1",
    )
    .bind(uid)
    .fetch_optional(&state.db)
    .await?;

    match row {
        Some((ep, key, model)) => Ok(Json(json!({
            "api_endpoint": ep.unwrap_or_default(),
            "has_api_key": key.as_deref().is_some_and(|k| !k.trim().is_empty()),
            "model_name": model,
        }))),
        None => Ok(Json(json!({
            "api_endpoint": "",
            "has_api_key": false,
            "model_name": "gpt-4o-mini",
        }))),
    }
}

async fn update_config(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(p): Json<UpdateConfigReq>,
) -> AppResult<Json<Value>> {
    let uid = auth_user(&state, &headers).await?;
    let ep = p
        .api_endpoint
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string);
    if let Some(ref u) = ep {
        if !validate_endpoint(u) {
            return Err(AppError::BadRequest("invalid_api_endpoint"));
        }
    }
    let model = p
        .model_name
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|s| s.chars().take(80).collect::<String>())
        .unwrap_or_else(|| "gpt-4o-mini".to_string());

    let new_key = p.api_key.as_deref().map(str::trim).map(str::to_string);

    sqlx::query(
        r#"
        INSERT INTO vico_user_configs (user_id, api_endpoint, api_key, model_name, updated_at)
        VALUES ($1, $2, $3, $4, now())
        ON CONFLICT (user_id) DO UPDATE SET
            api_endpoint = EXCLUDED.api_endpoint,
            api_key = CASE WHEN $5 THEN EXCLUDED.api_key ELSE vico_user_configs.api_key END,
            model_name = EXCLUDED.model_name,
            updated_at = now()
        "#,
    )
    .bind(uid)
    .bind(&ep)
    .bind(&new_key)
    .bind(&model)
    .bind(new_key.is_some())
    .execute(&state.db)
    .await?;

    Ok(Json(json!({ "ok": true })))
}

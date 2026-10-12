//! Real CLI + HTTP + SQLite + live-file regressions. No user data or public upstreams.
use reqwest::{Client, Method};
use rusqlite::Connection;
use serde_json::{json, Value};
use std::{
    fs,
    net::TcpListener,
    path::Path,
    process::{Child, Command, Stdio},
    time::{Duration, Instant},
};
use tempfile::TempDir;

const TOKEN: &str = "workflow-test-admin-token";

struct Service {
    child: Option<Child>,
    home: TempDir,
    admin_port: u16,
    proxy_port: u16,
    client: Client,
}

impl Service {
    fn new() -> Self {
        // Keep both reservations open until distinct ports have been allocated.
        let admin = TcpListener::bind("127.0.0.1:0").unwrap();
        let proxy = TcpListener::bind("127.0.0.1:0").unwrap();
        let home = tempfile::tempdir().unwrap();
        fs::create_dir_all(home.path().join(".claude")).unwrap();
        fs::write(
            home.path().join(".claude/settings.json"),
            r#"{"env":{"API_TIMEOUT_MS":"1000"},"permissions":{"allow":["Read"]}}"#,
        )
        .unwrap();
        Self {
            child: None,
            home,
            admin_port: admin.local_addr().unwrap().port(),
            proxy_port: proxy.local_addr().unwrap().port(),
            client: Client::builder()
                .no_proxy()
                .timeout(Duration::from_secs(10))
                .build()
                .unwrap(),
        }
    }

    fn path(&self, path: &str) -> std::path::PathBuf {
        self.home.path().join(path)
    }

    fn live(&self) -> Value {
        read_json(&self.path(".claude/settings.json"))
    }

    fn db(&self) -> Connection {
        let conn = Connection::open(self.path(".cc-switch/cc-switch.db")).unwrap();
        conn.busy_timeout(Duration::from_secs(5)).unwrap();
        conn
    }

    async fn start(&mut self) {
        assert!(self.child.is_none());
        let log = fs::File::create(self.path("server.log")).unwrap();
        let mut command = Command::new(env!("CARGO_BIN_EXE_cc-switch-ui"));
        command
            .args([
                "start",
                "--host",
                "127.0.0.1",
                "--port",
                &self.admin_port.to_string(),
                "--proxy-port",
                &self.proxy_port.to_string(),
                "--admin-token",
                TOKEN,
            ])
            .current_dir(self.home.path())
            // Child-only overrides: never mutate the test runner's environment.
            .env("HOME", self.home.path())
            .env("USERPROFILE", self.home.path())
            .env("CC_SWITCH_TEST_HOME", self.home.path())
            .env("XDG_CONFIG_HOME", self.path("xdg-config"))
            .env("APPDATA", self.path("appdata"))
            .env("LOCALAPPDATA", self.path("local-appdata"))
            .env_remove("CC_SWITCH_ADMIN_TOKEN")
            .env_remove("CC_SWITCH_UI_DIR")
            .stdin(Stdio::null())
            .stdout(Stdio::from(log.try_clone().unwrap()))
            .stderr(Stdio::from(log));
        self.child = Some(command.spawn().unwrap());
        let deadline = Instant::now() + Duration::from_secs(15);
        loop {
            assert!(
                self.child.as_mut().unwrap().try_wait().unwrap().is_none(),
                "server exited: {}",
                fs::read_to_string(self.path("server.log")).unwrap()
            );
            if let Ok(response) = self
                .client
                .get(format!("http://127.0.0.1:{}/health", self.admin_port))
                .send()
                .await
            {
                if response.status().is_success() {
                    return;
                }
            }
            assert!(
                Instant::now() < deadline,
                "server readiness timeout: {}",
                fs::read_to_string(self.path("server.log")).unwrap()
            );
            tokio::time::sleep(Duration::from_millis(40)).await;
        }
    }

    fn stop(&mut self) {
        if let Some(mut child) = self.child.take() {
            let _ = child.kill();
            child.wait().unwrap();
        }
    }

    async fn request(&self, method: Method, path: &str, body: Value, status: u16) -> Value {
        let response = self
            .client
            .request(
                method,
                format!("http://127.0.0.1:{}{path}", self.admin_port),
            )
            .bearer_auth(TOKEN)
            .json(&body)
            .send()
            .await
            .unwrap();
        let actual = response.status().as_u16();
        let value: Value = response.json().await.unwrap();
        assert_eq!(actual, status, "{path}: {value}");
        value
    }

    async fn get(&self, path: &str) -> Value {
        self.request(Method::GET, path, Value::Null, 200).await
    }

    async fn post(&self, path: &str, body: Value) -> Value {
        self.request(Method::POST, path, body, 200).await
    }

    async fn success(&self, path: &str, body: Value) -> Value {
        let value = self.post(path, body).await;
        assert_eq!(value["success"], true, "{path}: {value}");
        value
    }

    async fn provider(&self, id: &str, endpoint: &str) {
        self.success(
            "/api/providers",
            json!({"id":id,"name":id,"create":true,
            "settingsConfig":{"env":{"ANTHROPIC_AUTH_TOKEN":format!("fake-{id}"),
            "ANTHROPIC_BASE_URL":endpoint}},"meta":{"apiFormat":"anthropic"}}),
        )
        .await;
    }
}

impl Drop for Service {
    fn drop(&mut self) {
        self.stop(); // Always reap the child before TempDir removes its files.
    }
}

fn read_json(path: &Path) -> Value {
    serde_json::from_slice(&fs::read(path).unwrap()).unwrap()
}

#[tokio::test]
async fn imported_oauth_signals_route_as_codex_without_api_key_fallback() {
    let mut service = Service::new();
    service.start().await;
    let original_live = service.live();
    for (index, mut declaration) in [
        json!({"authMode":"oauth_proxy"}),
        json!({"authBinding":{"authProvider":"codex_oauth","accountId":"synthetic-account"}}),
        json!({"authBinding":{"auth_provider":"codex_oauth","accountId":"synthetic-account"}}),
    ]
    .into_iter()
    .enumerate()
    {
        for format in ["gemini_native", "anthropic"] {
            let id = format!("legacy-auth-{index}-{format}");
            declaration["apiFormat"] = json!(format);
            service.success("/api/providers", json!({
                "id":id,"name":id,"create":true,
                "settingsConfig":{"env":{"ANTHROPIC_BASE_URL":"http://127.0.0.1:9","ANTHROPIC_AUTH_TOKEN":"synthetic-unused-api-key"}},
                "meta":declaration
            })).await;
            service
                .success("/api/proxy/target", json!({"provider_id":id}))
                .await;
            service.success("/api/proxy/start", Value::Null).await;
            assert_eq!(
                service.get("/api/proxy/status").await["active_target_provider_id"],
                id
            );
            // The isolated profile has no OAuth account. Authentication must
            // fail locally, rather than using the fake API key or endpoint.
            let response = service.client.post(format!("http://127.0.0.1:{}/v1/messages",service.proxy_port))
                .json(&json!({"model":"synthetic-model","stream":false,"messages":[{"role":"user","content":"fixture"}]}))
                .send().await.unwrap();
            assert_eq!(response.status(), reqwest::StatusCode::UNAUTHORIZED);
            service.success("/api/proxy/stop", Value::Null).await;
            assert_eq!(service.live(), original_live);
        }
    }
}

#[tokio::test]
async fn protocol_routes_preserve_prompts_and_convert_actual_http_responses() {
    use axum::{
        body::Body,
        extract::{OriginalUri, State},
        http::{header, HeaderMap},
        response::Response,
        routing::post,
        Json, Router,
    };
    use tokio::sync::mpsc;
    let (sender, mut captured) = mpsc::channel(16);
    let app = Router::new().fallback(post(|State(sender): State<mpsc::Sender<(String,HeaderMap,Value)>>, OriginalUri(uri): OriginalUri, headers: HeaderMap, Json(body): Json<Value>| async move {
        sender.send((uri.to_string(), headers, body.clone())).await.unwrap();
        let path = uri.path();
        if path.ends_with("count_tokens") {
            return Response::builder().header(header::CONTENT_TYPE,"application/json").body(Body::from(r#"{"input_tokens":42}"#)).unwrap();
        }
        let chat = path.ends_with("chat/completions");
        let responses = path.ends_with("responses");
        if body["stream"] == true {
            let events = if chat {
                "data: {\"id\":\"chat\",\"model\":\"test\",\"choices\":[{\"delta\":{\"content\":\"stream ok\"}}]}\n\ndata: {\"choices\":[{\"delta\":{},\"finish_reason\":\"stop\"}]}\n\ndata: [DONE]\n\n".to_string()
            } else {
                [
                    ("response.created",json!({"response":{"id":"r","model":"test"}})),
                    ("response.content_part.added",json!({"part":{"type":"output_text"}})),
                    ("response.output_text.delta",json!({"delta":"stream ok"})),
                    ("response.content_part.done",json!({})),
                    ("response.completed",json!({"response":{"status":"completed","usage":{"input_tokens":3,"output_tokens":2}}}))
                ].into_iter().map(|(name,data)|format!("event: {name}\ndata: {data}\n\n")).collect()
            };
            Response::builder().header(header::CONTENT_TYPE,"text/event-stream").body(Body::from(events)).unwrap()
        } else {
            let response = if chat { json!({"id":"chat","model":"test","choices":[{"message":{"content":"json ok"},"finish_reason":"stop"}]}) }
                else if responses { json!({"id":"r","model":"test","status":"completed","output":[{"type":"message","content":[{"type":"output_text","text":"json ok"}]}],"usage":{"input_tokens":3,"output_tokens":2}}) }
                else { json!({"id":"native","type":"message","role":"assistant","model":"test","content":[{"type":"text","text":"json ok"}],"stop_reason":"end_turn","usage":{"input_tokens":3,"output_tokens":2}}) };
            Response::builder().header(header::CONTENT_TYPE,"application/json").header("retry-after","3").body(Body::from(response.to_string())).unwrap()
        }
    })).with_state(sender);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let endpoint = format!("http://{}", listener.local_addr().unwrap());
    let task = tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    struct Mock(tokio::task::JoinHandle<()>);
    impl Drop for Mock {
        fn drop(&mut self) {
            self.0.abort();
        }
    }
    let _mock = Mock(task);
    let mut service = Service::new();
    service.start().await;
    let fixture = json!({"model":"gpt-5[1m]", "stream":false,"max_tokens":800,
        "system":[{"type":"text","text":"x-anthropic-billing-header: synthetic"},{"type":"text","text":"Preserve project policy","cache_control":{"type":"ephemeral"}}],
        "thinking":{"type":"adaptive","display":"omitted"},"output_config":{"effort":"high"},"context_management":{"edits":[]},"future_field":{"preserve":"native"},
        "tools":[{"name":"ToolSearch","input_schema":{"type":"object"}},{"name":"mcp__audit__echo","input_schema":{"type":"object"},"defer_loading":true}],
        "messages":[{"role":"system","content":"mid-turn rule"},{"role":"assistant","content":[{"type":"tool_use","id":"search","name":"ToolSearch","input":{}}]},
        {"role":"user","content":[{"type":"tool_result","tool_use_id":"search","content":[{"type":"tool_reference","tool_name":"mcp__audit__echo"}]},{"type":"text","text":"<system-reminder>keep this reminder</system-reminder>"}]}]});
    for (id, kind, format) in [
        ("protocol-native", "deepseek", "anthropic"),
        ("protocol-chat", "openrouter", "openai_chat"),
        ("protocol-responses", "openai_responses", "openai_responses"),
    ] {
        service.success("/api/providers",json!({"id":id,"name":id,"create":true,"settingsConfig":{"env":{"ANTHROPIC_BASE_URL":endpoint,"ANTHROPIC_AUTH_TOKEN":"synthetic-key"}},"meta":{"providerType":kind,"apiFormat":format}})).await;
        service
            .success("/api/proxy/target", json!({"provider_id":id}))
            .await;
        service.success("/api/proxy/start", Value::Null).await;
        let url = format!(
            "http://127.0.0.1:{}/v1/messages?beta=true",
            service.proxy_port
        );
        let response = service
            .client
            .post(&url)
            .header("anthropic-version", "2023-06-01")
            .header("anthropic-beta", "test-capability")
            .json(&fixture)
            .send()
            .await
            .unwrap();
        assert!(response.status().is_success());
        assert_eq!(response.headers()["retry-after"], "3");
        assert_eq!(
            response.json::<Value>().await.unwrap()["content"][0]["text"],
            "json ok"
        );
        let (path, headers, body) = captured.recv().await.unwrap();
        assert_eq!(headers["authorization"], "Bearer synthetic-key");
        match format {
            "anthropic" => {
                assert_eq!(path, "/v1/messages?beta=true");
                assert_eq!(headers["anthropic-beta"], "test-capability");
                assert_eq!(body, fixture);
                let response = service
                    .client
                    .post(format!(
                        "http://127.0.0.1:{}/v1/messages/count_tokens",
                        service.proxy_port
                    ))
                    .json(&json!({"model":"test","messages":[]}))
                    .send()
                    .await
                    .unwrap();
                assert_eq!(response.headers()["content-type"], "application/json");
                assert_eq!(response.json::<Value>().await.unwrap()["input_tokens"], 42);
                assert!(captured.recv().await.unwrap().0.ends_with("count_tokens"));
            }
            "openai_chat" => {
                assert_eq!(path, "/v1/chat/completions");
                assert_eq!(body["model"], "gpt-5");
                assert_eq!(body["messages"][0]["content"], "Preserve project policy");
                assert_eq!(body["messages"][1]["role"], "system");
                assert!(body["messages"][3]["content"]
                    .as_str()
                    .unwrap()
                    .contains("mcp__audit__echo"));
                assert_eq!(
                    body["messages"][4]["content"],
                    "<system-reminder>keep this reminder</system-reminder>"
                );
            }
            _ => {
                assert_eq!(path, "/v1/responses");
                assert_eq!(body["model"], "gpt-5");
                assert_eq!(body["max_output_tokens"], 800);
                assert_eq!(body["stream"], false);
                assert_eq!(body["input"][0]["role"], "system");
                assert!(body["input"][2]["output"][0]["text"]
                    .as_str()
                    .unwrap()
                    .contains("mcp__audit__echo"));
                assert_eq!(
                    body["input"][3]["content"][0]["text"],
                    "<system-reminder>keep this reminder</system-reminder>"
                );
            }
        }
        if format != "anthropic" {
            let count = service
                .client
                .post(format!(
                    "http://127.0.0.1:{}/v1/messages/count_tokens",
                    service.proxy_port
                ))
                .json(&json!({"model":"test","messages":[]}))
                .send()
                .await
                .unwrap();
            assert_eq!(count.status().as_u16(), 400);
            assert!(captured.try_recv().is_err());
            let mut streamed = fixture.clone();
            streamed["stream"] = json!(true);
            let response = service
                .client
                .post(&url)
                .json(&streamed)
                .send()
                .await
                .unwrap();
            assert!(response.status().is_success());
            assert_eq!(response.headers()["content-type"], "text/event-stream");
            let stream = response.text().await.unwrap();
            assert!(stream.contains("stream ok"));
            assert_eq!(stream.matches("event: message_stop").count(), 1);
            assert_eq!(captured.recv().await.unwrap().2["stream"], true);
        }
        service.success("/api/proxy/stop", Value::Null).await;
    }
    service.success("/api/providers",json!({"id":"protocol-gemini","name":"gemini","create":true,"settingsConfig":{"env":{"ANTHROPIC_BASE_URL":endpoint,"ANTHROPIC_AUTH_TOKEN":"synthetic-key"}},"meta":{"apiFormat":"gemini_native"}})).await;
    let live_before = fs::read(service.path(".claude/settings.json")).unwrap();
    let rejected = service
        .post(
            "/api/proxy/target",
            json!({"provider_id":"protocol-gemini"}),
        )
        .await;
    assert_eq!(rejected["success"], false);
    assert!(rejected["error"]
        .as_str()
        .unwrap()
        .contains("not implemented"));
    assert_eq!(
        service.get("/api/proxy/target").await["provider_id"],
        "protocol-responses"
    );
    service
        .db()
        .execute(
            "UPDATE providers SET meta=?1 WHERE id='protocol-responses'",
            [json!({"apiFormat":"gemini_native"}).to_string()],
        )
        .unwrap();
    assert_eq!(
        service.post("/api/proxy/start", Value::Null).await["success"],
        false
    );
    assert_eq!(
        fs::read(service.path(".claude/settings.json")).unwrap(),
        live_before
    );
    service
        .db()
        .execute(
            "UPDATE providers SET meta=?1 WHERE id='protocol-responses'",
            [json!({"apiFormat":"openai_responses"}).to_string()],
        )
        .unwrap();
    service.success("/api/proxy/start", Value::Null).await;
    // Even a record changed outside the guarded APIs cannot reach an upstream
    // through an unimplemented conversion.
    service
        .db()
        .execute(
            "UPDATE providers SET meta=?1 WHERE id='protocol-responses'",
            [json!({"providerType":"openrouter","apiFormat":"gemini_native"}).to_string()],
        )
        .unwrap();
    let response = service
        .client
        .post(format!(
            "http://127.0.0.1:{}/v1/messages",
            service.proxy_port
        ))
        .json(&fixture)
        .send()
        .await
        .unwrap();
    assert_eq!(response.status().as_u16(), 400);
    assert!(response.json::<Value>().await.unwrap()["error"]["message"]
        .as_str()
        .unwrap()
        .contains("not implemented"));
    assert!(captured.try_recv().is_err());
    service.success("/api/proxy/stop", Value::Null).await;
}

#[tokio::test]
async fn provider_route_and_restart_preserve_user_owned_files() {
    // Actual loopback upstream; requests must arrive with the selected provider's credential.
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let endpoint = format!("http://{}", listener.local_addr().unwrap());
    let app = axum::Router::new().route(
        "/v1/messages",
        axum::routing::post(
            |headers: axum::http::HeaderMap, axum::Json(body): axum::Json<Value>| async move {
                assert_eq!(headers["x-api-key"], "fake-route");
                assert_eq!(body["messages"][0]["content"], "hello");
                axum::Json(json!({"id":"msg_test","type":"message","role":"assistant",
                "model":"claude-test","content":[{"type":"text","text":"local upstream ok"}],
                "stop_reason":"end_turn","usage":{"input_tokens":1,"output_tokens":2}}))
            },
        ),
    );
    let upstream = tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    // Abort the mock even when an assertion unwinds this test.
    struct Mock(tokio::task::JoinHandle<()>);
    impl Drop for Mock {
        fn drop(&mut self) {
            self.0.abort();
        }
    }
    let _upstream = Mock(upstream);
    let mut service = Service::new();
    service.start().await;
    let unauthorized = service
        .client
        .get(format!(
            "http://127.0.0.1:{}/api/providers",
            service.admin_port
        ))
        .send()
        .await
        .unwrap();
    assert_eq!(unauthorized.status().as_u16(), 401);
    service.provider("direct", &endpoint).await;
    service.provider("route", &endpoint).await;
    service
        .success("/api/providers/direct/switch", Value::Null)
        .await;
    let original = service.get("/api/settings/claude-user").await;
    let edit = json!({"revision":original["revision"],"values":{"timeoutMs":"2000","nonessentialTraffic":"1"}});
    let saved = service
        .request(Method::PUT, "/api/settings/claude-user", edit.clone(), 200)
        .await;
    assert_eq!(saved["success"], true);
    service
        .request(Method::PUT, "/api/settings/claude-user", edit, 409)
        .await;
    service
        .success("/api/proxy/target", json!({"provider_id":"route"}))
        .await;
    // Binding failure must leave the live file byte-for-byte unchanged.
    let occupied = TcpListener::bind(("127.0.0.1", service.proxy_port)).unwrap();
    let before = fs::read(service.path(".claude/settings.json")).unwrap();
    assert_eq!(
        service.post("/api/proxy/start", Value::Null).await["success"],
        false
    );
    assert_eq!(
        fs::read(service.path(".claude/settings.json")).unwrap(),
        before
    );
    drop(occupied);
    service.success("/api/proxy/start", Value::Null).await;
    assert_eq!(
        service.get("/api/proxy/status").await["active_target_provider_id"],
        "route"
    );
    assert_eq!(
        service.get("/api/providers/current").await["current_provider_id"],
        "direct"
    );
    assert_eq!(
        service.live()["env"]["ANTHROPIC_BASE_URL"],
        format!("http://127.0.0.1:{}", service.proxy_port)
    );
    let response = service.client.post(format!("http://127.0.0.1:{}/v1/messages",service.proxy_port))
        .json(&json!({"model":"claude-test","max_tokens":16,"messages":[{"role":"user","content":"hello"}]}))
        .send().await.unwrap();
    assert!(response.status().is_success());
    assert_eq!(
        response.json::<Value>().await.unwrap()["content"][0]["text"],
        "local upstream ok"
    );
    service
        .request(Method::POST, "/api/database-backups", Value::Null, 409)
        .await;
    // Abrupt process termination retains durable takeover state for an explicit Stop after restart.
    service.stop();
    service.start().await;
    let status = service.get("/api/proxy/status").await;
    assert_eq!(status["running"], false);
    assert_eq!(status["recovery_required"], true);
    service.success("/api/proxy/stop", Value::Null).await;
    assert_eq!(service.live()["env"]["ANTHROPIC_AUTH_TOKEN"], "fake-direct");
    assert_eq!(service.live()["env"]["API_TIMEOUT_MS"], "2000");
    assert_eq!(
        service.live()["env"]["CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC"],
        "1"
    );
    assert_eq!(service.live()["permissions"]["allow"], json!(["Read"]));
    assert_eq!(
        service.get("/api/proxy/status").await["recovery_required"],
        false
    );
    service.success("/api/proxy/start", Value::Null).await;
    service.success("/api/proxy/stop", Value::Null).await;
    assert_eq!(service.live()["env"]["ANTHROPIC_AUTH_TOKEN"], "fake-direct");
    let backups = service.get("/api/file-backups").await;
    assert_eq!(backups["backups"].as_array().unwrap().len(), 3);
    assert!(!backups.to_string().contains("fake-direct"));
    let count: i64 = service
        .db()
        .query_row("SELECT COUNT(*) FROM proxy_live_backup", [], |row| {
            row.get(0)
        })
        .unwrap();
    assert_eq!(count, 0);
}

#[tokio::test]
async fn managed_files_import_sync_and_delete_recover_after_restart() {
    let mut service = Service::new();
    fs::write(
        service.path(".claude.json"),
        r#"{"externalSetting":true,"mcpServers":{"external":{"command":"external-tool"}}}"#,
    )
    .unwrap();
    service.start().await;
    service.success("/api/mcp/servers",json!({"id":"managed","name":"Managed","serverSpec":{"command":"managed-tool"},"appType":"claude_code","enabled":true})).await;
    let live = read_json(&service.path(".claude.json"));
    assert_eq!(live["externalSetting"], true);
    assert_eq!(live["mcpServers"]["external"]["command"], "external-tool");
    service.db().execute_batch("CREATE TRIGGER fail_import BEFORE INSERT ON mcp_servers WHEN NEW.id='external' BEGIN SELECT RAISE(ABORT,'injected import failure'); END;").unwrap();
    let report = service.post("/api/mcp/servers/import", Value::Null).await;
    assert_eq!(report["success"], false);
    assert_eq!(report["failed"], 1);
    assert_eq!(report["imported"], 0);
    assert_eq!(read_json(&service.path(".claude.json")), live);
    service
        .db()
        .execute_batch("DROP TRIGGER fail_import;")
        .unwrap();
    assert_eq!(
        service.post("/api/mcp/servers/import", Value::Null).await["imported"],
        1
    );
    let source = service.path(".cc-switch/skills/demo");
    fs::create_dir_all(&source).unwrap();
    fs::write(source.join("SKILL.md"), "# Demo\nUseful instructions\n").unwrap();
    service.success("/api/skills",json!({"id":"demo","name":"Demo","directory":"demo","appType":"claude_code","enabled":true})).await;
    assert_eq!(
        fs::read(service.path(".claude/skills/demo/SKILL.md")).unwrap(),
        fs::read(source.join("SKILL.md")).unwrap()
    );
    service
        .success("/api/skills/demo/toggle", Value::Null)
        .await;
    assert!(!service.path(".claude/skills/demo").exists());
    assert!(source.join("SKILL.md").exists());
    service
        .success("/api/skills/demo/toggle", Value::Null)
        .await;
    assert_eq!(
        service.post("/api/skills/sync", Value::Null).await["failed"],
        0
    );
    // Persist deletion, remove live entry, then simulate a DB failure clearing the durable intent.
    service.db().execute_batch("CREATE TRIGGER fail_intent_clear BEFORE DELETE ON managed_deletion BEGIN SELECT RAISE(ABORT,'injected intent clear failure'); END;").unwrap();
    let deleted = service
        .request(Method::DELETE, "/api/mcp/servers/managed", Value::Null, 500)
        .await;
    assert_eq!(deleted["deleted"], true);
    assert_eq!(deleted["recoveryRequired"], true);
    let pending = service.get("/api/managed-deletion").await["pending"].clone();
    assert!(pending["operationId"].is_string());
    assert!(read_json(&service.path(".claude.json"))["mcpServers"]
        .get("managed")
        .is_none());
    service.stop();
    service.start().await;
    assert_eq!(
        service.get("/api/managed-deletion").await["pending"],
        pending
    );
    service
        .request(
            Method::POST,
            "/api/managed-deletion/retry",
            json!({"operationId":"stale"}),
            409,
        )
        .await;
    service
        .request(Method::POST, "/api/database-backups", Value::Null, 409)
        .await;
    service
        .db()
        .execute_batch("DROP TRIGGER fail_intent_clear;")
        .unwrap();
    service
        .success(
            "/api/managed-deletion/retry",
            json!({"operationId":pending["operationId"]}),
        )
        .await;
    assert!(service.get("/api/managed-deletion").await["pending"].is_null());
    assert_eq!(
        service
            .db()
            .query_row(
                "SELECT COUNT(*) FROM mcp_servers WHERE id='managed'",
                [],
                |row| row.get::<_, i64>(0)
            )
            .unwrap(),
        0
    );
    assert_eq!(
        read_json(&service.path(".claude.json"))["mcpServers"]["external"]["command"],
        "external-tool"
    );
    service
        .request(Method::DELETE, "/api/skills/demo", Value::Null, 200)
        .await;
    assert!(!service.path(".claude/skills/demo").exists());
    assert!(source.join("SKILL.md").exists());
    service.stop();
    service.start().await;
    assert!(service.get("/api/skills").await["skills"]
        .as_array()
        .unwrap()
        .is_empty());
    assert!(service.get("/api/managed-deletion").await["pending"].is_null());
    let backups = service.get("/api/skill-backups").await;
    let backup = backups["backups"]
        .as_array()
        .unwrap()
        .iter()
        .find(|backup| backup["directory"] == "demo")
        .unwrap();
    service
        .success(
            "/api/skill-backups/restore",
            json!({"backupId":backup["id"]}),
        )
        .await;
    assert_eq!(
        fs::read(service.path(".claude/skills/demo/SKILL.md")).unwrap(),
        fs::read(source.join("SKILL.md")).unwrap()
    );
    service.stop();
    service.start().await;
    let restored = service.get("/api/skills").await;
    assert_eq!(restored["skills"].as_array().unwrap().len(), 1);
    assert_eq!(restored["skills"][0]["directory"], "demo");
    assert_eq!(restored["skills"][0]["enabled"], true);
}

#[tokio::test]
async fn staged_database_restore_applies_only_on_restart_and_preserves_live_files() {
    let mut service = Service::new();
    service.start().await;
    service.provider("saved", "http://127.0.0.1:1").await;
    service
        .success("/api/providers/saved/switch", Value::Null)
        .await;
    let backup = service.success("/api/database-backups", Value::Null).await;
    service.provider("later", "http://127.0.0.1:2").await;
    service
        .success("/api/providers/later/switch", Value::Null)
        .await;
    let before = fs::read(service.path(".claude/settings.json")).unwrap();
    service
        .success("/api/database-backups/restore", json!({"id":backup["id"]}))
        .await;
    assert_eq!(
        service.get("/api/database-backups").await["pending"]["sourceId"],
        backup["id"]
    );
    assert_eq!(
        service
            .db()
            .query_row(
                "SELECT COUNT(*) FROM providers WHERE id='later'",
                [],
                |row| row.get::<_, i64>(0)
            )
            .unwrap(),
        1
    );
    service
        .success("/api/database-backups/cancel", Value::Null)
        .await;
    assert!(service.get("/api/database-backups").await["pending"].is_null());
    service
        .success("/api/database-backups/restore", json!({"id":backup["id"]}))
        .await;
    service.stop();
    service.start().await;
    assert_eq!(
        service
            .db()
            .query_row(
                "SELECT COUNT(*) FROM providers WHERE id='later'",
                [],
                |row| row.get::<_, i64>(0)
            )
            .unwrap(),
        0
    );
    assert!(service.get("/api/providers/current").await["current_provider_id"].is_null());
    assert_eq!(
        fs::read(service.path(".claude/settings.json")).unwrap(),
        before
    );
    let backups = service.get("/api/database-backups").await;
    assert!(backups["pending"].is_null());
    assert!(backups["entries"]
        .as_array()
        .unwrap()
        .iter()
        .any(|entry| entry["kind"] == "safety" && entry["protected"] == true));
    // Re-selection must use the restored credential instead of backfilling the live 'later' key.
    service
        .success("/api/providers/saved/switch", Value::Null)
        .await;
    assert_eq!(service.live()["env"]["ANTHROPIC_AUTH_TOKEN"], "fake-saved");
}

#[tokio::test]
async fn provider_edits_and_failed_selection_recover_without_overwriting_external_edits() {
    let mut service = Service::new();
    service.start().await;
    service.provider("first", "http://127.0.0.1:1").await;
    service.provider("second", "http://127.0.0.1:2").await;
    service
        .success("/api/providers/first/switch", Value::Null)
        .await;
    let base = service.get("/api/providers/first").await["provider"].clone();
    let mut edit = base.clone();
    edit["base"] = base.clone();
    edit["settingsConfig"]["env"]["ANTHROPIC_AUTH_TOKEN"] = json!("fake-edited-first");
    let edited = service
        .request(Method::PUT, "/api/providers/first", edit.clone(), 200)
        .await;
    assert_eq!(edited["success"], true);
    assert_eq!(
        service.live()["env"]["ANTHROPIC_AUTH_TOKEN"],
        "fake-edited-first"
    );
    // A second tab changing the same key from the old baseline must receive a conflict.
    edit["settingsConfig"]["env"]["ANTHROPIC_AUTH_TOKEN"] = json!("fake-stale-tab");
    service
        .request(Method::PUT, "/api/providers/first", edit, 409)
        .await;
    service.db().execute_batch("CREATE TRIGGER fail_selection BEFORE UPDATE OF is_current ON providers WHEN NEW.is_current=1 BEGIN SELECT RAISE(ABORT,'injected selection failure'); END;").unwrap();
    let failed = service
        .request(
            Method::POST,
            "/api/providers/second/switch",
            Value::Null,
            500,
        )
        .await;
    assert_eq!(failed["recovery_required"], true);
    assert_eq!(service.live()["env"]["ANTHROPIC_AUTH_TOKEN"], "fake-second");
    assert_eq!(
        service.get("/api/providers/current").await["current_provider_id"],
        "first"
    );
    service.stop();
    // A user edit made while the service is down must survive journal replay.
    let mut live = service.live();
    live["env"]["API_TIMEOUT_MS"] = json!("3000");
    fs::write(
        service.path(".claude/settings.json"),
        serde_json::to_vec(&live).unwrap(),
    )
    .unwrap();
    service
        .db()
        .execute_batch("DROP TRIGGER fail_selection;")
        .unwrap();
    service.start().await;
    assert_eq!(
        service.get("/api/providers/current").await["recovery_required"],
        true
    );
    service.success("/api/providers/recover", Value::Null).await;
    let current = service.get("/api/providers/current").await;
    assert_eq!(current["current_provider_id"], "second");
    assert_eq!(current["recovery_required"], false);
    assert_eq!(service.live(), live);
    assert!(!service.path(".cc-switch/provider-operation.json").exists());
    assert_eq!(
        service
            .db()
            .query_row("SELECT id FROM providers WHERE is_current=1", [], |row| row
                .get::<_, String>(0))
            .unwrap(),
        "second"
    );
    service.stop();
    service.start().await;
    assert_eq!(
        service.get("/api/providers/current").await["current_provider_id"],
        "second"
    );
    assert_eq!(service.live(), live);
}

#[tokio::test]
async fn selected_import_removal_and_explicit_restore_survive_service_restart() {
    let mut service = Service::new();
    let source = service.path(".claude/plugins/example/skills/demo");
    fs::create_dir_all(&source).unwrap();
    fs::write(source.join("SKILL.md"), "demo").unwrap();
    service.start().await;
    let candidates = service.get("/api/managed-items/skill/candidates").await;
    assert_eq!(candidates["items"][0]["id"], "demo");
    assert!(service.get("/api/skills").await["skills"]
        .as_array()
        .unwrap()
        .is_empty());
    service
        .success("/api/managed-items/skill/import", json!({"ids":["demo"]}))
        .await;
    service.success("/api/skills/sync", json!({})).await;
    assert!(service.path(".claude/skills/demo/SKILL.md").is_file());
    service
        .request(Method::DELETE, "/api/skills/local:demo", Value::Null, 200)
        .await;
    service.stop();
    service.start().await;
    assert!(
        service.get("/api/managed-items/skill/candidates").await["items"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    assert_eq!(
        service.post("/api/skills/import", json!({})).await["imported"],
        0
    );
    service.success("/api/skills/sync", json!({})).await;
    assert!(!service.path(".claude/skills/demo").exists());
    assert_eq!(
        service.get("/api/managed-items/skill/removed").await[0]["id"],
        "demo"
    );
    service
        .success("/api/managed-items/skill/restore", json!({"ids":["demo"]}))
        .await;
    assert_eq!(
        service.get("/api/skills").await["skills"][0]["enabled"],
        false
    );
    service.success("/api/skills/sync", json!({})).await;
    assert!(!service.path(".claude/skills/demo").exists());
    service
        .success("/api/skills/local:demo/toggle", json!({}))
        .await;
    assert!(service.path(".claude/skills/demo/SKILL.md").is_file());
}

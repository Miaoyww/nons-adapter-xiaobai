use crate::{AdapterResult, HostRequest, Http, Request, Response};

wit_bindgen::generate!({
    path: ["vendor/wit/host", "vendor/wit/plugin"],
    inline: "package nons:xiaobai; world adapter { import nons:host/transport@1.1.0; import nons:plugin/host@1.0.0; export initialize: func() -> result<_, string>; export shutdown: func() -> result<_, string>; export call: func(method: string, args: string) -> result<string, string>; }",
    world: "nons:xiaobai/adapter",
    generate_all,
});
struct Adapter;
struct Host;
impl Http for Host {
    fn send(&self, request: Request) -> AdapterResult<Response> {
        use nons::host::transport as host;
        let request = HostRequest::from(request);
        let headers = request
            .headers
            .into_iter()
            .map(|(name, value)| host::Header { name, value })
            .collect();
        let response = host::http_request(&host::Request {
            url: request.url,
            method: request.method.into(),
            headers,
            body: request.body,
            follow_redirects: request.follow_redirects,
            timeout_ms: request.timeout_ms,
        })
        .map_err(|error| match error.as_str() {
            "cancelled" => "cancelled",
            "timeout" => "timeout",
            "staleContext" => "staleContext",
            "permissionDenied" => "permissionDenied",
            _ => "network",
        })?;
        Ok(Response {
            url: response.url,
            status: response.status,
            headers: response
                .headers
                .into_iter()
                .map(|h| (h.name, h.value))
                .collect(),
            body: response.body,
        })
    }
}
impl Guest for Adapter {
    fn initialize() -> std::result::Result<(), String> {
        Ok(())
    }
    fn shutdown() -> std::result::Result<(), String> {
        Ok(())
    }
    fn call(method: String, args: String) -> std::result::Result<String, String> {
        let result = crate::call(&Host, &method, &args);
        report(&result);
        Ok(result)
    }
}
export!(Adapter);

fn report(result: &str) {
    let value: serde_json::Value = match serde_json::from_str(result) {
        Ok(value) => value,
        Err(_) => return,
    };
    if value["status"] != "error" {
        return;
    }
    let code = value["data"]["code"].as_str().unwrap_or("internal");
    let class = match code {
        "network" | "timeout" | "cancelled" => "network",
        "rateLimited" | "permissionDenied" | "notFound" => "upstream",
        _ => "protocol",
    };
    let _ = nons::plugin::host::call(
        "diagnostic.report",
        &serde_json::json!({"stage":"music.call","class":class,"code":code}).to_string(),
    );
}

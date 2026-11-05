//! 小白音乐: exact platform ID resource resolution through the shared website protocol.
#[cfg(target_arch = "wasm32")]
mod component;

#[cfg(any(target_arch = "wasm32", test))]
use nons_music_web_provider::{AdapterResult, Http, Response};
use nons_music_web_provider::{Config, Request};

const CONFIG: Config = Config {
    base: "https://music.90svip.cn/",
    user_agent: "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/144.0.0.0 Safari/537.36",
    manifest: include_str!("../manifest.json"),
};

/// Host request mapping kept independent of the generated WIT bindings.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HostRequest {
    pub url: String,
    pub method: &'static str,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
    pub follow_redirects: bool,
    pub timeout_ms: u32,
}
impl From<Request> for HostRequest {
    fn from(value: Request) -> Self {
        Self {
            url: value.url,
            method: value.method,
            headers: value.headers,
            body: value.body,
            follow_redirects: value.follow_redirects,
            timeout_ms: value.timeout_ms,
        }
    }
}

pub fn call(http: &impl nons_music_web_provider::Http, method: &str, args: &str) -> String {
    nons_music_web_provider::call(http, method, args, CONFIG)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{json, Value};
    use std::cell::RefCell;

    struct NoNetwork;
    impl nons_music_web_provider::Http for NoNetwork {
        fn send(
            &self,
            _: nons_music_web_provider::Request,
        ) -> nons_music_web_provider::AdapterResult<nons_music_web_provider::Response> {
            panic!("descriptor and unsupported methods must not access network")
        }
    }

    #[test]
    fn descriptor_identity_and_no_search() {
        let value: serde_json::Value =
            serde_json::from_str(&call(&NoNetwork, "music.resource-descriptor", "null")).unwrap();
        assert_eq!(value["data"]["id"], "xiaobai");
        assert_eq!(value["data"]["searchPlatforms"], serde_json::json!([]));
        assert_eq!(value["data"]["contractVersion"], 2);
        assert!(value["data"].get("coverHosts").is_none());
        let manifest: Value = serde_json::from_str(include_str!("../manifest.json")).unwrap();
        assert_eq!(manifest["version"], "1.0.0");
        assert!(manifest["resourceProvider"].get("coverHosts").is_none());
        assert!(manifest["permissions"].get("httpHosts").is_none());
        assert!(manifest["permissions"].get("websocketHosts").is_none());
        assert_eq!(
            manifest["permissions"],
            serde_json::json!({"system":false,"state":false,"credentials":false})
        );
        let value: serde_json::Value =
            serde_json::from_str(&call(&NoNetwork, "music.search-resources", "null")).unwrap();
        assert_eq!(value["data"]["code"], "unsupported");
    }

    #[test]
    fn host_mapping_preserves_redirect_and_timeout_values() {
        let request = Request {
            url: "https://example.org/audio".into(),
            method: "GET",
            headers: vec![("Range".into(), "bytes=0-1".into())],
            body: vec![],
            follow_redirects: true,
            timeout_ms: 5000,
        };
        let mapped = HostRequest::from(request);
        assert_eq!(mapped.timeout_ms, 5000);
        assert!(mapped.follow_redirects);
        assert_eq!(mapped.url, "https://example.org/audio");
    }

    fn input(platform: &str, id: &str) -> Value {
        json!({"request":{"track":{"reference":{"source":platform,"kind":"track","id":id},"title":"Test track","aliases":[],"artist":"Test artist","artists":[],"album":"","albumReference":null,"durationMs":180000,"cover":{"original":"","small":"","normal":"","large":"","exlarge":""},"associations":[]},"preferredQuality":"exhigh","allowDowngrade":false}})
    }

    fn mp3() -> Vec<u8> {
        let mut data = vec![0; 6890 * 1044];
        for offset in (0..data.len()).step_by(1044) {
            data[offset..offset + 4].copy_from_slice(&[255, 251, 224, 0]);
        }
        data
    }

    struct Fixture {
        media: Vec<u8>,
        requests: RefCell<Vec<Request>>,
        row_url: &'static str,
        final_url: &'static str,
        api_final: &'static str,
        home_final: &'static str,
        home_padding: usize,
        post_padding: usize,
    }
    impl Fixture {
        fn new() -> Self {
            Self {
                media: mp3(),
                requests: RefCell::new(vec![]),
                row_url: "api.php?get=url&id=test",
                final_url: "http://user:secret@127.0.0.1:8088/final.mp3",
                api_final: "https://music.90svip.cn/resolved/",
                home_final: "https://music.90svip.cn/home-final",
                home_padding: 0,
                post_padding: 0,
            }
        }
    }
    impl Http for Fixture {
        fn send(&self, request: Request) -> AdapterResult<Response> {
            assert!(request.follow_redirects, "{}", request.url);
            assert_eq!(request.timeout_ms, 0, "{}", request.url);
            self.requests.borrow_mut().push(request.clone());
            if request.method == "POST" {
                let mut body = json!({
                    "code": 200,
                    "data": [{"songid": "347230", "url": self.row_url}],
                })
                .to_string()
                .into_bytes();
                body.resize(body.len() + self.post_padding, b' ');
                return Ok(Response {
                    url: self.api_final.into(),
                    status: 200,
                    headers: vec![],
                    body,
                });
            }
            if request.url == CONFIG.base || request.url == self.home_final {
                let mut body =
                    br#"<script>document.cookie="mk_visit=visit;path=/";</script>"#.to_vec();
                body.resize(body.len() + self.home_padding, b' ');
                return Ok(Response {
                    url: self.home_final.into(),
                    status: 200,
                    headers: vec![(
                        "Set-Cookie".into(),
                        "session=anonymous; Path=/; Secure; HttpOnly".into(),
                    )],
                    body,
                });
            }
            let range = request
                .headers
                .iter()
                .find(|(name, _)| name.eq_ignore_ascii_case("Range"))
                .map(|(_, value)| value.as_str())
                .expect("media request must carry Range");
            let (start, end) = range
                .strip_prefix("bytes=")
                .unwrap()
                .split_once('-')
                .unwrap();
            let start = start.parse::<usize>().unwrap();
            let end = end.parse::<usize>().unwrap().min(self.media.len() - 1);
            Ok(Response {
                url: self.final_url.into(),
                status: 206,
                headers: vec![
                    (
                        "Content-Range".into(),
                        format!("bytes {start}-{end}/{}", self.media.len()),
                    ),
                    ("ETag".into(), "\"fixture\"".into()),
                ],
                body: self.media[start..=end].to_vec(),
            })
        }
    }

    #[test]
    fn resolve_without_hosts_accepts_private_address_and_large_final_url_response() {
        let fixture = Fixture::new();
        let fixture = Fixture {
            post_padding: 200 * 1024,
            home_padding: 200 * 1024,
            ..fixture
        };
        let value: Value = serde_json::from_str(&call(
            &fixture,
            "music.resolve-resource",
            &input("netease", "347230").to_string(),
        ))
        .unwrap();
        assert_eq!(value["status"], "ok", "{value}");
        assert_eq!(value["data"]["url"], fixture.final_url);
        assert_eq!(value["data"]["metadata"]["actualQuality"], "320k");
        assert!(fixture
            .requests
            .borrow()
            .iter()
            .all(|request| request.follow_redirects));
        let requests = fixture.requests.borrow();
        let home: Vec<_> = requests
            .iter()
            .filter(|request| request.method == "GET" && request.url == CONFIG.base)
            .collect();
        assert_eq!(home.len(), 2);
        assert!(home.iter().all(|request| request.url == CONFIG.base));
        let media: Vec<_> = requests
            .iter()
            .filter(|request| request.url.contains("api.php") || request.url == fixture.final_url)
            .collect();
        assert_eq!(media[0].url, format!("{}{}", CONFIG.base, fixture.row_url));
        assert!(media[1..]
            .iter()
            .all(|request| request.url == fixture.final_url));
    }
}

use rand::Rng;
use rand::{distributions::Alphanumeric, Rng};
use reqwest::{Client, Method, StatusCode};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct DVHOST {
    host: String,
    ip: String,
    auth_token: Option<String>,
    system: Option<Value>,
    inbounds: HashMap<String, Vec<String>>,
    client: Client,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DVHOSTResponse {
    status: u16,
    data: Value,
}

impl DVHOST {
    pub fn new(host: String, ip: String, username: String, password: String) -> Self {
        let formatted_host = Self::format_server_url(&host);
        let formatted_ip = Self::format_server_url(&ip);

        let client = Client::new();
        let mut instance = Self {
            host: formatted_host,
            ip: formatted_ip,
            auth_token: None,
            system: None,
            inbounds: HashMap::new(),
            client,
        };

        instance.inbounds.insert(
            "vmess".to_string(),
            vec!["VMess TCP".to_string(), "VMess Websocket".to_string()],
        );
        instance.inbounds.insert(
            "vless".to_string(),
            vec![
                "VLESS TCP REALITY".to_string(),
                "VLESS GRPC REALITY".to_string(),
            ],
        );
        instance.inbounds.insert(
            "shadowsocks".to_string(),
            vec!["Shadowsocks TCP".to_string()],
        );

        instance.auth_token = instance.auth_token(&username, &password);
        instance.system = instance.system().data.into();

        instance
    }

    pub fn system(&self) -> DVHOSTResponse {
        if let Some(system) = &self.system {
            return DVHOSTResponse {
                status: 200,
                data: system.clone(),
            };
        }

        self.send_request("/system", None, Method::GET, true)
    }

    pub fn get_users(&self) -> DVHOSTResponse {
        self.send_request("/users", None, Method::GET, true)
    }

    pub fn get_user(&self, username: &str) -> DVHOSTResponse {
        let path = format!("/user/{}", username);
        let response = self.send_request(&path, None, Method::GET, true);

        if response.status == 200 {
            let mut user = response.data.clone();
            let expire = user["expire"].as_i64().unwrap_or(0);
            let current_time = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs() as i64;

            let days = (expire - current_time) / (60 * 60 * 24);
            let data_limit = user["data_limit"].as_f64().unwrap_or(0.0);
            let used_traffic = user["used_traffic"].as_f64().unwrap_or(0.0);

            user["remaining_days"] = json!(days.max(0));
            user["remaining_traffic"] = json!((data_limit - used_traffic).max(0.0));
            user["used_percent"] = json!((used_traffic / data_limit * 100.0).round());

            return DVHOSTResponse {
                status: 200,
                data: user,
            };
        }

        response
    }

    pub fn delete_user(&self, username: &str) -> DVHOSTResponse {
        let path = format!("/user/{}", username);
        self.send_request(&path, None, Method::DELETE, true)
    }

    pub fn add_user(
        &self,
        username: &str,
        volume: f64,  // Data volume in GB
        days: i64,    // Validity in days
        status: bool, // User status: false = on_hold, true = active
        note: &str,   // Optional note
        onhold: bool, // Whether user should be on_hold
    ) -> DVHOSTResponse {
        // Convert volume to bytes
        let volume_bytes = volume * 1024.0 * 1024.0 * 1024.0;

        // Calculate expiration and on_hold parameters
        let current_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        let (expire, on_hold_timeout, on_hold_expire_duration) = if onhold {
            // On_hold mode: expiration is disabled
            let hold_timeout = current_time + (days * 24 * 60 * 60);
            let hold_duration = days * 24 * 60 * 60;
            (0, Some(hold_timeout), Some(hold_duration))
        } else {
            let expire_time = if days > 0 {
                current_time + (days * 24 * 60 * 60)
            } else {
                0
            };
            (expire_time, None, None)
        };

        let data = json!({
            "username": username,
            "proxies": self.proxies(),
            "inbounds": self.get_inbounds(),
            "expire": expire,
            "data_limit": volume_bytes,
            "data_limit_reset_strategy": "no_reset",
            "status": if status { "active" } else { "on_hold" },
            "note": note,
            "on_hold_timeout": on_hold_timeout,
            "on_hold_expire_duration": on_hold_expire_duration
        });

        self.send_request("/user", Some(data.to_string()), Method::POST, true)
    }

    pub fn generate_unique_name(prefix: &str, total_length: usize) -> String {
        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        let micros = now.as_micros().to_string();

        let mut rng = rand::thread_rng();
        let random_bytes: [u8; 4] = rng.gen();
        let random_string = hex::encode(random_bytes);

        let mut unique_name = format!("{}{}{}", prefix, micros, random_string);

        if unique_name.len() > total_length {
            let remaining_length = total_length.saturating_sub(prefix.len());
            let combined = format!("{}{}", micros, random_string);
            unique_name = format!(
                "{}{}",
                prefix,
                combined.chars().take(remaining_length).collect::<String>()
            );
        }

        unique_name
    }

    pub fn reset_user_traffic(&self, username: &str) -> DVHOSTResponse {
        let path = format!("/user/{}/reset", username);
        self.send_request(&path, None, Method::POST, true)
    }

    pub fn revoke_user_sub(&self, username: &str) -> DVHOSTResponse {
        let path = format!("/user/{}/revoke", username);
        self.send_request(&path, None, Method::POST, true)
    }

    pub fn sub_info(&self, sub_link: &str) -> DVHOSTResponse {
        let path = format!("/sub/{}/info", sub_link);
        self.send_request(&path, None, Method::GET, true)
    }

    pub fn edit_user(&self, username: &str, update: Value) -> DVHOSTResponse {
        let user_response = self.get_user(username);
        if user_response.status != 200 {
            return user_response;
        }

        let user = user_response.data;
        let status = update
            .get("status")
            .map(|v| v.as_bool().unwrap_or(false))
            .unwrap_or(user["status"].as_str().unwrap_or("") == "active");

        let toggle_status = update
            .get("toggle_status")
            .map(|v| v.as_bool().unwrap_or(false))
            .unwrap_or(false);

        let final_status = if toggle_status { !status } else { status };

        let expire = update
            .get("expire")
            .map(|v| v.as_i64().unwrap_or(0))
            .unwrap_or(user["expire"].as_i64().unwrap_or(0));

        let add_days = update
            .get("add_days")
            .map(|v| v.as_i64().unwrap_or(0))
            .unwrap_or(0);

        let data_limit = update
            .get("volume")
            .map(|v| v.as_f64().unwrap_or(0.0))
            .unwrap_or(user["data_limit"].as_f64().unwrap_or(0.0));

        let add_volume = update
            .get("add_volume")
            .map(|v| v.as_f64().unwrap_or(0.0))
            .unwrap_or(0.0);

        let data = json!({
            "proxies": user["proxies"],
            "inbounds": user["inbounds"],
            "expire": expire + (add_days * 24 * 60 * 60),
            "data_limit": data_limit + (add_volume * 1024.0 * 1024.0 * 1024.0),
            "data_limit_reset_strategy": update.get("data_limit_reset_strategy")
                .unwrap_or(&user["data_limit_reset_strategy"]),
            "status": if final_status { "active" } else { "disabled" },
            "note": update.get("note").unwrap_or(&user["note"]),
            "on_hold_timeout": user["on_hold_timeout"],
            "on_hold_expire_duration": user["on_hold_expire_duration"],
        });

        let path = format!("/user/{}", username);
        self.send_request(&path, Some(data.to_string()), Method::PUT, true)
    }

    fn proxies(&self, vmess: bool, vless: bool, shadow_socks: bool) -> Value {
        let mut proxies = json!({});

        if vmess {
            proxies["vmess"] = json!({
                "id": self.gen_user_id()
            });
        }

        if vless {
            proxies["vless"] = json!({
                "id": self.gen_user_id(),
                "flow": ""
            });
        }

        if shadow_socks {
            proxies["shadowsocks"] = json!({
                "password": self.random_string(6),
                "method": "chacha20-ietf-poly1305"
            });
        }

        proxies
    }

    fn inbounds(&self, vmess: bool, vless: bool, shadow_socks: bool) -> Value {
        let mut inbounds = json!({});

        if vmess {
            inbounds["vmess"] = json!(self.inbounds["vmess"]);
        }

        if vless {
            inbounds["vless"] = json!(self.inbounds["vless"]);
        }

        if shadow_socks {
            inbounds["shadowsocks"] = json!(self.inbounds["shadowsocks"]);
        }

        inbounds
    }

    fn format_server_url(url: &str) -> String {
        if url.is_empty() {
            return String::new();
        }

        let mut formatted = url.to_string();
        if !formatted.ends_with('/') {
            formatted.push('/');
        }

        formatted
    }

    fn gen_user_id(&self) -> String {
        Uuid::new_v4().to_string()
    }

    fn random_string(&self, length: usize) -> String {
        rand::thread_rng()
            .sample_iter(&Alphanumeric)
            .take(length)
            .map(char::from)
            .collect()
    }

    fn auth_token(&self, username: &str, password: &str) -> Option<String> {
        let data = format!("username={}&password={}", username, password);
        let response = self.send_request("/admin/token", Some(data), Method::POST, false);

        if response.status == 200 {
            response.data["access_token"]
                .as_str()
                .map(|s| s.to_string())
        } else {
            None
        }
    }

    fn send_request(
        &self,
        path: &str,
        data: Option<String>,
        method: Method,
        require_auth: bool,
    ) -> DVHOSTResponse {
        if require_auth && self.auth_token.is_none() {
            return DVHOSTResponse {
                status: 401,
                data: json!({}),
            };
        }

        let url = if self.ip.is_empty() {
            format!("{}{}", self.host, path)
        } else {
            format!("{}{}", self.ip, path)
        };

        let mut request = self.client.request(method.clone(), &url);

        if let Some(auth_token) = &self.auth_token {
            request = request.header("Authorization", format!("Bearer {}", auth_token));
        }

        if let Some(body) = data {
            request = request.header(
                "Content-Type",
                if method == Method::POST || method == Method::PUT {
                    "application/json"
                } else {
                    "application/x-www-form-urlencoded"
                },
            );
            request = request.body(body);
        }

        match request.send() {
            Ok(response) => {
                let status = response.status().as_u16();
                let data = response.json().unwrap_or(json!({}));
                DVHOSTResponse { status, data }
            }
            Err(_) => DVHOSTResponse {
                status: 500,
                data: json!({}),
            },
        }
    }
}


let name = generate_unique_name("user_", 20);
println!("Generated name: {}", name);
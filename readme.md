# DVHOST - Marzban Panel API Client

A Rust implementation for interacting with Marzban panel API with improved performance and type safety.

## Features

- **User Management**
  - Create, retrieve, update, and delete users
  - Reset user traffic
  - Revoke user subscriptions
- **Proxy Configuration**
  - Support for VMess, VLESS, and Shadowsocks protocols
  - Custom inbound configurations
- **System Information**
  - Retrieve panel system status
  - Get subscription information
- **Authentication**
  - Secure token handling
  - Automatic reauthentication

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
dvhost = { git = "https://github.com/dev-ir/Marzban-Api-Rust.git" }
```

## Usage

### Basic Setup

```rust
use dvhost::DVHOST;

let panel = DVHOST::new(
    "https://your-panel.com".to_string(),
    "".to_string(),  // Optional alternative IP
    "admin".to_string(),
    "password".to_string()
);
```

### User Operations

**Create User:**
```rust
let response = panel.add_user(
    "new_user",
    10.0,  // 10GB data limit
    30,    // 30 days expiration
    true,  // active
    "Test user",
    true,  // enable VLESS
    true,  // enable VMess
    false  // disable Shadowsocks
);
```

**Get User Info:**
```rust
let user_info = panel.get_user("existing_user");
```

**Delete User:**
```rust
let delete_result = panel.delete_user("old_user");
```

### System Info

**Get All Users:**
```rust
let all_users = panel.get_users();
```

**Get Panel Status:**
```rust
let system_status = panel.system();
```

## API Reference

### Methods

| Method | Description | Parameters |
|--------|-------------|------------|
| `new()` | Initialize client | host, ip, username, password |
| `system()` | Get panel system info | - |
| `get_users()` | List all users | - |
| `get_user()` | Get specific user | username |
| `add_user()` | Create new user | username, volume, days, status, note, vless, vmess, shadowsocks |
| `delete_user()` | Remove user | username |
| `reset_user_traffic()` | Reset user data usage | username |
| `revoke_user_sub()` | Revoke subscription | username |
| `edit_user()` | Modify user settings | username, update_data |

## Error Handling

All methods return a `DVHOSTResponse` with:
- `status`: HTTP status code
- `data`: Response payload

## 🙏 Support with Crypto 
**We don't need financial support, only Star (⭐) is enough, thank you.**
- USDT (TRC20): `TVUqVMoCEe5DVUoxmPg8MwmgcHvZLqLjr4`

## 📧 Join Telegram Channel

TG : https://t.me/+EpErnDsDPhw3ZThk
# Pokemon API Proxy

A high-performance Rust web service that acts as a **transparent proxy** for the [PokéAPI](https://pokeapi.co/) with built-in caching and improved error handling. Built with Axum framework for blazing-fast async performance.

## 🚀 Features

- **Transparent Proxy**: Proxies **any** PokéAPI endpoint without modification
- **Universal Wildcard Routing**: Supports all PokéAPI endpoints (`/pokemon/*`, `/type/*`, `/pokemon-species/*`, etc.)
- **Raw JSON Caching**: Stores response data as-is without deserialization for maximum performance
- **In-Memory Caching**: Reduces API calls and improves response times dramatically
- **Random Pokemon Endpoint**: Get a random Pokemon (ID 1-1025)
- **Robust Error Handling**: Comprehensive error handling with detailed logging
- **Structured Logging**: Uses `tracing` for observability and debugging
- **Configuration-Driven**: TOML-based configuration
- **Thread-Safe**: Built for concurrent request handling

## 🛠️ Tech Stack

- **Framework**: [Axum](https://github.com/tokio-rs/axum) - Ergonomic async web framework
- **Runtime**: [Tokio](https://tokio.rs/) - Async runtime for Rust
- **HTTP Client**: [Reqwest](https://github.com/seanmonstar/reqwest) - Easy HTTP client
- **Serialization**: [Serde](https://serde.rs/) - JSON/TOML serialization
- **Logging**: [Tracing](https://tracing.rs/) - Structured logging
- **Config**: [TOML](https://github.com/toml-lang/toml) - Configuration format

## 📦 Installation

### Prerequisites

- Rust 1.75+ (2024 edition)
- Cargo

### Quick Start

1. **Clone the repository**
   ```bash
   git clone https://github.com/anwam/pokemon-api-proxy.git
   cd pokemon-api-proxy
   ```

2. **Build the project**
   ```bash
   cargo build --release
   ```

3. **Run the server**
   ```bash
   cargo run --release
   ```

The server will start on `http://0.0.0.0:3000` by default.

## 🎯 API Endpoints

### Get Pokemon by ID
```http
GET /pokemon/{id}
```

**Example:**
```bash
curl http://localhost:3000/pokemon/25
```

### Get Pokemon Species
```http
GET /pokemon-species/{id}
```

**Example:**
```bash
curl http://localhost:3000/pokemon-species/25
```

### Get Pokemon Types
```http
GET /type/{id}
```

**Example:**
```bash
curl http://localhost:3000/type/1
```

### Get Random Pokemon
```http
GET /random
```

**Example:**
```bash
curl http://localhost:3000/random
```

Returns a random Pokemon from the first 1025 Pokemon.

### Universal Proxy Support

The service supports **any** PokéAPI endpoint through wildcard routing:

```bash
curl http://localhost:3000/pokemon/25/encounters
curl http://localhost:3000/ability/1
curl http://localhost:3000/move/1
curl http://localhost:3000/item/1
curl http://localhost:3000/generation/1
curl http://localhost:3000/region/1
# ... and many more!
```

## ⚙️ Configuration

Configuration is managed through `config/config.toml`:

```toml
[pokemon]
api_url = "https://pokeapi.co/api/v2"
timeout = 30
cache_enabled = true

[cache]
type = "memory"
max_size = 1000
expiration = 3600
```

### Configuration Options

| Section | Key | Description | Default |
|---------|-----|-------------|---------|
| `pokemon` | `api_url` | PokéAPI base URL | `https://pokeapi.co/api/v2` |
| `pokemon` | `timeout` | Request timeout (seconds) | `30` |
| `pokemon` | `cache_enabled` | Enable/disable caching | `true` |
| `cache` | `type` | Cache type | `memory` |
| `cache` | `max_size` | Maximum cache entries | `1000` |
| `cache` | `expiration` | Cache expiration (seconds) | `3600` |

## 🔧 Development

### Running in Development Mode

```bash
# Run with debug logging
RUST_LOG=debug cargo run

# Run with automatic recompilation
cargo watch -x run
```

### Running Tests

```bash
cargo test
```

### Code Quality

```bash
# Check for issues
cargo check

# Run clippy for linting
cargo clippy -- -D warnings

# Format code
cargo fmt
```

## 📊 Performance

### Caching Benefits

- **Cache Hit**: ~0.1ms response time
- **Cache Miss**: ~100-300ms (depending on PokéAPI response time)
- **Memory Usage**: ~1KB per cached Pokemon

### Benchmarks

The proxy can handle thousands of concurrent requests efficiently thanks to:
- Async/await with Tokio runtime
- In-memory caching with `Arc<Mutex<HashMap>>`
- Zero-copy JSON serialization where possible

## 🐛 Error Handling

The service implements comprehensive error handling:

### Error Types

- **`ConfigError`**: Configuration parsing issues
- **`NetworkError`**: HTTP request failures
- **`CacheError`**: Cache operation failures
- **`ParseError`**: JSON parsing errors

### Error Responses

All errors return appropriate HTTP status codes:
- `500 Internal Server Error`: For upstream API failures
- Detailed logging for debugging

### Logging Levels

- **`DEBUG`**: Cache hits/misses, request flows
- **`INFO`**: Server startup events
- **`WARN`**: Non-critical failures (cache issues)
- **`ERROR`**: Critical errors affecting functionality

### Structured Logging

The service now supports **structured JSON logging** for enhanced observability and debugging. Logs are emitted in JSON format, making it easier to integrate with log aggregation tools and monitor service behavior.

### Production Build

```bash
# Optimized release build
cargo build --release

# The binary will be in target/release/pokemon-api-proxy
```

## 🤝 Contributing

1. Fork the repository
2. Create a feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes (`git commit -m 'Add amazing feature'`)
4. Push to the branch (`git push origin feature/amazing-feature`)
5. Open a Pull Request

### Development Guidelines

- Follow Rust naming conventions
- Add tests for new features
- Update documentation for API changes
- Use `cargo fmt` and `cargo clippy`

## 📝 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## 🙏 Acknowledgments

- [PokéAPI](https://pokeapi.co/) for providing the Pokemon data
- [Tokio](https://tokio.rs/) and [Axum](https://github.com/tokio-rs/axum) teams for excellent async tools
- Rust community for the amazing ecosystem

## 🔗 Related Projects

- [PokéAPI](https://github.com/PokeAPI/pokeapi) - The original Pokemon API
- [Axum Examples](https://github.com/tokio-rs/axum/tree/main/examples) - More Axum patterns
- [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/) - Best practices

---

**Built with ❤️ and ⚡ by the Rust community**

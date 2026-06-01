# srvcs-inrange

## Name

| Field | Value |
| --- | --- |
| Service | `srvcs-inrange` |
| Slug | `inrange` |
| Repository | `srvcs/inrange` |
| Package | `srvcs-inrange` |
| Kind | `orchestrator` |

## Function

range: is value within [lo, hi]

## Dependencies

| Dependency | Repository |
| --- | --- |
| `srvcs-between` | [srvcs/between](https://github.com/srvcs/between) |

## API

| Method | Path | Purpose |
| --- | --- | --- |
| `GET` | `/` | Service identity |
| `POST` | `/` | Evaluate the service function |
| `GET` | `/healthz` | Liveness probe |
| `GET` | `/readyz` | Readiness probe |
| `GET` | `/metrics` | Prometheus metrics |
| `GET` | `/openapi.json` | OpenAPI document |

## Inputs

| Name | Type | Required |
| --- | --- | --- |
| `value` | `json` | yes |
| `lo` | `json` | yes |
| `hi` | `json` | yes |

## Outputs

| Name | Type |
| --- | --- |
| `value` | `json` |
| `lo` | `json` |
| `hi` | `json` |
| `result` | `boolean` |

## Configuration

| Variable | Default | Purpose |
| --- | --- | --- |
| `SRVCS_BIND_ADDR` | `0.0.0.0:8080` | Bind address |
| `SRVCS_ENV` | `development` | Environment label for logs |
| `RUST_LOG` | `info,tower_http=info` | Tracing filter |
| `SRVCS_BETWEEN_URL` | `http://127.0.0.1:8087` | Base URL for srvcs-between |

## Error Behavior

- `422` means the request could not be evaluated for the documented input shape.
- `503` means a required dependency was unavailable or returned an unexpected response.
- Dependency validation errors are forwarded when this service delegates validation.

## Local Checks

```sh
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
```

See the [srvcs service standard](https://github.com/srvcs/platform/blob/main/STANDARD.md) for the full operational contract.

## Metadata

Machine-readable service metadata lives in `srvcs.yaml`. Keep it aligned with this README when the service contract changes.

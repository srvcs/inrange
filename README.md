# srvcs-inrange

The range orchestrator of the srvcs.cloud distributed standard library.

Its single concern: **range: is value within [lo, hi].** It is a thin
orchestrator over [`srvcs-between`](https://github.com/srvcs/between): it owns
the *control flow* but does no comparison of its own. It forwards
`{"value", "lo", "hi"}` to `srvcs-between` and returns its boolean `result`.

```
inrange(value, lo, hi):
    return between(value, lo, hi).result
```

The result is a boolean, e.g. `inrange(5, 0, 10) == true` and
`inrange(15, 0, 10) == false`.

Validation is not handled here. This service never calls `srvcs-isnumber`
directly; instead `srvcs-between` (and its own dependencies) validate the
operands, and any `422` raised is forwarded verbatim.

## API

| Method | Path | Purpose |
| --- | --- | --- |
| `GET` | `/` | Service identity, concern, and dependency list |
| `POST` | `/` | Decide whether `value` is within `[lo, hi]` |
| `GET` | `/healthz` `/readyz` `/metrics` `/openapi.json` | srvcs service standard surface |

```sh
curl -s -X POST localhost:8080/ -H 'content-type: application/json' -d '{"value": 5, "lo": 0, "hi": 10}'
# {"value":5,"lo":0,"hi":10,"result":true}
```

Responses:

- `200 {"value": value, "lo": lo, "hi": hi, "result": r}` — evaluated; `result`
  is a boolean.
- `422` — `srvcs-between` rejected the input, forwarded verbatim.
- `500` — a reachable dependency returned a `200` without a usable result.
- `503` — `srvcs-between` is unavailable.

## Dependencies

- [`srvcs-between`](https://github.com/srvcs/between)

## Configuration

| Variable | Default | Purpose |
| --- | --- | --- |
| `SRVCS_BIND_ADDR` | `0.0.0.0:8080` | Bind address |
| `SRVCS_BETWEEN_URL` | `http://127.0.0.1:8087` | Base URL of `srvcs-between` |
| `SRVCS_ENV` | `development` | Environment label for logs |
| `RUST_LOG` | `info,tower_http=info` | Tracing filter |

## Local checks

```sh
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
```

Orchestration tests stand up a *computing* mock `srvcs-between` service
in-process — it reads the request body and returns the real
`lo <= value <= hi`, so the composition is genuinely exercised against the
asserted cases. See [`srvcs/platform`](https://github.com/srvcs/platform) for the
shared standard.

> Note: the `cargoHash` in `flake.nix` is inherited from the template and must be
> refreshed with a `nix build` before the Nix gates pass.

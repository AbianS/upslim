---
title: Conditions DSL
description: Complete reference for the UpSlim condition expression language.
---

# Conditions DSL

Conditions are simple expressions evaluated after each check. All conditions in a monitor must pass for the check to be considered successful.

## Syntax

```
[VARIABLE] OPERATOR value
```

- **Variable** — a bracketed keyword identifying what to check
- **Operator** — `==`, `!=`, `<`, `>`, `<=`, `>=`
- **Value** — a string, number, or boolean

## Variables

### `[STATUS]`

HTTP status code as an integer. Only available on HTTP monitors.

```yaml
conditions:
  - "[STATUS] == 200"
  - "[STATUS] < 400"
  - "[STATUS] != 503"
```

### `[RESPONSE_TIME]`

Time in milliseconds from sending the request to receiving the response.

Available on both HTTP and TCP monitors.

```yaml
conditions:
  - "[RESPONSE_TIME] < 500"
  - "[RESPONSE_TIME] < 2000"
```

### `[BODY]`

Raw response body as a string (HTTP monitors only).

```yaml
conditions:
  - "[BODY] == ok"
```

### `[BODY].path.to.field`

JSON dot-path into the response body. UpSlim parses the body as JSON and extracts the value at the given path.

```yaml
conditions:
  - "[BODY].status == healthy"
  - "[BODY].db.connected == true"
  - "[BODY].version == 1.0.0"
```

::: tip
The response body is only read if at least one condition references `[BODY]`. This avoids unnecessary memory allocation for large responses.
:::

### `[CONNECTED]`

Whether the TCP connection was established successfully. Only available on TCP monitors.

```yaml
conditions:
  - "[CONNECTED] == true"
```

## Operators

| Operator | Description |
|----------|-------------|
| `==` | Equals |
| `!=` | Not equals |
| `<` | Less than |
| `>` | Greater than |
| `<=` | Less than or equal |
| `>=` | Greater than or equal |

Comparison operators (`<`, `>`, `<=`, `>=`) work on numbers. Equality operators work on strings, numbers, and booleans.

## Multiple conditions

All conditions must pass. If any condition fails, the check is marked as failed and the failure reason reports which condition failed.

```yaml
conditions:
  - "[STATUS] == 200"         # must pass
  - "[RESPONSE_TIME] < 500"  # must pass
  - "[BODY].ok == true"       # must pass
```

Failure log example:

```
WARN  Check FAILED monitor=api response_time_ms=612 reason=Some("[RESPONSE_TIME] < 500: got '612'")
```

## Examples

### HTTP health endpoint returning JSON

```yaml
- name: api
  type: http
  url: "https://api.example.com/health"
  conditions:
    - "[STATUS] == 200"
    - "[RESPONSE_TIME] < 1000"
    - "[BODY].status == healthy"
```

### HTTP with auth header

```yaml
- name: internal-api
  type: http
  url: "https://internal.example.com/status"
  headers:
    Authorization: "Bearer ${API_TOKEN}"
  conditions:
    - "[STATUS] == 200"
    - "[BODY].ready == true"
```

### TCP database check

```yaml
- name: postgres
  type: tcp
  host: "db.internal"
  port: 5432
  timeout: 3s
  conditions:
    - "[CONNECTED] == true"
    - "[RESPONSE_TIME] < 100"
```

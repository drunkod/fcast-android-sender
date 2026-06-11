# STEP-1C — Serde wire contract + JSON examples

> Documentation sub-step (no code edit). Pins down the exact JSON the
> `Srt` variant accepts and emits, so STEP-7's UI and the Rust command
> sender agree on the shape.

---

## Serde representation

`DestinationFamily` has **no** `#[serde(rename_all = …)]`, so it uses serde's
default **externally-tagged** representation with PascalCase variant names.
The tag is the variant name, the payload is the struct fields:

```json
{ "Srt": { "uri": "…", "latency": 200, "passphrase": "…", "pbkeylen": 16 } }
```

This matches the existing `Whep` shape (`{"Whep":{"server_port":0}}`), so the
SRT variant needs no special handling — the existing `whep_destination_serdes_roundtrip`
test pattern carries over directly (see [STEP-1D](STEP-1D-verification-pitfalls.md)).

---

## Full command examples

These are the `CreateDestination` commands the migration runtime accepts. The
`createdestination` wrapper, `id`, `audio`, and `video` fields are unchanged
from the existing RTMP/UDP/WHEP commands.

### Plain transport (latency defaults to 200 ms)

```json
{
  "createdestination": {
    "id": "srt-out-1",
    "family": {
      "Srt": { "uri": "srt://10.0.0.42:9000" }
    },
    "audio": false,
    "video": true
  }
}
```

### With AES-128 encryption

```json
{
  "createdestination": {
    "id": "srt-enc",
    "family": {
      "Srt": {
        "uri": "srt://media.example.com:1234",
        "latency": 500,
        "passphrase": "supersecretphrase",
        "pbkeylen": 16
      }
    },
    "audio": true,
    "video": true
  }
}
```

### Listener mode (phone accepts an inbound connection)

```json
{
  "createdestination": {
    "id": "srt-listener",
    "family": {
      "Srt": { "uri": "srt://0.0.0.0:9000?mode=listener", "latency": 200 }
    },
    "audio": true,
    "video": true
  }
}
```

### IPv6 (brackets required)

```json
{
  "createdestination": {
    "id": "srt-v6",
    "family": {
      "Srt": { "uri": "srt://[fe80::1]:1234", "latency": 200 }
    },
    "audio": false,
    "video": true
  }
}
```

---

## Round-trip invariants

| Input | Serialised output |
|---|---|
| `latency` omitted | deserialises to `200` |
| `passphrase` / `pbkeylen` = `None` | **omitted** from serialised JSON (no `null`) |
| `passphrase` / `pbkeylen` = `Some(...)` | emitted as normal fields |
| any variant | re-serialises to the same `{"Srt":{…}}` shape it parsed from |

These four invariants are exactly what the STEP-4 unit tests assert.

---

## Contract with the UI (STEP-5 / STEP-7)

The encryption `pbkeylen` is chosen in the UI as an **index**, then mapped to
the byte count this JSON expects by the Rust `start-srt-destination` handler:

| UI idx (`Bridge.srt-destination-pbkeylen-idx`) | Label | JSON `pbkeylen` |
|---|---|---|
| 0 | None | field omitted (+ omit `passphrase`) |
| 1 | AES-128 | `16` |
| 2 | AES-192 | `24` |
| 3 | AES-256 | `32` |

See [../step-7/INDEX.md](../step-7/INDEX.md) for the
slintcn UI and [../step-5/INDEX.md](../step-5/INDEX.md)
for the Bridge property.

---

## Next

→ [STEP-1D-verification-pitfalls.md](STEP-1D-verification-pitfalls.md)

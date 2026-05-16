# Tantivy CLI

A command-line interface and HTTP server for the [Tantivy](https://github.com/quickwit-oss/tantivy) search engine, built with **Tokio** and **Axum**.

Supports dynamic index/schema creation, full CRUD for documents, **Elasticsearch-style Query DSL**, field sorting, aggregations, highlighted search, document expiration, and index maintenance operations.

---

## Features

- **Dynamic Indexes & Schemas** — Create indexes at runtime with a JSON schema definition.
- **Document CRUD** — Add, get, list, and delete documents via CLI or HTTP.
- **Elasticsearch-style Query DSL** — `bool`, `match`, `term`, `range`, `query_string` queries with nested composition.
- **Field Sorting** — Sort results by any `fast` field (ascending/descending).
- **Aggregations** — Native ES-compatible aggregations (`terms`, `stats`, `histogram`, etc.) powered by Tantivy.
- **Document Expiration** — Automatic cleanup of documents with an `expired_at` date field.
- **Highlighted Search** — Full-text search with configurable snippet highlighting.
- **Index Maintenance** — Stats, rebuild (merge segments), and compress operations.
- **Dual Interface** — Use as a traditional CLI tool or run as an async HTTP server.
- **Async I/O** — Powered by Tokio; all index operations are non-blocking.
- **Auto Commit** — Server automatically commits pending writes every 30 seconds.

---

## Quick Start

### Build

```bash
cargo build --release
```

The binary is located at `./target/release/tantivy-cli`.

---

## CLI Usage

### Global Options

```bash
tantivy-cli --index-dir /var/lib/tantivy <command>
```

| Option | Default | Description |
|--------|---------|-------------|
| `--index-dir` | `./indexes` | Base directory for all indexes |

### Index Management

#### Create an index

Create a file named `schema.json`:

```json
{
  "fields": [
    { "name": "id",         "kind": "string", "stored": true, "indexed": true, "fast": true },
    { "name": "title",      "kind": "text",   "stored": true, "indexed": true, "fast": false },
    { "name": "body",       "kind": "text",   "stored": true, "indexed": true, "fast": false },
    { "name": "status",     "kind": "string", "stored": true, "indexed": true, "fast": true },
    { "name": "category",   "kind": "string", "stored": true, "indexed": true, "fast": true },
    { "name": "price",      "kind": "u64",    "stored": true, "indexed": true, "fast": true },
    { "name": "created_at", "kind": "date",   "stored": true, "indexed": true, "fast": true },
    { "name": "expired_at", "kind": "date",   "stored": true, "indexed": true, "fast": true }
  ]
}
```

```bash
tantivy-cli create-index articles --schema-file schema.json
```

#### List indexes

```bash
tantivy-cli list-indexes
```

#### Delete an index

```bash
tantivy-cli delete-index articles
```

### Document Operations

#### Add a document

```bash
tantivy-cli add-doc articles --doc '{"id":"1","title":"Hello Tantivy","body":"A fast full-text search engine.","price":10}'

tantivy-cli add-doc articles --doc '{"id":"2","title":"Rust Programming","body":"Rust is a systems language.","price":20}'
```

#### Get a document

```bash
tantivy-cli get-doc articles id 1
```

#### List documents

```bash
tantivy-cli list-docs articles --limit 10 --offset 0
```

> **Note:** `offset + limit` cannot exceed `10,000`.

#### Delete documents

```bash
tantivy-cli delete-doc articles id 1
```

### Search

```bash
tantivy-cli search articles "tantivy" --limit 5
```

Search with **highlighted snippets**:

```bash
tantivy-cli search articles "systems language" --highlight title --highlight body
```

### Index Maintenance

```bash
# Show stats
tantivy-cli stats articles

# Rebuild (merge segments)
tantivy-cli rebuild articles

# Compress (commit)
tantivy-cli compress articles
```

---

## HTTP Server

Start the server:

```bash
tantivy-cli serve --bind 127.0.0.1:3000
```

All endpoints return JSON. Errors are returned with appropriate HTTP status codes and an `{ "error": "..." }` body.

### Index Management

#### Create an Index

```bash
curl -X POST http://127.0.0.1:3000/indexes/articles \
  -H "Content-Type: application/json" \
  -d '{
    "schema": {
      "fields": [
        { "name": "id",    "kind": "string", "stored": true, "indexed": true, "fast": true },
        { "name": "title", "kind": "text",   "stored": true, "indexed": true, "fast": false },
        { "name": "body",  "kind": "text",   "stored": true, "indexed": true, "fast": false }
      ]
    }
  }'
```

**Response:** `201 Created`

```json
{ "index": "articles" }
```

#### List Indexes

```bash
curl http://127.0.0.1:3000/indexes
```

**Response:**

```json
["articles"]
```

#### Get Index Info

```bash
curl http://127.0.0.1:3000/indexes/articles
```

**Response:**

```json
{
  "name": "articles",
  "num_docs": 2,
  "schema": [ /* ... */ ]
}
```

#### Delete an Index

```bash
curl -X DELETE http://127.0.0.1:3000/indexes/articles
```

**Response:** `204 No Content`

### Document Operations

#### Add a Document

```bash
curl -X POST http://127.0.0.1:3000/indexes/articles/docs \
  -H "Content-Type: application/json" \
  -d '{"id":"1","title":"Hello","body":"World"}'
```

**Response:**

```json
{ "id": "1" }
```

#### List Documents

```bash
curl "http://127.0.0.1:3000/indexes/articles/docs?limit=10&offset=0"
```

> **Note:** `offset + limit` cannot exceed `10,000`.

#### Get a Document

```bash
curl http://127.0.0.1:3000/indexes/articles/docs/id/1
```

#### Delete a Document

```bash
curl -X DELETE http://127.0.0.1:3000/indexes/articles/docs/id/1
```

**Response:**

```json
{ "deleted": 1 }
```

### Search

The search endpoint supports an **Elasticsearch-style Query DSL** via POST, as well as a simple GET interface.

#### GET (simple query string)

```bash
curl "http://127.0.0.1:3000/indexes/articles/search?q=hello&limit=5&offset=0"
```

With highlighting:

```bash
curl "http://127.0.0.1:3000/indexes/articles/search?q=hello&highlight=title&highlight=body&snippet_max_chars=150"
```

#### POST (ES-style JSON DSL)

##### 1. Simple full-text query (`query_string`)

```bash
curl -X POST http://127.0.0.1:3000/indexes/articles/search \
  -H "Content-Type: application/json" \
  -d '{
    "query": { "query_string": { "query": "hello world" } },
    "size": 10,
    "from": 0
  }'
```

##### 2. `match` query (single field)

```bash
curl -X POST http://127.0.0.1:3000/indexes/articles/search \
  -H "Content-Type: application/json" \
  -d '{
    "query": { "match": { "title": "hello world" } }
  }'
```

##### 3. `term` query (exact match)

```bash
curl -X POST http://127.0.0.1:3000/indexes/articles/search \
  -H "Content-Type: application/json" \
  -d '{
    "query": { "term": { "status": "active" } }
  }'
```

##### 4. `range` query

```bash
curl -X POST http://127.0.0.1:3000/indexes/articles/search \
  -H "Content-Type: application/json" \
  -d '{
    "query": {
      "range": {
        "price": { "gte": 100, "lte": 500 }
      }
    }
  }'
```

##### 5. `bool` query (nested composition)

```bash
curl -X POST http://127.0.0.1:3000/indexes/articles/search \
  -H "Content-Type: application/json" \
  -d '{
    "query": {
      "bool": {
        "must": [
          { "match": { "title": "rust" } }
        ],
        "filter": [
          { "term": { "status": "active" } },
          { "range": { "price": { "gte": 50 } } }
        ],
        "must_not": [
          { "term": { "status": "deleted" } }
        ]
      }
    }
  }'
```

##### 6. Sort by fast field

Sort fields **must** be declared with `"fast": true` in the schema.

```bash
curl -X POST http://127.0.0.1:3000/indexes/articles/search \
  -H "Content-Type: application/json" \
  -d '{
    "query": { "query_string": { "query": "rust" } },
    "sort": [{ "price": "desc" }]
  }'
```

##### 7. Field filtering (`_source`)

```bash
curl -X POST http://127.0.0.1:3000/indexes/articles/search \
  -H "Content-Type: application/json" \
  -d '{
    "query": { "query_string": { "query": "rust" } },
    "_source": ["id", "title", "price"]
  }'
```

##### 8. Aggregations

Aggregations use **Tantivy's native ES-compatible syntax** and work only on `fast` fields.

```bash
curl -X POST http://127.0.0.1:3000/indexes/articles/search \
  -H "Content-Type: application/json" \
  -d '{
    "query": { "query_string": { "query": "rust" } },
    "aggs": {
      "by_status": { "terms": { "field": "status" } },
      "price_stats": { "stats": { "field": "price" } }
    }
  }'
```

**Aggregation Response:**

```json
{
  "total": 42,
  "hits": [ /* ... */ ],
  "aggregations": {
    "by_status": {
      "buckets": [
        { "key": "active", "doc_count": 30 },
        { "key": "draft", "doc_count": 12 }
      ]
    },
    "price_stats": {
      "count": 42,
      "min": 10,
      "max": 999,
      "avg": 250.5,
      "sum": 10521
    }
  }
}
```

##### 9. Highlighting snippets

```bash
curl -X POST http://127.0.0.1:3000/indexes/articles/search \
  -H "Content-Type: application/json" \
  -d '{
    "query": { "query_string": { "query": "rust" } },
    "highlight_fields": ["title", "body"],
    "snippet_max_chars": 150
  }'
```

**Response:**

```json
{
  "total": 1,
  "hits": [
    {
      "doc": {
        "id": "1",
        "title": "Hello",
        "body": "World"
      },
      "score": 1.23,
      "snippets": {
        "body": "<b>Hello</b> <b>World</b>"
      }
    }
  ],
  "limit": 10,
  "offset": 0
}
```

> **Note:** The POST body also accepts the legacy field names `limit` and `offset` as aliases for `size` and `from`.

### Index Maintenance

#### Index Stats

```bash
curl http://127.0.0.1:3000/indexes/articles/stats
```

**Response:**

```json
{
  "num_docs": 2,
  "num_segments": 1,
  "schema": [ /* ... */ ]
}
```

#### Rebuild Index

```bash
curl -X POST http://127.0.0.1:3000/indexes/articles/rebuild
```

**Response:**

```json
{ "status": "rebuilt" }
```

#### Compress Index

```bash
curl -X POST http://127.0.0.1:3000/indexes/articles/compress
```

**Response:**

```json
{ "status": "compressed" }
```

---

## Schema Field Types

The following field kinds are supported in schema definitions:

| Kind | Description |
|------|-------------|
| `text` | Full-text analyzed text (tokenized, with positions) |
| `string` | Raw string (not tokenized, useful for IDs) |
| `u64` | Unsigned 64-bit integer |
| `i64` | Signed 64-bit integer |
| `f64` | 64-bit floating point |
| `bool` | Boolean value |
| `date` | Date/time (parse from ISO-8601 string) |
| `facet` | Hierarchical category (e.g. `/category/sub`) |
| `bytes` | Base64-encoded binary data |
| `json` | Arbitrary JSON object |

Each field accepts three boolean flags:

- `stored` — Retrieve the field in search results.
- `indexed` — Make the field searchable/sortable.
- `fast` — Enable fast field access (efficient sorting/aggregation).

---

## Document Expiration

If a schema contains an `expired_at` field with type `date`, the server will automatically clean up expired documents in the background (checked every 60 seconds).

Example document with expiration:

```json
{
  "id": "1",
  "title": "Temporary Article",
  "expired_at": "2026-05-10T12:00:00Z"
}
```

If `expired_at` is absent or the schema does not define it, documents never expire (default behavior).

---

## Configuration

The default index storage directory is `./indexes`. You can override it with:

```bash
tantivy-cli --index-dir /var/lib/tantivy <command>
```

---

## Tech Stack

- [Tantivy 0.26](https://docs.rs/tantivy/0.26.1/tantivy/) — Full-text search engine
- [Tokio](https://tokio.rs/) — Async runtime
- [Axum](https://github.com/tokio-rs/axum) — HTTP web framework
- [Clap](https://github.com/clap-rs/clap) — CLI parser
- [Flowbite](./llms.txt) - Flowbite UI

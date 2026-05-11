# Tantivy CLI

A command-line interface and HTTP server for the [Tantivy](https://github.com/quickwit-oss/tantivy) search engine, built with **Tokio** and **Axum**.

Supports dynamic index/schema creation, full CRUD for documents, highlighted search, and index maintenance operations.

---

## Features

- **Dynamic Indexes & Schemas** — Create indexes at runtime with a JSON schema definition.
- **Document CRUD** — Add, get, list, and delete documents via CLI or HTTP.
- **Highlighted Search** — Full-text search with configurable snippet highlighting.
- **Index Maintenance** — Stats, rebuild (merge segments), and compress operations.
- **Dual Interface** — Use as a traditional CLI tool or run as an async HTTP server.
- **Async I/O** — Powered by Tokio; all index operations are non-blocking.

---

## Quick Start

### Build

```bash
cargo build --release
```

The binary is located at `./target/release/tantivy-cli`.

### CLI Usage

#### 1. Create an index with a schema

Create a file named `schema.json`:

```json
{
  "fields": [
    { "name": "id",    "kind": "string", "stored": true, "indexed": true, "fast": true },
    { "name": "title", "kind": "text",   "stored": true, "indexed": false, "fast": false },
    { "name": "body",  "kind": "text",   "stored": true, "indexed": false, "fast": false },
    { "name": "score", "kind": "u64",    "stored": true, "indexed": true,  "fast": true }
  ]
}
```

Then create the index:

```bash
tantivy-cli create-index articles --schema-file schema.json
```

#### 2. Add documents

```bash
tantivy-cli add-doc articles --doc '{"id":"1","title":"Hello Tantivy","body":"A fast full-text search engine.","score":10}'

tantivy-cli add-doc articles --doc '{"id":"2","title":"Rust Programming","body":"Rust is a systems language.","score":20}'
```

#### 3. Search

```bash
tantivy-cli search articles "tantivy" --limit 5
```

Search with **highlighted snippets**:

```bash
tantivy-cli search articles "systems language" --highlight title --highlight body
```

#### 4. Get a document

```bash
tantivy-cli get-doc articles id 1
```

#### 5. Delete a document

```bash
tantivy-cli delete-doc articles id 1
```

#### 6. Index maintenance

```bash
# Show stats
tantivy-cli stats articles

# Rebuild (merge segments)
tantivy-cli rebuild articles

# Compress (commit)
tantivy-cli compress articles
```

#### 7. List indexes

```bash
tantivy-cli list-indexes
```

---

## HTTP Server

Start the server:

```bash
tantivy-cli serve --bind 127.0.0.1:3000
```

### HTTP API Reference

All endpoints return JSON. Errors are returned with appropriate HTTP status codes and an `{ "error": "..." }` body.

#### Create an Index

```bash
curl -X POST http://127.0.0.1:3000/indexes/articles \
  -H "Content-Type: application/json" \
  -d '{
    "schema": {
      "fields": [
        { "name": "id",    "kind": "string", "stored": true, "indexed": true, "fast": true },
        { "name": "title", "kind": "text",   "stored": true, "indexed": false, "fast": false },
        { "name": "body",  "kind": "text",   "stored": true, "indexed": false, "fast": false }
      ]
    }
  }'
```

**Response:**

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

#### Search

**GET (query string):**

```bash
curl "http://127.0.0.1:3000/indexes/articles/search?q=hello&limit=5&offset=0"
```

**POST (JSON body):**

```bash
curl -X POST http://127.0.0.1:3000/indexes/articles/search \
  -H "Content-Type: application/json" \
  -d '{
    "query": "hello world",
    "limit": 10,
    "offset": 0,
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
  "query": "hello world"
}
```

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

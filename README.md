# image-indexer-demo

A small Rust/Actix-web application that demonstrates an asynchronous image intake and thumbnail generation pipeline.

The application accepts image uploads, stores them as jobs, processes them in the background, and tracks status, errors, attempts, and retry timing.

The scope is intentionally small: one feature, implemented clearly.

## What it does

The application provides a simple image processing queue:

1. A client uploads an image through `POST /images`.
2. The uploaded file is stored locally.
3. A persisted image job is created with status `pending`.
4. A background worker claims pending jobs.
5. The worker generates a 64x64 thumbnail.
6. The job is marked as `done` or `failed`.
7. Failed jobs are retried using a small backoff policy.

## Features

- `POST /images` accepts a multipart image upload
- uploaded files are stored under `storage/originals/`
- each upload creates a persisted image job
- background worker for asynchronous processing
- explicit job lifecycle: `pending`, `processing`, `done`, `failed`
- retry/backoff support for failed jobs
- `GET /images/{id}` returns job status and metadata
- `GET /health` returns service health
- SQLite persistence
- tests for domain transitions, retry behavior, repository behavior, and thumbnail processing


## Tech stack

- Rust
- Actix-web
- Tokio
- SQLite
- SQLx
- Serde
- image crate
- tracing

## Prerequisites

- Rust stable
- Cargo

Install Rust using `rustup`:

```text
https://rustup.rs
```

After installation, verify that Rust and Cargo are available:

```bash
rustc --version
cargo --version
```

No external database server is required. The application uses SQLite, and the database file is created automatically when the application starts.

## Configuration

Optionally, a local `.env` file can be created from the example:

```bash
cp .env.example .env
```

Example configuration:

```env
HOST=127.0.0.1
PORT=8080
DATABASE_URL=sqlite://image-indexer-demo.db
STORAGE_ROOT=storage
MAX_ATTEMPTS=3
MAX_UPLOAD_BYTES=10485760
RUST_LOG=image_indexer_demo=info,actix_web=info
```

## Run locally

```bash
cargo run
```

The application starts an HTTP server and a background worker.

By default, the API is available at:

```text
http://127.0.0.1:8080
```

## API

### Health check

```bash
curl http://127.0.0.1:8080/health
```

Example response:

```json
{
  "status": "ok",
  "service": "image-indexer-demo"
}
```

### Upload an image

Linux/macOS:

```bash
curl -X POST http://127.0.0.1:8080/images \
  -F "image=@/path/to/image.jpg"
```

Windows PowerShell:

```powershell
curl.exe -X POST http://127.0.0.1:8080/images `
  -F "image=@C:\Users\you\Downloads\image.jpg"
```

Example response:

```json
{
  "id": "33ec0cc1-9147-4129-a2df-4f4afa1eb497",
  "original_filename": "image.jpg",
  "stored_path": "storage\\originals\\33ec0cc1-9147-4129-a2df-4f4afa1eb497.jpg",
  "thumbnail_path": null,
  "status": "pending",
  "attempts": 0,
  "max_attempts": 3,
  "last_error": null,
  "next_retry_at": null,
  "created_at": "2026-05-11T08:22:38.364570100Z",
  "updated_at": "2026-05-11T08:22:38.364570100Z",
  "indexed_at": null
}
```

The response may initially show `pending`. The background worker should pick it up shortly after.

### Get image job status

```bash
curl http://127.0.0.1:8080/images/<id>
```

Example completed job:

```json
{
  "id": "33ec0cc1-9147-4129-a2df-4f4afa1eb497",
  "original_filename": "image.jpg",
  "stored_path": "storage\\originals\\33ec0cc1-9147-4129-a2df-4f4afa1eb497.jpg",
  "thumbnail_path": "storage\\thumbnails\\33ec0cc1-9147-4129-a2df-4f4afa1eb497.png",
  "status": "done",
  "attempts": 0,
  "max_attempts": 3,
  "last_error": null,
  "next_retry_at": null,
  "created_at": "2026-05-11T08:22:38.364570100Z",
  "updated_at": "2026-05-11T08:22:39.101570100Z",
  "indexed_at": "2026-05-11T08:22:39.101570100Z"
}
```


## Status lifecycle

Image jobs move through a small explicit lifecycle.

```text
pending
   ↓
processing
   ↓
done
```

Failure path:

```text
pending
   ↓
processing
   ↓
failed
   ↓ if retryable
processing
```

A failed job is retryable when:

```text
attempts < max_attempts
and next_retry_at is null or in the past
```

When a job reaches `max_attempts`, `next_retry_at` is set to `null` and the worker will no longer retry it.

## Retry and backoff

Failed jobs are retried using a small backoff policy.

Default behavior with `MAX_ATTEMPTS=3`:

| Failed attempt | Next retry |
| --- | --- |
| 1 | 10 seconds |
| 2 | 30 seconds |
| 3 | no retry |

The full backoff table is:

| Failed attempt | Backoff |
| --- | --- |
| 1 | 10 seconds |
| 2 | 30 seconds |
| 3 | 120 seconds |
| 4 | 600 seconds |
| 5+ | 1800 seconds |

When a job reaches `max_attempts`, `next_retry_at` becomes `null`.

## Example failed job

For this demo, the intake endpoint keeps validation minimal. If a non-image file is uploaded, the worker fails during thumbnail generation and records the failure on the job.

Example:

```powershell
curl.exe -X POST http://127.0.0.1:8080/images `
  -F "image=@C:\Users\you\Downloads\not-an-image.txt"
```

The worker will later mark the job as failed:

```json
{
  "status": "failed",
  "attempts": 1,
  "max_attempts": 3,
  "last_error": "failed to read image: ...",
  "next_retry_at": "2026-05-11T08:40:00.602076600Z"
}
```

This behavior keeps the upload endpoint simple and demonstrates the retry/failure pipeline clearly.

## Project structure

```text
src/
├─ app_state.rs
├─ config.rs
├─ error.rs
├─ main.rs
├─ domain/
│  ├─ mod.rs
│  ├─ image_job.rs
│  └─ status.rs
├─ processing/
│  ├─ mod.rs
│  ├─ image_processor.rs
│  └─ thumbnail_processor.rs
├─ repository/
│  ├─ mod.rs
│  └─ sqlite_image_job_repository.rs
├─ routes/
│  ├─ mod.rs
│  ├─ health.rs
│  └─ images.rs
├─ storage/
│  ├─ mod.rs
│  └─ file_storage.rs
└─ worker/
   ├─ mod.rs
   ├─ image_indexing_worker.rs
   └─ retry.rs
```

## Design decisions

### Small scope

The project focuses on one working feature: uploaded images are processed into thumbnails through a persisted background job pipeline.

I chose to keep the scope small so the code remains easy to review, run, and explain.

### SQLite persistence

SQLite keeps the project simple to run locally while still providing real persistence.

No separate database server or Docker setup is required.

### Repository abstraction

Persistence is hidden behind an `ImageJobRepository` trait.

This keeps database-specific code separate from the rest of the application.

### Processor abstraction

Image processing is hidden behind an `ImageProcessor` trait.

The current implementation generates 64x64 thumbnails, but the worker only depends on the processing abstraction.

### Explicit status transitions

The `ImageJob` domain model owns transitions such as `mark_processing`, `mark_done`, `mark_failed`, and `can_retry`.

This keeps lifecycle rules close to the data they modify.

### Retry policy

Retry timing is handled by a dedicated `RetryPolicy`.

This keeps retry/backoff behavior testable and keeps the worker focused on orchestration.

## Testing

Run the full test suite:

```bash
cargo test
```

The tests cover:

- image job status transitions
- invalid lifecycle transitions
- retry eligibility
- retry/backoff timing
- SQLite repository behavior
- thumbnail generation

## AI usage

I used AI as a development assistant while building this project.

The AI was mainly used to:

- discuss project scope
- recommend approaches to structure
- generate first drafts of some code
- think through and help set up test cases
- help structure this README

I reviewed and adjusted AI-assisted code myself while building the project step by step. I can explain the structure and implementation.

## Possible improvements

This demo intentionally keeps the scope small. Natural next steps could include:

- upload content-type validation
- serving thumbnails through a static route
- structured JSON error responses
- integration tests around HTTP routes
- configurable thumbnail dimensions
- cleanup of old failed jobs and associated files

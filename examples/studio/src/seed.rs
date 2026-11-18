//! The workspace as it starts, as the web's `seed.ts`: each project's pipelines, schemas and
//! queries, a sample value for each schema, and what a new file of each kind starts with.

use crate::js::utc;
use crate::model::{FileKind, Project, WorkFile};

/// When the seeded files were last saved: 2026-11-09 15:30 UTC.
pub fn seeded_at() -> i64 {
    utc(2026, 10, 9, 15, 30, 0)
}

const NIGHTLY_EXPORT: &str = r##"name: nightly-export
description: Exports the day's orders to the harbor bucket.
project: harbor
schedule: "0 2 * * *"
params:
  bucket: harbor-exports
  since: 1d
steps:
  - id: fetch
    kind: fetch
    with:
      url: https://api.example.org/orders?since=${params.since}
      format: json
  - id: transform
    kind: transform
    needs: [fetch]
    with:
      language: jq
      script: '.orders | map({id, total, email: .customer.email})'
  - id: validate
    kind: validate
    needs: [transform]
    with:
      rules:
        - field: email
          check: email
        - field: total
          check: positive
      on_invalid: fail
      max_rejected: 10
  - id: upload
    kind: upload
    needs: [validate]
    retries: 2
    with:
      bucket: ${params.bucket}
      path: orders/${run.date}.json
  - id: notify
    kind: notify
    needs: [upload]
    with:
      channel: "#harbor"
      on: [failed]
      message: "nightly-export wrote ${steps.validate.rows} orders"
"##;

const IMPORT_ORDERS: &str = r##"name: import-orders
description: Takes in the order files dropped every half hour.
project: harbor
schedule: "*/30 * * * *"
steps:
  - id: fetch
    kind: fetch
    with:
      url: https://files.example.org/orders/latest.csv
      format: csv
  - id: validate
    kind: validate
    needs: [fetch]
    with:
      rules:
        - field: id
          check: unique
        - field: placed_at
          check: date
      on_invalid: skip
  - id: upload
    kind: upload
    needs: [validate]
    with:
      bucket: harbor-orders
      path: incoming/${run.id}.csv
"##;

const ORDER_SCHEMA: &str = r##"{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "$id": "https://schemas.example.org/harbor/order.json",
  "title": "Order",
  "description": "One order, as nightly-export writes it.",
  "type": "object",
  "required": ["id", "total", "email"],
  "additionalProperties": false,
  "properties": {
    "id": { "type": "string", "pattern": "^ord-[0-9]{6}$", "description": "The order's number." },
    "total": { "type": "number", "exclusiveMinimum": 0, "description": "What was paid, in euros." },
    "email": { "type": "string", "format": "email" },
    "placed_at": { "type": "string", "format": "date-time" },
    "lines": {
      "type": "array",
      "minItems": 1,
      "items": {
        "type": "object",
        "required": ["sku", "count"],
        "properties": {
          "sku": { "type": "string" },
          "count": { "type": "integer", "minimum": 1 }
        }
      }
    }
  }
}
"##;

const FAILED_RUNS: &str = r##"-- The failed steps of every run, newest first
SELECT r.id, r.pipeline, s.step, s.duration_ms, r.started_at
FROM runs r
JOIN steps s ON s.run_id = r.id
WHERE r.state = 'failed' AND s.state = 'failed'
ORDER BY r.started_at DESC
LIMIT 50
"##;

const SYNC_INVENTORY: &str = r##"name: sync-inventory
description: Copies the stock counts from the warehouse every hour.
project: juniper
schedule: "0 * * * *"
params:
  warehouse: north
steps:
  - id: fetch
    kind: fetch
    with:
      url: https://warehouse.example.org/${params.warehouse}/stock
      format: ndjson
      since: 1h
  - id: transform
    kind: transform
    needs: [fetch]
    with:
      language: sql
      script: SELECT sku, sum(count) AS count FROM rows GROUP BY sku
  - id: upload
    kind: upload
    needs: [transform]
    with:
      bucket: juniper-stock
      path: stock/${run.date}.ndjson
      overwrite: true
"##;

const REBUILD_INDEX: &str = r##"name: rebuild-index
description: Rebuilds the search index from the catalog every Sunday.
project: juniper
schedule: "0 4 * * 0"
params:
  catalog: https://catalog.example.org/items
steps:
  - id: fetch
    kind: fetch
    with:
      url: ${params.catalog}
      format: json
  - id: transform
    kind: transform
    needs: [fetch]
    retries: 9
    with:
      script: '.items | map({sku, title, words: (.title | ascii_downcase | split(" "))})'
  - id: upload
    kind: upload
    needs: [transform]
    with:
      bucket: juniper-search
      path: index/${params.index}.json
  - id: notify
    kind: notify
    needs: [upload]
    with:
      channel: "#juniper"
      on: [failed, succeeded]
"##;

const ITEM_SCHEMA: &str = r##"{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "title": "Stock count",
  "description": "One line of sync-inventory's output.",
  "type": "object",
  "required": ["sku", "count"],
  "properties": {
    "sku": { "type": "string", "minLength": 4 },
    "count": { "type": "integer", "minimum": 0 },
    "warehouse": { "enum": ["north", "south"] }
  }
}
"##;

const SLOW_STEPS: &str = r##"-- How long each step takes, slowest first
SELECT step, count(*) AS runs, round(avg(duration_ms)) AS avg_ms, max(duration_ms) AS max_ms
FROM steps
WHERE duration_ms IS NOT NULL
GROUP BY step
ORDER BY avg_ms DESC
"##;

const SEND_DIGEST: &str = r##"name: send-digest
description: Sends the weekday digest of new items.
project: larkspur
schedule: "0 8 * * 1-5"
steps:
  - id: fetch
    kind: fetch
    with:
      url: https://feeds.example.org/larkspur/new
  - id: transform
    kind: transform
    needs: [fetch]
    with:
      script: '.entries | sort_by(.published) | reverse | .[:20]'
  - id: upload
    kind: upload
    needs: [transform]
    with:
      bucket: larkspur-digests
      path: digest/${run.date}.json
  - id: notify
    kind: notify
    needs: [transform]
    with:
      channel: "#larkspur"
      on: [succeeded, failed]
      message: "Today's digest has ${steps.transform.rows} new items"
"##;

const DIGEST_SCHEMA: &str = r##"{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "title": "Digest entry",
  "type": "object",
  "required": ["title", "published"],
  "properties": {
    "title": { "type": "strin", "maxLength": 120 },
    "published": { "type": "string", "format": "date" },
    "tags": { "type": "array", "items": { "type": "string" }, "uniqueItems": "yes" }
  }
}
"##;

const BY_OWNER: &str = r##"-- The runs of each owner's jobs, by how they ended
SELECT j.owner, r.state, count(*) AS runs
FROM runs r
JOIN jobs j ON j.name = r.pipeline
GROUP BY j.owner, r.state
ORDER BY j.owner, runs DESC
"##;

/// The files, each `(project, kind, slug, name, text)`.
const FILES: [(Project, FileKind, &str, &str, &str); 11] = [
    (
        Project::Harbor,
        FileKind::Pipeline,
        "nightly-export",
        "nightly-export.yaml",
        NIGHTLY_EXPORT,
    ),
    (
        Project::Harbor,
        FileKind::Pipeline,
        "import-orders",
        "import-orders.yaml",
        IMPORT_ORDERS,
    ),
    (
        Project::Harbor,
        FileKind::Schema,
        "order-schema",
        "order.schema.json",
        ORDER_SCHEMA,
    ),
    (
        Project::Harbor,
        FileKind::Query,
        "failed-runs",
        "failed-runs.sql",
        FAILED_RUNS,
    ),
    (
        Project::Juniper,
        FileKind::Pipeline,
        "sync-inventory",
        "sync-inventory.yaml",
        SYNC_INVENTORY,
    ),
    (
        Project::Juniper,
        FileKind::Pipeline,
        "rebuild-index",
        "rebuild-index.yaml",
        REBUILD_INDEX,
    ),
    (
        Project::Juniper,
        FileKind::Schema,
        "stock-schema",
        "stock-count.schema.json",
        ITEM_SCHEMA,
    ),
    (
        Project::Juniper,
        FileKind::Query,
        "slow-steps",
        "slow-steps.sql",
        SLOW_STEPS,
    ),
    (
        Project::Larkspur,
        FileKind::Pipeline,
        "send-digest",
        "send-digest.yaml",
        SEND_DIGEST,
    ),
    (
        Project::Larkspur,
        FileKind::Schema,
        "digest-schema",
        "digest-entry.schema.json",
        DIGEST_SCHEMA,
    ),
    (
        Project::Larkspur,
        FileKind::Query,
        "runs-by-owner",
        "runs-by-owner.sql",
        BY_OWNER,
    ),
];

/// Every file the workspace starts with, its id `<project>-<slug>`.
pub fn seed_files() -> Vec<WorkFile> {
    FILES
        .iter()
        .map(|(project, kind, slug, name, text)| WorkFile {
            id: format!("{}-{slug}", project.name()),
            project: *project,
            kind: *kind,
            name: (*name).to_owned(),
            text: (*text).to_owned(),
            updated: seeded_at(),
            rank: None,
        })
        .collect()
}

/// A sample value for each seeded schema, by the schema file's id, tried against it beside the editor.
pub const SAMPLES: [(&str, &str); 3] = [
    (
        "harbor-order-schema",
        r##"{
  "id": "ord-104233",
  "total": 0,
  "email": "grace@example.org",
  "lines": [{ "sku": "HB-2201", "count": 2 }]
}
"##,
    ),
    (
        "juniper-stock-schema",
        r##"{ "sku": "JN-7781", "count": 42, "warehouse": "north" }
"##,
    ),
    (
        "larkspur-digest-schema",
        r##"{ "title": "Harbor opens a second bucket", "published": "2026-11-09" }
"##,
    ),
];

/// What a new file of each kind starts with.
pub fn template(kind: FileKind, name: &str, project: Project) -> String {
    match kind {
        FileKind::Pipeline => format!(
            "name: {name}\nproject: {}\nsteps:\n  - id: fetch\n    kind: fetch\n    with:\n      url: https://api.example.org/\n",
            project.name()
        ),
        FileKind::Schema => format!(
            "{{\n  \"$schema\": \"https://json-schema.org/draft/2020-12/schema\",\n  \"title\": \"{name}\",\n  \"type\": \"object\",\n  \"properties\": {{}}\n}}\n"
        ),
        FileKind::Query => format!("-- {name}\nSELECT *\nFROM runs\nLIMIT 20\n"),
    }
}

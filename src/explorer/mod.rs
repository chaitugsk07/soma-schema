//! Visual schema + migration explorer — builds JSON and HTML output.
//!
//! Gated on the `explorer` feature. All SQL parsing uses `sqlparser` (PostgreSQL
//! dialect). The HTML output embeds a self-contained viewer; the `__SOMA_DATA__`
//! placeholder in `viewer.html` is replaced with the serialised JSON.
#![cfg(feature = "explorer")]

use serde::Serialize;
use sqlparser::ast::{AlterTableOperation, ColumnOption, Statement, TableConstraint};
use sqlparser::dialect::PostgreSqlDialect;
use sqlparser::parser::Parser;
use std::collections::{BTreeMap, HashMap};
use std::path::Path;

// ── output structs ────────────────────────────────────────────────────────────

#[derive(Serialize)]
struct MigrationEntry {
    order_index: usize,
    version: u32,
    file: String,
    name: String,
    checksum: String,
    created: Option<String>,
    author: Option<String>,
    why: Option<String>,
    is_seed: bool,
    up_sql: String,
    down_sql: Option<String>,
}

#[derive(Serialize)]
struct OrderEntry {
    order_index: usize,
    version: u32,
    file: String,
    name: String,
}

#[derive(Serialize)]
struct VersionGroup {
    version: u32,
    migrations: Vec<MigrationEntry>,
}

// ── schema / ERD structs ──────────────────────────────────────────────────────

#[derive(Serialize, Clone)]
struct FkRef {
    table: String,
    column: String,
}

#[derive(Serialize, Clone)]
struct ColumnDef {
    name: String,
    #[serde(rename = "type")]
    col_type: String,
    pk: bool,
    nullable: bool,
    unique: bool,
    default: Option<String>,
    fk: Option<FkRef>,
}

#[derive(Serialize)]
struct TableDef {
    name: String,
    schema: String,
    x: i64,
    y: i64,
    columns: Vec<ColumnDef>,
}

#[derive(Serialize)]
struct Relation {
    from_table: String,
    from_column: String,
    to_table: String,
    to_column: String,
}

#[derive(Serialize)]
struct SeedTable {
    table: String,
    columns: Vec<String>,
    rows: Vec<Vec<String>>,
}

#[derive(Serialize)]
struct Schema {
    tables: Vec<TableDef>,
    relations: Vec<Relation>,
}

#[derive(Serialize)]
struct Output {
    generated_for: String,
    versions: Vec<VersionGroup>,
    apply_order: Vec<OrderEntry>,
    rollback_order: Vec<OrderEntry>,
    setup_files: Vec<String>,
    schema: Schema,
    seed_data: Vec<SeedTable>,
}

// ── schema builder ────────────────────────────────────────────────────────────

/// Internal mutable table representation during parsing.
#[derive(Default, Clone)]
struct TableBuf {
    schema: String,
    columns: Vec<ColumnDef>,
}

/// Strip a schema qualifier and return (schema, table).
/// "vault.organizations" → ("vault", "organizations")
/// "organizations"       → ("", "organizations")
fn split_table_name(name: &sqlparser::ast::ObjectName) -> (String, String) {
    let parts: Vec<String> = name.0.iter().map(|i| i.value.clone()).collect();
    match parts.len() {
        0 => (String::new(), String::new()),
        1 => (String::new(), parts[0].clone()),
        _ => (
            parts[parts.len() - 2].clone(),
            parts[parts.len() - 1].clone(),
        ),
    }
}

/// Render a DataType to a compact string for ERD display.
fn render_type(dt: &sqlparser::ast::DataType) -> String {
    let s = format!("{dt}");
    match s.as_str() {
        "CHARACTER VARYING" | "VARCHAR" => "text".to_owned(),
        "TIMESTAMP WITH TIME ZONE" | "TIMESTAMPTZ" => "timestamptz".to_owned(),
        "BOOLEAN" | "BOOL" => "bool".to_owned(),
        "INTEGER" | "INT" | "INT4" => "int".to_owned(),
        "BIGINT" | "INT8" => "bigint".to_owned(),
        "TEXT" => "text".to_owned(),
        "UUID" => "uuid".to_owned(),
        _ => s.to_lowercase(),
    }
}

/// Accumulate CREATE TABLE and ALTER TABLE statements into the schema map.
fn ingest_sql(sql: &str, tables: &mut BTreeMap<String, TableBuf>) {
    let dialect = PostgreSqlDialect {};
    let stmts = match Parser::parse_sql(&dialect, sql) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("  [sql-parse] skipping unparseable block: {e}");
            return;
        }
    };

    for stmt in stmts {
        if let Err(e) = ingest_stmt(stmt, tables) {
            eprintln!("  [sql-parse] skipping statement: {e}");
        }
    }
}

fn ingest_stmt(stmt: Statement, tables: &mut BTreeMap<String, TableBuf>) -> Result<(), String> {
    match stmt {
        Statement::CreateTable(ct) => {
            let (schema, tname) = split_table_name(&ct.name);
            let entry = tables.entry(tname.clone()).or_default();
            if !schema.is_empty() {
                entry.schema = schema;
            }

            let mut pk_cols: std::collections::HashSet<String> = std::collections::HashSet::new();
            let mut unique_cols: std::collections::HashSet<String> =
                std::collections::HashSet::new();
            for tc in &ct.constraints {
                match tc {
                    TableConstraint::PrimaryKey { columns, .. } => {
                        for c in columns {
                            pk_cols.insert(c.value.clone());
                        }
                    }
                    TableConstraint::Unique { columns, .. } if columns.len() == 1 => {
                        unique_cols.insert(columns[0].value.clone());
                    }
                    TableConstraint::Unique { .. } => {}
                    _ => {}
                }
            }

            let mut table_fks: Vec<(String, String, String)> = Vec::new();
            for tc in &ct.constraints {
                if let TableConstraint::ForeignKey {
                    columns,
                    foreign_table,
                    referred_columns,
                    ..
                } = tc
                {
                    let (_, ref_table) = split_table_name(foreign_table);
                    let ref_col = referred_columns
                        .first()
                        .map(|c| c.value.clone())
                        .unwrap_or_default();
                    if let Some(col) = columns.first() {
                        table_fks.push((col.value.clone(), ref_table, ref_col));
                    }
                }
            }

            for col in &ct.columns {
                let cname = col.name.value.clone();
                let mut is_pk = pk_cols.contains(&cname);
                let mut is_unique = unique_cols.contains(&cname);
                let mut nullable = true;
                let mut default_val: Option<String> = None;
                let mut fk: Option<FkRef> = None;

                for opt in &col.options {
                    match &opt.option {
                        ColumnOption::NotNull => {
                            nullable = false;
                        }
                        ColumnOption::Null => {
                            nullable = true;
                        }
                        ColumnOption::Unique { is_primary, .. } => {
                            if *is_primary {
                                is_pk = true;
                                nullable = false;
                            } else {
                                is_unique = true;
                            }
                        }
                        ColumnOption::Default(expr) => {
                            default_val = Some(expr.to_string());
                        }
                        ColumnOption::ForeignKey {
                            foreign_table,
                            referred_columns,
                            ..
                        } => {
                            let (_, ref_table) = split_table_name(foreign_table);
                            let ref_col = referred_columns
                                .first()
                                .map(|c| c.value.clone())
                                .unwrap_or_default();
                            fk = Some(FkRef {
                                table: ref_table,
                                column: ref_col,
                            });
                        }
                        _ => {}
                    }
                }

                if fk.is_none() {
                    for (fc, rt, rc) in &table_fks {
                        if fc == &cname {
                            fk = Some(FkRef {
                                table: rt.clone(),
                                column: rc.clone(),
                            });
                            break;
                        }
                    }
                }

                entry.columns.push(ColumnDef {
                    name: cname,
                    col_type: render_type(&col.data_type),
                    pk: is_pk,
                    nullable,
                    unique: is_unique,
                    default: default_val,
                    fk,
                });
            }
        }

        Statement::AlterTable {
            name, operations, ..
        } => {
            let (_, tname) = split_table_name(&name);

            for op in operations {
                match op {
                    AlterTableOperation::AddColumn { column_def, .. } => {
                        let cname = column_def.name.value.clone();
                        let mut nullable = true;
                        let mut is_unique = false;
                        let mut default_val: Option<String> = None;
                        let mut fk: Option<FkRef> = None;

                        for opt in &column_def.options {
                            match &opt.option {
                                ColumnOption::NotNull => nullable = false,
                                ColumnOption::Null => nullable = true,
                                ColumnOption::Unique { .. } => is_unique = true,
                                ColumnOption::Default(expr) => {
                                    default_val = Some(expr.to_string());
                                }
                                ColumnOption::ForeignKey {
                                    foreign_table,
                                    referred_columns,
                                    ..
                                } => {
                                    let (_, rt) = split_table_name(foreign_table);
                                    let rc = referred_columns
                                        .first()
                                        .map(|c| c.value.clone())
                                        .unwrap_or_default();
                                    fk = Some(FkRef {
                                        table: rt,
                                        column: rc,
                                    });
                                }
                                _ => {}
                            }
                        }

                        let entry = tables.entry(tname.clone()).or_default();
                        entry.columns.push(ColumnDef {
                            name: cname,
                            col_type: render_type(&column_def.data_type),
                            pk: false,
                            nullable,
                            unique: is_unique,
                            default: default_val,
                            fk,
                        });
                    }

                    AlterTableOperation::AddConstraint(TableConstraint::ForeignKey {
                        columns,
                        foreign_table,
                        referred_columns,
                        ..
                    }) => {
                        let (_, ref_table) = split_table_name(&foreign_table);
                        let ref_col = referred_columns
                            .first()
                            .map(|c| c.value.clone())
                            .unwrap_or_default();

                        if let Some(fk_col) = columns.first() {
                            let fk_col_name = fk_col.value.clone();
                            let entry = tables.entry(tname.clone()).or_default();
                            if let Some(col) =
                                entry.columns.iter_mut().find(|c| c.name == fk_col_name)
                            {
                                col.fk = Some(FkRef {
                                    table: ref_table,
                                    column: ref_col,
                                });
                            } else {
                                entry.columns.push(ColumnDef {
                                    name: fk_col_name,
                                    col_type: "uuid".to_owned(),
                                    pk: false,
                                    nullable: false,
                                    unique: false,
                                    default: None,
                                    fk: Some(FkRef {
                                        table: ref_table,
                                        column: ref_col,
                                    }),
                                });
                            }
                        }
                    }
                    AlterTableOperation::AddConstraint(_) => {}

                    _ => {}
                }
            }
        }

        _ => {}
    }
    Ok(())
}

/// Compute layered x/y layout.
/// depth(t) = 0 if no outgoing FKs; else 1 + max(depth(referenced tables)).
fn compute_layout(tables: &BTreeMap<String, TableBuf>) -> HashMap<String, (i64, i64)> {
    const X_STEP: i64 = 380;
    const HEADER_H: f64 = 38.0;
    const ROW_H: f64 = 30.0;
    const GAP: f64 = 48.0;

    let refs: HashMap<&str, Vec<&str>> = tables
        .iter()
        .map(|(name, buf)| {
            let targets: Vec<&str> = buf
                .columns
                .iter()
                .filter_map(|c| c.fk.as_ref())
                .map(|fk| fk.table.as_str())
                .collect();
            (name.as_str(), targets)
        })
        .collect();

    let mut depth_cache: HashMap<String, i64> = HashMap::new();

    fn depth(
        name: &str,
        refs: &HashMap<&str, Vec<&str>>,
        cache: &mut HashMap<String, i64>,
        visiting: &mut Vec<String>,
    ) -> i64 {
        if let Some(&d) = cache.get(name) {
            return d;
        }
        if visiting.contains(&name.to_owned()) {
            return 0;
        }
        visiting.push(name.to_owned());
        let d = refs
            .get(name)
            .map(|targets| {
                targets
                    .iter()
                    .map(|t| 1 + depth(t, refs, cache, visiting))
                    .max()
                    .unwrap_or(0)
            })
            .unwrap_or(0);
        let d = d.min(6);
        visiting.pop();
        cache.insert(name.to_owned(), d);
        d
    }

    let mut visiting: Vec<String> = Vec::new();
    for name in tables.keys() {
        depth(name, &refs, &mut depth_cache, &mut visiting);
    }

    let mut by_layer: BTreeMap<i64, Vec<&str>> = BTreeMap::new();
    for name in tables.keys() {
        let d = *depth_cache.get(name.as_str()).unwrap_or(&0);
        by_layer.entry(d).or_default().push(name.as_str());
    }

    let mut positions: HashMap<String, (i64, i64)> = HashMap::new();
    for (layer, names) in &by_layer {
        let mut y_cursor: f64 = 0.0;
        for name in names {
            let num_cols = tables.get(*name).map(|b| b.columns.len()).unwrap_or(0);
            let card_h = HEADER_H + num_cols as f64 * ROW_H;
            let y = y_cursor.round() as i64;
            positions.insert(name.to_string(), (layer * X_STEP, y));
            y_cursor += card_h + GAP;
        }
    }
    positions
}

// ponytail: only handles INSERT … (columns) VALUES (…) — not INSERT … SELECT or
// columnless inserts. That's sufficient for the seed-migration convention (explicit
// column lists required). Upgrade path: match more Insert variants if needed.

/// Returns true if the SQL contains only data statements (≥1 INSERT, no DDL).
fn is_seed_sql(sql: &str) -> bool {
    let dialect = PostgreSqlDialect {};
    let stmts = match Parser::parse_sql(&dialect, sql) {
        Ok(s) => s,
        Err(_) => return false,
    };
    if stmts.is_empty() {
        return false;
    }
    let mut has_insert = false;
    for stmt in &stmts {
        match stmt {
            Statement::Insert(_) => has_insert = true,
            Statement::CreateTable(_) | Statement::AlterTable { .. } | Statement::Drop { .. } => {
                return false;
            }
            _ => {}
        }
    }
    has_insert
}

/// Render a sqlparser Expr to a display string.
/// String literals are unquoted; everything else uses Display.
fn render_value(expr: &sqlparser::ast::Expr) -> String {
    use sqlparser::ast::{Expr, Value};
    match expr {
        Expr::Value(Value::SingleQuotedString(s)) => s.clone(),
        Expr::Value(Value::Null) => "NULL".to_owned(),
        _ => expr.to_string(),
    }
}

/// Parse INSERT statements from UP SQL and accumulate into `order` (table insertion order)
/// and `data` (table → (columns, rows)).
fn collect_seed_data(
    sql: &str,
    order: &mut Vec<String>,
    data: &mut std::collections::HashMap<String, (Vec<String>, Vec<Vec<String>>)>,
) {
    let dialect = PostgreSqlDialect {};
    let stmts = match Parser::parse_sql(&dialect, sql) {
        Ok(s) => s,
        Err(_) => return,
    };
    for stmt in stmts {
        let Statement::Insert(insert) = stmt else {
            continue;
        };
        let (_, tname) = split_table_name(&insert.table_name);
        if tname.is_empty() {
            continue;
        }
        let cols: Vec<String> = insert.columns.iter().map(|c| c.value.clone()).collect();
        if cols.is_empty() {
            continue;
        }
        let rows: Vec<Vec<String>> = {
            let Some(src) = insert.source else { continue };
            let sqlparser::ast::SetExpr::Values(vals) = *src.body else {
                continue;
            };
            vals.rows
                .iter()
                .map(|row| row.iter().map(render_value).collect())
                .collect()
        };
        if rows.is_empty() {
            continue;
        }
        let entry = data.entry(tname.clone()).or_insert_with(|| {
            order.push(tname.clone());
            (cols, Vec::new())
        });
        entry.1.extend(rows);
    }
}

/// Build the `Output` struct from `root`, serialise to pretty JSON.
pub fn build_json(root: &Path) -> crate::Result<String> {
    let (migrations, setup_files) = crate::discover(root)?;

    let mut apply_order: Vec<OrderEntry> = Vec::with_capacity(migrations.len());
    let mut all_entries: Vec<MigrationEntry> = Vec::with_capacity(migrations.len());

    for (idx, m) in migrations.iter().enumerate() {
        apply_order.push(OrderEntry {
            order_index: idx,
            version: m.version,
            file: m.file.clone(),
            name: m.name.clone(),
        });
        all_entries.push(MigrationEntry {
            order_index: idx,
            version: m.version,
            file: m.file.clone(),
            name: m.name.clone(),
            checksum: m.checksum.clone(),
            created: m.created.clone(),
            author: m.author.clone(),
            why: m.why.clone(),
            is_seed: is_seed_sql(&m.up()),
            up_sql: m.up(),
            down_sql: m.down(),
        });
    }

    let mut rollback_order: Vec<OrderEntry> = apply_order
        .iter()
        .map(|e| OrderEntry {
            order_index: e.order_index,
            version: e.version,
            file: e.file.clone(),
            name: e.name.clone(),
        })
        .collect();
    rollback_order.reverse();

    let mut by_version: BTreeMap<u32, Vec<MigrationEntry>> = BTreeMap::new();
    for entry in all_entries {
        by_version.entry(entry.version).or_default().push(entry);
    }
    let versions: Vec<VersionGroup> = by_version
        .into_iter()
        .map(|(v, migs)| VersionGroup {
            version: v,
            migrations: migs,
        })
        .collect();

    let mut tables_buf: BTreeMap<String, TableBuf> = BTreeMap::new();
    for m in &migrations {
        let up = m.up();
        ingest_sql(&up, &mut tables_buf);
    }

    let positions = compute_layout(&tables_buf);

    let mut relations: Vec<Relation> = Vec::new();
    for (tname, buf) in &tables_buf {
        for col in &buf.columns {
            if let Some(fk) = &col.fk {
                relations.push(Relation {
                    from_table: tname.clone(),
                    from_column: col.name.clone(),
                    to_table: fk.table.clone(),
                    to_column: fk.column.clone(),
                });
            }
        }
    }

    let tables: Vec<TableDef> = tables_buf
        .into_iter()
        .map(|(name, buf)| {
            let (x, y) = positions.get(&name).copied().unwrap_or((0, 0));
            TableDef {
                name,
                schema: buf.schema,
                x,
                y,
                columns: buf.columns,
            }
        })
        .collect();

    // Collect seed data from seed migrations
    let mut seed_order: Vec<String> = Vec::new();
    let mut seed_data_map: std::collections::HashMap<String, (Vec<String>, Vec<Vec<String>>)> =
        std::collections::HashMap::new();
    for m in &migrations {
        let up = m.up();
        if is_seed_sql(&up) {
            collect_seed_data(&up, &mut seed_order, &mut seed_data_map);
        }
    }
    // Order seed tables to match schema table order, appending any extras at the end
    let schema_table_names: Vec<String> = tables.iter().map(|t| t.name.clone()).collect();
    let mut ordered: Vec<String> = schema_table_names
        .iter()
        .filter(|n| seed_data_map.contains_key(*n))
        .cloned()
        .collect();
    for t in &seed_order {
        if !ordered.contains(t) {
            ordered.push(t.clone());
        }
    }
    let seed_data: Vec<SeedTable> = ordered
        .into_iter()
        .filter_map(|tname| {
            seed_data_map.remove(&tname).map(|(cols, rows)| SeedTable {
                table: tname,
                columns: cols,
                rows,
            })
        })
        .collect();

    let output = Output {
        generated_for: root.display().to_string(),
        versions,
        apply_order,
        rollback_order,
        setup_files: setup_files.into_iter().map(|s| s.name).collect(),
        schema: Schema { tables, relations },
        seed_data,
    };

    serde_json::to_string_pretty(&output).map_err(|e| crate::Error::Explorer(e.to_string()))
}

/// Build and embed JSON into the viewer HTML template.
///
/// Replaces the `__SOMA_DATA__` placeholder in `viewer.html` with the serialised JSON.
/// The JSON is sanitised so that a literal `</script>` in any data value cannot break out
/// of the enclosing `<script>` block. `<\/script>` is valid JSON and parses identically.
pub fn render_html(root: &Path) -> crate::Result<String> {
    let json = build_json(root)?;
    // Prevent </script> tag breakout: replace every occurrence so the literal tag
    // sequence never appears inside the <script> block, regardless of data content.
    let json = json.replace("</script>", r"<\/script>");
    Ok(include_str!("viewer.html").replace("__SOMA_DATA__", &json))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn insert_only_up_is_seed() {
        let sql = "INSERT INTO s.t (a, b) VALUES ('x', 1), ('y', 2);";
        assert!(is_seed_sql(sql));
    }

    #[test]
    fn create_table_up_is_not_seed() {
        let sql = "CREATE TABLE foo (id SERIAL PRIMARY KEY);";
        assert!(!is_seed_sql(sql));
    }

    #[test]
    fn collect_seed_data_parses_insert() {
        let sql = "INSERT INTO s.t (a, b) VALUES ('x', 1), ('y', 2);";
        let mut order: Vec<String> = Vec::new();
        let mut data: std::collections::HashMap<String, (Vec<String>, Vec<Vec<String>>)> =
            std::collections::HashMap::new();
        collect_seed_data(sql, &mut order, &mut data);

        assert_eq!(order, vec!["t"]);
        let (cols, rows) = data.get("t").expect("table t must be present");
        assert_eq!(cols, &["a", "b"]);
        assert_eq!(rows.len(), 2);
        // Single-quoted string 'x' should be rendered unquoted as x
        assert_eq!(rows[0][0], "x");
        assert_eq!(rows[1][0], "y");
    }

    #[test]
    fn create_table_contributes_no_seed_rows() {
        let sql = "CREATE TABLE foo (id SERIAL PRIMARY KEY);";
        let mut order: Vec<String> = Vec::new();
        let mut data: std::collections::HashMap<String, (Vec<String>, Vec<Vec<String>>)> =
            std::collections::HashMap::new();
        collect_seed_data(sql, &mut order, &mut data);
        assert!(order.is_empty());
        assert!(data.is_empty());
    }

    #[test]
    fn render_value_unquotes_string() {
        use sqlparser::ast::{Expr, Value};
        let expr = Expr::Value(Value::SingleQuotedString("hello".to_owned()));
        assert_eq!(render_value(&expr), "hello");
    }

    /// Verify that render_html() sanitises </script> so a data value containing that
    /// sequence cannot break out of the enclosing <script> block in the generated HTML.
    #[test]
    fn render_html_escapes_script_breakout() {
        // Build a minimal migrations directory in a temp dir so render_html can run.
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();

        // Write migration-order.yaml
        std::fs::create_dir_all(root.join("01_migrated/1")).unwrap();
        std::fs::write(
            root.join("migration-order.yaml"),
            r#"manifest_version: 1
versions:
  - version: 1
    description: "test"
    migrations:
      - file: "20260101_01_xss.sql"
        why: "</script><script>alert(1)</script>"
        author: "attacker"
        created: "2026-01-01"
"#,
        )
        .unwrap();

        // Write the migration SQL file (content doesn't matter for this test)
        std::fs::write(
            root.join("01_migrated/1/20260101_01_xss.sql"),
            "-- UP section\nSELECT 1;\n\n-- DOWN ==\n-- nothing\n",
        )
        .unwrap();

        let html = render_html(root).expect("render_html should succeed");

        // The raw breakout sequence must NOT appear in the output.
        assert!(
            !html.contains("</script><script>"),
            "raw </script> breakout must not appear in rendered HTML"
        );

        // The escaped form must be present (proves the sanitisation ran).
        assert!(
            html.contains(r"<\/script>"),
            "escaped <\\/script> must appear in rendered HTML"
        );
    }
}

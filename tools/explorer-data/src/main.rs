use serde::Serialize;
use soma_schema::discovery::discover;
use sqlparser::ast::{
    AlterTableOperation, ColumnOption, Statement, TableConstraint,
};
use sqlparser::dialect::PostgreSqlDialect;
use sqlparser::parser::Parser;
use std::collections::{BTreeMap, HashMap};
use std::env;
use std::path::Path;

// ── existing output structs ──────────────────────────────────────────────────

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

// ── schema / ERD structs ─────────────────────────────────────────────────────

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
struct Schema {
    tables: Vec<TableDef>,
    relations: Vec<Relation>,
}

// ── output root ──────────────────────────────────────────────────────────────

#[derive(Serialize)]
struct Output {
    generated_for: String,
    versions: Vec<VersionGroup>,
    apply_order: Vec<OrderEntry>,
    rollback_order: Vec<OrderEntry>,
    setup_files: Vec<String>,
    schema: Schema,
}

// ── schema builder ───────────────────────────────────────────────────────────

/// Strip a schema qualifier and return (schema, table).
/// "vault.organizations" → ("vault", "organizations")
/// "organizations"       → ("", "organizations")
fn split_table_name(name: &sqlparser::ast::ObjectName) -> (String, String) {
    let parts: Vec<String> = name.0.iter().map(|i| i.value.clone()).collect();
    match parts.len() {
        0 => (String::new(), String::new()),
        1 => (String::new(), parts[0].clone()),
        _ => (parts[parts.len() - 2].clone(), parts[parts.len() - 1].clone()),
    }
}

/// Render a DataType to a compact string, dropping modifiers we don't need.
fn render_type(dt: &sqlparser::ast::DataType) -> String {
    // Use Debug formatting then clean it up a bit — good enough for ERD display.
    let s = format!("{dt}");
    // Normalise common verbose forms.
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

/// Internal mutable table representation during parsing.
#[derive(Default, Clone)]
struct TableBuf {
    schema: String,
    columns: Vec<ColumnDef>,
}

/// Accumulate CREATE TABLE and ALTER TABLE statements into the schema map.
/// Key is the bare table name (no schema qualifier).
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
        // Try statement-by-statement; ignore anything we don't handle.
        if let Err(e) = ingest_stmt(stmt, tables) {
            eprintln!("  [sql-parse] skipping statement: {e}");
        }
    }
}

fn ingest_stmt(
    stmt: Statement,
    tables: &mut BTreeMap<String, TableBuf>,
) -> Result<(), String> {
    match stmt {
        Statement::CreateTable(ct) => {
            let (schema, tname) = split_table_name(&ct.name);
            let entry = tables.entry(tname.clone()).or_default();
            if !schema.is_empty() {
                entry.schema = schema;
            }

            // Collect PK and UNIQUE columns from table-level constraints first.
            let mut pk_cols: std::collections::HashSet<String> =
                std::collections::HashSet::new();
            let mut unique_cols: std::collections::HashSet<String> =
                std::collections::HashSet::new();
            for tc in &ct.constraints {
                match tc {
                    TableConstraint::PrimaryKey { columns, .. } => {
                        for c in columns {
                            pk_cols.insert(c.value.clone());
                        }
                    }
                    TableConstraint::Unique { columns, .. } => {
                        // Only mark as unique if it's a single-column constraint.
                        if columns.len() == 1 {
                            unique_cols.insert(columns[0].value.clone());
                        }
                    }
                    _ => {}
                }
            }

            // Collect table-level FK constraints.
            let mut table_fks: Vec<(String, String, String)> = Vec::new(); // (col, ref_table, ref_col)
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
                            fk = Some(FkRef { table: ref_table, column: ref_col });
                        }
                        _ => {}
                    }
                }

                // Check table-level FK map.
                if fk.is_none() {
                    for (fc, rt, rc) in &table_fks {
                        if fc == &cname {
                            fk = Some(FkRef { table: rt.clone(), column: rc.clone() });
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

        Statement::AlterTable { name, operations, .. } => {
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
                                    fk = Some(FkRef { table: rt, column: rc });
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

                    AlterTableOperation::AddConstraint(tc) => {
                        if let TableConstraint::ForeignKey {
                            columns,
                            foreign_table,
                            referred_columns,
                            ..
                        } = tc
                        {
                            let (_, ref_table) = split_table_name(&foreign_table);
                            let ref_col = referred_columns
                                .first()
                                .map(|c| c.value.clone())
                                .unwrap_or_default();

                            if let Some(fk_col) = columns.first() {
                                let fk_col_name = fk_col.value.clone();
                                let entry = tables.entry(tname.clone()).or_default();
                                // Find the column and set its FK.
                                if let Some(col) =
                                    entry.columns.iter_mut().find(|c| c.name == fk_col_name)
                                {
                                    col.fk = Some(FkRef {
                                        table: ref_table,
                                        column: ref_col,
                                    });
                                } else {
                                    // Column doesn't exist yet — add a placeholder.
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
                    }

                    // DROP COLUMN etc. — silently ignore for ERD purposes.
                    _ => {}
                }
            }
        }

        // Any other statement (INSERT, SELECT, CREATE INDEX, etc.) — skip silently.
        _ => {}
    }
    Ok(())
}

/// Compute layered x/y layout.
/// depth(t) = 0 if no outgoing FKs; else 1 + max(depth(referenced tables)).
///
/// Card height = HEADER_H + num_columns * ROW_H (must match soma-ui component).
/// Cards in the same depth column are stacked cumulatively with GAP between them
/// so that tables with many columns never overlap the next card.
fn compute_layout(tables: &BTreeMap<String, TableBuf>) -> HashMap<String, (i64, i64)> {
    // Horizontal: card width 260 + ~120 gap between columns.
    const X_STEP: i64 = 380;
    // Card dimensions — must match soma-ui HEADER_H / ROW_H constants.
    const HEADER_H: f64 = 38.0;
    const ROW_H: f64 = 30.0;
    // Vertical gap between card bottom and next card top.
    const GAP: f64 = 48.0;

    // Build adjacency: table → set of tables it references (outgoing FKs).
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

    // Memoised depth.
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
            return 0; // cycle guard
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

    // Group tables by depth, preserving BTreeMap order within each layer.
    let mut by_layer: BTreeMap<i64, Vec<&str>> = BTreeMap::new();
    for name in tables.keys() {
        let d = *depth_cache.get(name.as_str()).unwrap_or(&0);
        by_layer.entry(d).or_default().push(name.as_str());
    }

    // Stack cards cumulatively per column so cards never overlap.
    let mut positions: HashMap<String, (i64, i64)> = HashMap::new();
    for (layer, names) in &by_layer {
        let mut y_cursor: f64 = 0.0;
        for name in names {
            let num_cols = tables.get(*name).map(|b| b.columns.len()).unwrap_or(0);
            let card_h = HEADER_H + num_cols as f64 * ROW_H;
            let y = y_cursor.round() as i64;
            positions.insert(name.to_string(), (layer * X_STEP, y));
            eprintln!(
                "  layout: {name:40} x={x:4}  y={y:5}  h={h:.0}  cols={cols}",
                x = layer * X_STEP,
                h = card_h,
                cols = num_cols
            );
            y_cursor += card_h + GAP;
        }
    }
    positions
}

// ── main ─────────────────────────────────────────────────────────────────────

fn main() {
    let args: Vec<String> = env::args().collect();
    let root_str = args.get(1).map(String::as_str).unwrap_or("examples/showcase");
    let out_str =
        args.get(2).map(String::as_str).unwrap_or("website/data/migrations.json");

    let root = Path::new(root_str);
    let out_path = Path::new(out_str);

    let (migrations, setup_files) = discover(root).unwrap_or_else(|e| {
        eprintln!("discover error: {e}");
        std::process::exit(1);
    });

    // ── build migration entries ──────────────────────────────────────────────

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
            up_sql: m.up(),
            down_sql: m.down(),
        });
    }

    // Self-check: versions must be non-decreasing in apply_order.
    for w in apply_order.windows(2) {
        assert!(
            w[0].version <= w[1].version,
            "apply_order version not non-decreasing at index {}",
            w[1].order_index
        );
    }

    // rollback_order is exact reverse of apply_order.
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

    // Self-check: rollback_order is the reverse.
    assert_eq!(apply_order.len(), rollback_order.len());
    for (a, r) in apply_order.iter().zip(rollback_order.iter().rev()) {
        assert_eq!(a.order_index, r.order_index);
    }

    // Group by version.
    let mut by_version: BTreeMap<u32, Vec<MigrationEntry>> = BTreeMap::new();
    for entry in all_entries {
        by_version.entry(entry.version).or_default().push(entry);
    }
    let versions: Vec<VersionGroup> = by_version
        .into_iter()
        .map(|(v, migs)| VersionGroup { version: v, migrations: migs })
        .collect();

    // ── build schema from UP SQL in apply order ──────────────────────────────

    let mut tables_buf: BTreeMap<String, TableBuf> = BTreeMap::new();

    for m in &migrations {
        let up = m.up();
        eprintln!("  parsing UP of {}", m.file);
        ingest_sql(&up, &mut tables_buf);
    }

    // Compute positions.
    let positions = compute_layout(&tables_buf);

    // Build relations list.
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

    // Convert to final TableDef vec (sorted by name for determinism).
    let tables: Vec<TableDef> = tables_buf
        .into_iter()
        .map(|(name, buf)| {
            let (x, y) = positions.get(&name).copied().unwrap_or((0, 0));
            TableDef {
                name: name.clone(),
                schema: buf.schema,
                x,
                y,
                columns: buf.columns,
            }
        })
        .collect();

    // ── self-checks ──────────────────────────────────────────────────────────

    let table_names: std::collections::HashSet<&str> =
        tables.iter().map(|t| t.name.as_str()).collect();

    assert!(
        tables.len() >= 8,
        "expected ≥8 tables, got {}",
        tables.len()
    );
    assert!(
        relations.len() >= 6,
        "expected ≥6 relations, got {}",
        relations.len()
    );
    for rel in &relations {
        assert!(
            table_names.contains(rel.to_table.as_str()),
            "relation.to_table '{}' not found in tables",
            rel.to_table
        );
    }

    eprintln!(
        "schema: {} tables, {} relations, {} migrations",
        tables.len(),
        relations.len(),
        migrations.len()
    );

    let schema = Schema { tables, relations };

    // ── write output ─────────────────────────────────────────────────────────

    let output = Output {
        generated_for: root.display().to_string(),
        versions,
        apply_order,
        rollback_order,
        setup_files: setup_files.into_iter().map(|s| s.name).collect(),
        schema,
    };

    if let Some(parent) = out_path.parent() {
        std::fs::create_dir_all(parent).unwrap_or_else(|e| {
            eprintln!("create_dir_all {}: {e}", parent.display());
            std::process::exit(1);
        });
    }
    let json = serde_json::to_string_pretty(&output).expect("serialization failed");
    std::fs::write(out_path, &json).unwrap_or_else(|e| {
        eprintln!("write {}: {e}", out_path.display());
        std::process::exit(1);
    });

    println!(
        "{} migrations across {} versions → {}",
        output.apply_order.len(),
        output.versions.len(),
        out_path.display()
    );
}

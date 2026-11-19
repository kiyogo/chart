// Re-embed SQL migrations (sqlx::migrate!) whenever a migration file is added or changed.
fn main() {
    println!("cargo:rerun-if-changed=migrations");
}

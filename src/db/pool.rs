use r2d2_sqlite::SqliteConnectionManager;
use r2d2::Pool;

pub type DbPool = Pool<SqliteConnectionManager>;

pub fn create_db_pool(database_path: &str) -> DbPool {
    let manager = SqliteConnectionManager::file(database_path);
    Pool::builder()
        .max_size(15)
        .build(manager)
        .expect("Failed to create database pool")
}

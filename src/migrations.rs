pub struct Migrations(sqlx::SqlitePool);

impl Migrations {
    pub async fn new(pool: sqlx::SqlitePool) -> Result<Self, sqlx::Error> {
        let mut cxn = pool.acquire().await.unwrap();
        sqlx::query(sql_file!("migrations_schema"))
            .execute(&mut cxn)
            .await?;

        Ok(Self(pool))
    }

    pub async fn run<T>(&self, migration: NeedsMigration<T>) -> Result<T, sqlx::Error>
    where
        T: MigrationInfo,
    {
        let mut cxn = self.0.acquire().await.unwrap();
        for Migration { ident, migration } in migration.0.migrations() {
            let is_migrated = sqlx::query!("SELECT ident FROM migrations WHERE ident = ?", ident)
                .fetch_optional(&mut *cxn)
                .await?
                .is_some();

            if !is_migrated {
                sqlx::query(migration).execute(&mut *cxn).await?;
            }
        }

        Ok(migration.0)
    }
}

pub struct Migration {
    pub ident: &'static str,
    pub migration: &'static str,
}

pub struct NeedsMigration<T>(T)
where
    T: MigrationInfo;

impl<T> NeedsMigration<T>
where
    T: MigrationInfo,
{
    pub fn new(thing: T) -> Self {
        Self(thing)
    }
}

pub trait MigrationInfo {
    fn migrations(&self) -> &'static [Migration];
}

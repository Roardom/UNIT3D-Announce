use arc_swap::ArcSwap;

use sqlx::MySqlPool;

use anyhow::{Context, Result};

use crate::config;
use crate::queue::Queues;
use crate::stats::Stats;
use crate::store::Stores;

use dotenvy::dotenv;
use sqlx::mysql::MySqlPoolOptions;
use std::io::{self, Write};
use std::{env, sync::Arc, time::Duration};

pub struct Tracker {
    pub config: ArcSwap<config::Config>,
    pub pool: MySqlPool,
    pub queues: Queues,
    pub stats: Stats,
    pub stores: Stores,
}

impl Tracker {
    /// Creates a database connection pool, and loads all relevant tracker
    /// data into this shared tracker context. This is then passed to all
    /// handlers.
    pub async fn default() -> Result<Arc<Tracker>> {
        print!(".env file: verifying file exists                       ... ");
        io::stdout().flush().unwrap();
        let env_path = dotenv().context(".env file not found.")?;
        println!("[Finished] Path: {:?}", env_path);

        print!("Loading config from env                                ... ");
        io::stdout().flush().unwrap();
        let config = config::Config::from_env()?;
        println!("[Finished]");

        print!("Connecting to database                                 ... ");
        io::stdout().flush().unwrap();
        let pool = connect_to_database().await;
        println!("[Finished]");

        let stores = Stores::new(&pool, &config).await?;

        let stats = Stats::default();

        Ok(Arc::new(Tracker {
            config: ArcSwap::from_pointee(config),
            pool,
            queues: Queues::new(),
            stats,
            stores,
        }))
    }
}

/// Uses the values in the .env file to create a connection pool to the database
async fn connect_to_database() -> sqlx::Pool<sqlx::MySql> {
    // Get pool of database connections.
    MySqlPoolOptions::new()
        .min_connections(0)
        .max_connections(60)
        .max_lifetime(Duration::from_secs(30 * 60))
        .idle_timeout(Duration::from_secs(10 * 60))
        .acquire_timeout(Duration::from_secs(30))
        .connect(&env::var("DATABASE_URL").expect("DATABASE_URL not found in .env file. Aborting."))
        .await
        .expect("Could not connect to the database using the DATABASE_URL value in .env file. Aborting.")
}

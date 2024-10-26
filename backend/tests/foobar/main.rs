use std::{
    borrow::BorrowMut, collections::HashMap, future::Future, mem::take, process::Stdio, str,
    sync::Arc, time::Duration,
};

use eyre::{bail, eyre, Context};
use libtest_mimic::{Failed, Trial};
use serde::{Deserialize, Serialize};
use singit_srv::{db::DbPool, Opt};
use tokio::{
    io::{AsyncBufReadExt, BufReader},
    process::Command,
    runtime::{Handle, Runtime},
    time::timeout,
};

fn main() -> eyre::Result<()> {
    let args = libtest_mimic::Arguments::from_args();

    let runtime = Box::leak(Box::new(Runtime::new()?));
    let handle = runtime.handle();

    let db = runtime.block_on(MockDb::new())?;
    let db = Arc::new(db);

    let outcome = libtest_mimic::run(
        &args,
        vec![Trial::test(
            "insert_and_get",
            runner(&handle, &db, &insert_and_get),
        )],
    );

    drop(db);

    outcome.exit()
}

fn runner<F, Fut>(
    runtime: &'static Handle,
    db: &Arc<MockDb>,
    test: F,
) -> impl FnOnce() -> Result<(), Failed> + Send + 'static
where
    F: Fn(Arc<MockDb>) -> Fut + Send + 'static,
    Fut: Future + Send + 'static,
    <Fut as Future>::Output: Send,
{
    let db = Arc::clone(db);

    move || {
        runtime.block_on(test(db));
        Ok(())
    }
}

struct MockDb {
    container_name: String,
    container_port: u16,
}

impl MockDb {
    const DB: &str = "postgres";
    const USER: &str = "postgres";
    const PASSWORD: &str = "password";

    pub async fn new() -> eyre::Result<Self> {
        println!("Spinning up a temporary database");

        let container_name = exec(
            Command::new("docker")
                .args(["run", "-P", "--rm", "-d"])
                .args(["-e", &format!("POSTGRES_DB={}", Self::DB)])
                .args(["-e", &format!("POSTGRES_USER={}", Self::USER)])
                .args(["-e", &format!("POSTGRES_PASSWORD={}", Self::PASSWORD)])
                .arg("postgres:15"),
        )
        .await?;
        let container_name = container_name.trim().to_string();

        let docker_logs = Command::new("docker")
            .args(["logs", "-f", &container_name])
            .stdout(Stdio::piped())
            .kill_on_drop(true)
            .spawn()
            .wrap_err("Failed to spawn docker logs")?;

        let log_output = docker_logs.stdout.ok_or(eyre!("Missing stdout"))?;
        let mut log_output = BufReader::new(log_output).lines();

        // Wait for postgres to print that it's done starting up
        let wait_for_ready = async move {
            while let Some(line) = log_output.next_line().await? {
                println!("db> {line}");
                if line.contains("database system is ready to accept connections") {
                    return eyre::Ok(());
                }
            }

            bail!("Database exited unexpectedly");
        };

        timeout(Duration::from_secs(60), wait_for_ready)
            .await
            .wrap_err("Database didn't become ready within the required time")??;

        let inspect_output =
            exec(Command::new("docker").args(["inspect", "--format=json", &container_name]))
                .await?;

        let containers: Vec<ContainerInfo> =
            serde_json::from_str(&inspect_output).wrap_err("invalid json from docker inspect")?;

        let container = containers
            .first()
            .ok_or(eyre!("Empty output from docker inspect"))?;

        let port_mappings = container
            .network_settings
            .ports
            .get("5432/tcp")
            .ok_or(eyre!("Missing ports entry for `5432/tcp`"))?;

        let port_mapping = port_mappings
            .first()
            .ok_or(eyre!("Emtpy ports entry for `5432/tcp`"))?;

        let container_port = port_mapping
            .host_port
            .parse()
            .wrap_err("Port wasn't a valid u16")?;

        let this = Self {
            container_name,
            container_port,
        };

        println!("{}", this.postgres_url());

        tokio::time::sleep(Duration::from_secs(1)).await;

        Ok(this)
    }

    pub async fn get_pool(&self) -> DbPool {
        // TODO: create a temporary database first

        let opt = Opt {
            address: "0.0.0.0".to_string(),
            port: 0,
            database_url: self.postgres_url(),
            run_migrations: true,
            covers_dir: "covers".into(),
            gamma_client_id: "TODO".to_string(),
            gamma_client_secret: "TODO".to_string(),
            gamma_redirect_uri: "TODO".to_string(),
            gamma_api_key: "TODO".to_string(),
            gamma_uri: "TODO".to_string(),
            cookie_secret_key: "TODO".to_string(),
        };

        singit_srv::db::setup(&opt)
            .await
            .expect("Failed to set up mock db pool")
    }

    pub fn postgres_url(&self) -> String {
        format!(
            "postgres://{user}:{pass}@localhost:{port}/{db}",
            user = MockDb::USER,
            pass = MockDb::PASSWORD,
            port = self.container_port,
            db = MockDb::DB
        )
    }
}

impl Drop for MockDb {
    fn drop(&mut self) {
        let container_name = take(&mut self.container_name);
        tokio::task::spawn(async move {
            exec(Command::new("docker").args(["kill", &container_name]))
                .await
                .expect("Failed to kill docker container");
        });
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct ContainerInfo {
    #[serde(rename = "NetworkSettings")]
    network_settings: NetworkSettings,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct NetworkSettings {
    #[serde(rename = "Ports")]
    ports: HashMap<String, Vec<HostPortMapping>>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct HostPortMapping {
    #[serde(rename = "HostIp")]
    host_ip: String,

    #[serde(rename = "HostPort")]
    host_port: String,
}

async fn insert_and_get(db: Arc<MockDb>) {
    let pool = db.get_pool().await;

    let list_name = "My List".to_string();
    let user_cid = list_name.clone();
    let song_hash = "asdf".to_string();

    let status = singit_srv::route::custom_list::insert_entry_inner(
        &user_cid,
        &pool,
        list_name.clone(),
        song_hash.clone(),
    )
    .await
    .expect("Failed to insert list entry")
    .status();

    assert!(
        status.is_success(),
        "response status must be ok: {status:?}"
    );

    let list = singit_srv::route::custom_list::get_list_inner(&pool, list_name.clone())
        .await
        .expect("failed to get custom list")
        .into_inner();

    assert_eq!(&list[..], &[list_name])
}

/// Execute a command, assert that it succeeds, and return stdout as a string.
async fn exec(mut command: impl BorrowMut<Command>) -> eyre::Result<String> {
    let command = command.borrow_mut();

    let output = command
        .output()
        .await
        .with_context(|| eyre!("Failed to execute command: {command:?}"))?;

    let stdout = str::from_utf8(&output.stdout).unwrap_or("Invalid UTF-8");

    if !output.status.success() {
        let stderr = str::from_utf8(&output.stderr).unwrap_or("Invalid UTF-8");

        eprintln!("Error from {command:?}");
        eprintln!();
        eprintln!("stdout:");
        eprintln!();
        eprintln!("{stdout}");
        eprintln!();
        eprintln!("-------");
        eprintln!("stderr:");
        eprintln!();
        eprintln!("{stderr}");
        eprintln!();
        eprintln!("-------");

        return Err(eyre!("Failed to execute command: {command:?}")).with_context(|| {
            eyre!(
                "Command exited with a non-zero exit code: {}",
                output.status
            )
        });
    }

    Ok(stdout.to_string())
}

use clap::Parser;
use task::Task;
use tracing_subscriber::{EnvFilter, layer::SubscriberExt, util::SubscriberInitExt};

mod cli;
mod config;
mod error;
mod kmf;
mod resolver;
mod task;
mod util;

#[tokio::main]
async fn main() {
  tracing_subscriber::registry()
    .with(tracing_subscriber::fmt::layer())
    .with(EnvFilter::from_default_env())
    .try_init()
    .expect("tracing init failed");

  let cli = cli::Cli::parse();

  let config = config::Config::try_from_cli(&cli)
    .await
    .expect("config failed");
  let kmf = kmf::Kmf::try_from_config(&config)
    .await
    .expect("kmf failed");
  let tasks = Task::from_cli(&cli);

  for task in tasks {
    kmf.run(task).await.expect("task failed");
  }
}

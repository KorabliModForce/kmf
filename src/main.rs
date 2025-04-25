use clap::Parser;
use task::Task;

mod cli;
mod config;
mod error;
mod kmf;
mod resolve;
mod task;
mod util;

#[tokio::main]
async fn main() {
  // tracing_subscriber::fmt()
  //   .with_max_level(tracing::Level::DEBUG)
  //   .init();
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

use std::{path::PathBuf, sync::Once};

use color_eyre::eyre::Result;
use directories::ProjectDirs;
use lazy_static::lazy_static;
use tracing_error::ErrorLayer;
use tracing_subscriber::{self, Layer, layer::SubscriberExt, util::SubscriberInitExt};

use crate::ui::log::UiLogLayer;

lazy_static! {
    pub static ref PROJECT_NAME: String = env!("CARGO_CRATE_NAME").to_uppercase();
    pub static ref DATA_FOLDER: Option<PathBuf> =
        std::env::var(format!("{}_DATA", PROJECT_NAME.clone()))
            .ok()
            .map(PathBuf::from);
    pub static ref LOG_ENV: String = format!("{}_LOGLEVEL", PROJECT_NAME.clone());
    pub static ref LOG_FILE: String = format!("{}.log", env!("CARGO_PKG_NAME"));
}

fn project_directory() -> Option<ProjectDirs> {
    ProjectDirs::from("com", "fergdev", env!("CARGO_PKG_NAME"))
}

fn get_data_dir() -> PathBuf {
    if let Some(s) = DATA_FOLDER.clone() {
        s
    } else if let Some(proj_dirs) = project_directory() {
        proj_dirs.data_local_dir().to_path_buf()
    } else {
        PathBuf::from(".").join(".data")
    }
}

static INIT_TRACING: Once = Once::new();
pub fn initialize_logging() -> Result<()> {
    initialize_logging_with_layer(None)
}
#[allow(clippy::expect_used)]
pub fn initialize_logging_with_layer(layer: Option<UiLogLayer>) -> Result<()> {
    INIT_TRACING.call_once(|| {
        println!("Initializing logging for {}", env!("CARGO_PKG_NAME"));
        let directory = get_data_dir();
        std::fs::create_dir_all(directory.clone()).expect("Could not create logging dir");
        let log_path = directory.join(LOG_FILE.clone());
        let log_file = std::fs::File::create(log_path).expect("Could not create log file");
        unsafe {
            std::env::set_var(
                "RUST_LOG",
                std::env::var("RUST_LOG")
                    .or_else(|_| std::env::var(LOG_ENV.clone()))
                    .unwrap_or_else(|_| format!("{}=info", env!("CARGO_CRATE_NAME"))),
            )
        };
        let file_subscriber = tracing_subscriber::fmt::layer()
            .with_file(true)
            .with_line_number(true)
            .with_writer(log_file)
            .with_target(false)
            .with_ansi(false)
            .without_time()
            .with_filter(tracing_subscriber::filter::EnvFilter::from_default_env());

        let builder = tracing_subscriber::registry()
            .with(file_subscriber)
            .with(ErrorLayer::default());

        if let Some(layer) = layer {
            builder.with(layer).init();
        } else {
            builder.init();
        }
    });
    Ok(())
}

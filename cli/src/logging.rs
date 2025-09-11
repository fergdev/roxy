use std::{path::PathBuf, sync::Once};

use color_eyre::eyre::Result;
use directories::ProjectDirs;
use once_cell::sync::OnceCell;
use tracing_error::ErrorLayer;
use tracing_subscriber::{self, Layer, layer::SubscriberExt, util::SubscriberInitExt};

use crate::ui::log::UiLogLayer;

static PROJECT_NAME: &str = env!("CARGO_CRATE_NAME");
static DATA_FOLDER: OnceCell<Option<PathBuf>> = OnceCell::new();
fn data_folder() -> Option<PathBuf> {
    DATA_FOLDER
        .get_or_init(|| {
            std::env::var(format!("{}_DATA", PROJECT_NAME))
                .ok()
                .map(PathBuf::from)
        })
        .clone()
}

fn project_directory() -> Option<ProjectDirs> {
    ProjectDirs::from("com", "fergdev", env!("CARGO_PKG_NAME"))
}

fn get_data_dir() -> PathBuf {
    if let Some(s) = data_folder() {
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
        let log_path = directory.join(format!("{}.log", env!("CARGO_PKG_NAME")));
        let log_file = std::fs::File::create(log_path).expect("Could not create log file");
        unsafe {
            std::env::set_var(
                "RUST_LOG",
                std::env::var("RUST_LOG")
                    .or_else(|_| std::env::var(format!("{}_LOGLEVEL", PROJECT_NAME)))
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

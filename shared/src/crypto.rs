use once_cell::sync::OnceCell;

pub static INIT_CRYPTO: OnceCell<()> = OnceCell::new();

#[allow(clippy::expect_used)]
pub fn init_crypto() {
    INIT_CRYPTO.get_or_init(|| {
        rustls::crypto::aws_lc_rs::default_provider()
            .install_default()
            .expect("Failed to install rustls crypto provider");
    });
}

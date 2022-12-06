use librhodos::settings::{Settings, SslMode};
use secrecy::ExposeSecret;

fn make(base_name: &str) -> Settings {
    Settings::new(Some("./tests/config_files"), Some(base_name))
        .map_err(|e| {
            eprintln!("Failed to get settings: {}", e);
        })
        .unwrap()
}

#[tokio::test]
async fn test_ssl_mode_00() {
    let conf = make("test_ssl_mode_00");
    assert_eq!(
        conf.database.ssl_mode,
        SslMode::disable,
        "ssl_mode is set to DISABLED"
    );
}

#[tokio::test]
async fn test_ssl_mode_01() {
    let conf = make("test_ssl_mode_01");
    assert_eq!(
        conf.database.ssl_mode,
        SslMode::require,
        "ssl_mode is set to REQUIRE"
    );
}

// #[tokio::test]
// async fn test_ssl_mode_02() {
//     let conf = make("test_ssl_mode_02");
//     assert_eq!(
//         conf.database.ssl_mode,
//         SslMode::Disable,
//         "By default, if ssl_mode is empty it is set to DISABLED"
//     );
// }

#[tokio::test]
#[should_panic]
async fn test_ssl_mode_panic() {
    let conf = make("test_ssl_mode_panic_00");
    assert_eq!(
        conf.database.ssl_mode,
        SslMode::disable,
        "any ssl_mode other than disable/require causes a panic!"
    );
}

#[tokio::test]
async fn ssl_mode_uri_string_00() {
    let conf = make("ssl_mode_uri_string_00");
    let uri = conf.database.connection_string().expose_secret().to_owned();
    assert_eq!(
        conf.database.ssl_mode,
        SslMode::require,
        "ssl_mode is set to REQUIRE"
    );
    assert_eq!(
        uri, "postgres://postgres:password@127.0.0.1:5432/rhodos?sslmode=require",
        "in ssl mode the ssl setting is in the URI string",
    );
}

#[tokio::test]
async fn ssl_mode_uri_string_01() {
    let conf = make("ssl_mode_uri_string_00");
    let uri = conf.database.connection_options().get_url().to_owned();
    assert_eq!(
        conf.database.ssl_mode,
        SslMode::require,
        "ssl_mode is set to REQUIRE"
    );
    assert_eq!(
        uri, "postgres://postgres:password@127.0.0.1:5432/rhodos?sslmode=require",
        "in ssl mode the ssl setting is in the URI string",
    );
}

#[tokio::test]
async fn ssl_mode_uri_string_02() {
    let conf = make("ssl_mode_uri_string_00");
    let uri = conf
        .database
        .connection_options_no_db(true)
        .get_url()
        .to_owned();
    assert_eq!(
        conf.database.ssl_mode,
        SslMode::require,
        "ssl_mode is set to REQUIRE"
    );
    assert_eq!(
        uri, "postgres://postgres:password@127.0.0.1:5432/?sslmode=require",
        "in ssl mode the ssl setting is in the URI string",
    );
}

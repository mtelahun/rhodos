use std::env;

use librhodos::settings::{dotenv_override, SslMode};
use secrecy::ExposeSecret;

use crate::test_utils::{make_config, make_config_with_dotenv_override};

#[tokio::test]
async fn test_ssl_mode_00() {
    let conf = make_config("test_ssl_mode_00");
    assert_eq!(
        conf.database.ssl_mode,
        SslMode::disable,
        "ssl_mode is set to DISABLED"
    );
}

#[tokio::test]
async fn test_ssl_mode_01() {
    let conf = make_config("test_ssl_mode_01");
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
    let conf = make_config("test_ssl_mode_panic_00");
    assert_eq!(
        conf.database.ssl_mode,
        SslMode::disable,
        "any ssl_mode other than disable/require causes a panic!"
    );
}

#[tokio::test]
async fn ssl_mode_uri_string_00() {
    let conf = make_config("ssl_mode_uri_string_00");
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
    let conf = make_config("ssl_mode_uri_string_00");
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
    let conf = make_config("ssl_mode_uri_string_00");
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

#[tokio::test]
async fn email_outgoing_00() {
    let conf = make_config("email_outgoing_00");
    let host = conf.email_outgoing.smtp_host;
    let port = conf.email_outgoing.smtp_port;
    let user = conf.email_outgoing.smtp_user;
    let password = conf.email_outgoing.smtp_password;
    let sender = conf.email_outgoing.smtp_sender;
    let disable_ssl = conf.email_outgoing.disable_ssl;

    assert_eq!(host, "mylar.system");
    assert_eq!(port, 2525);
    assert_eq!(user, "macgregor");
    assert_eq!(password.expose_secret(), "buttercup");
    assert_eq!(sender.as_ref(), "wh@benji.org");
    assert_eq!(disable_ssl, false, "ssl is enabled by default");
}

#[tokio::test]
async fn test_redis_uri_localhost_00() {
    let conf = make_config("test_redis_uri_localhost_00");
    assert_eq!(
        conf.server.redis_uri.expose_secret(),
        "redis://127.0.0.1/",
        "by default redis uri is set to localhost"
    );
}

#[tokio::test]
async fn test_redis_uri_override_from_env_file() {
    // Arrange
    let (mut conf, dir, current_dir) = make_config_with_dotenv_override(
        "test_db_password_override_from_env_file",
        "REDIS_URI=redis://redishost/",
    );

    // Act
    dotenv_override(&mut conf);

    // Assert
    assert_eq!(
        env::var("REDIS_URI").expect("couldn't get environment variable"),
        "redis://redishost/",
        "the redis URI from dotenv is in the environment"
    );
    assert_eq!(
        conf.server.redis_uri.expose_secret(),
        "redis://redishost/",
        "the redis URI from dotenv is in the global config settings"
    );

    // Cleanup
    env::set_current_dir(current_dir).expect("couldn't reset current dir");
    dir.close().expect("couldn't close dir");
}

#[tokio::test]
async fn test_db_password_override_from_env_file() {
    // Arrange
    let (mut conf, dir, current_dir) = make_config_with_dotenv_override(
        "test_db_password_override_from_env_file",
        "DB_PASSWORD=myoverridepassword",
    );

    // Act
    dotenv_override(&mut conf);

    // Assert
    assert_eq!(
        env::var("DB_PASSWORD").expect("couldn't get environment variable"),
        "myoverridepassword",
        "the postgres password override from dotenv is in the environment"
    );
    assert_eq!(
        conf.database.db_password.expose_secret(),
        "myoverridepassword",
        "the postgres password override from dotenv is in the global config settings"
    );

    // Cleanup
    env::set_current_dir(current_dir).expect("couldn't reset current dir");
    dir.close().expect("couldn't close dir");
}

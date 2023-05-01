use crate::{
    domain::{AppUser, ClientId},
    oauth::{
        database::{resource::user::AuthUser, Database},
        error::Error,
        models::UserId,
        solicitor::Solicitor,
        Consent,
    },
    orm,
};
use axum::{
    extract::{FromRef, Query, State},
    response::IntoResponse,
    routing::{get, post},
    Extension, Router,
};
use oxide_auth::{
    endpoint::{OwnerConsent, PreGrant, QueryParameter, Solicitation},
    frontends::simple::endpoint::FnSolicitor,
    primitives::scope::Scope,
};
use oxide_auth_axum::{OAuthRequest, OAuthResponse, WebError};

pub fn routes<S>() -> Router<S>
where
    S: Send + Sync + 'static + Clone,
    crate::oauth::state::State: FromRef<S>,
    crate::oauth::database::Database: FromRef<S>,
{
    Router::new()
        .route("/authorize", get(get_authorize).post(post_authorize))
        .route("/refresh", get(refresh))
        .route("/token", post(token))
}

async fn get_authorize(
    State(state): State<crate::oauth::state::State>,
    State(db): State<Database>,
    Extension(user): Extension<AppUser>,
    request: OAuthRequest,
) -> Result<impl IntoResponse, Error> {
    tracing::debug!("in get_authorize()");
    tracing::debug!("OAuth Request:\n{:?}", request);
    let user = AuthUser {
        user_id: UserId::from(user.id.unwrap()),
        username: user.email.to_string(),
    };
    state
        .endpoint()
        .await
        .with_solicitor(Solicitor::new(db, user))
        .authorization_flow()
        .execute(request)
        .await
        .map(IntoResponse::into_response)
        .map_err(|e| Error::OAuth { source: e })
}

async fn post_authorize(
    State(state): State<super::super::state::State>,
    State(db): State<Database>,
    Query(consent): Query<Consent>,
    Extension(user): Extension<AppUser>,
    // Session { user }: Session,
    request: OAuthRequest,
) -> Result<impl IntoResponse, Error> {
    tracing::debug!("in post_authorize()");
    tracing::debug!("request:\n{:?}", request);
    tracing::debug!("consent:\n{:?}", consent);

    state
        .endpoint()
        .await
        .with_solicitor(FnSolicitor(
            move |_: &mut OAuthRequest, solicitation: Solicitation| {
                if let Consent::Allow = consent {
                    let PreGrant {
                        client_id, scope, ..
                    } = solicitation.pre_grant().clone();

                    let current_scope = futures::executor::block_on(get_current_authorization(
                        &db, &user, &client_id,
                    ));
                    if current_scope.is_none() || current_scope.unwrap() < scope {
                        futures::executor::block_on(update_authorization(
                            &db, &user, &client_id, scope,
                        ));
                    }

                    OwnerConsent::Authorized(user.email.to_string())
                } else {
                    OwnerConsent::Denied
                }
            },
        ))
        .authorization_flow()
        .execute(request)
        .await
        .map(IntoResponse::into_response)
        .map_err(|e| Error::OAuth { source: e })
}

async fn token(
    State(state): State<super::super::state::State>,
    request: OAuthRequest,
) -> Result<OAuthResponse, WebError> {
    tracing::debug!("Endpoint: token(), Request:\n{:?}", request);
    let grant_type = request
        .body()
        .and_then(|x| x.unique_value("grant_type"))
        .unwrap_or_default();
    tracing::debug!("Grant Type: {:?}", grant_type);

    match &*grant_type {
        "refresh_token" => refresh(State(state), request).await,
        // "client_credentials" => state
        //     .endpoint()
        //     .await
        //     .with_solicitor(FnSolicitor(
        //         move |_: &mut OAuthRequest, solicitation: Solicitation| {
        //             let PreGrant {
        //                 client_id, ..
        //             } = solicitation.pre_grant().clone();
        //             tracing::debug!("Client credentials consent OK: {}", client_id);
        //             OwnerConsent::Authorized(client_id.to_string())
        //         },
        //     ))
        //     .client_credentials_flow()
        //     .execute(request)
        //     .await,
        _ => {
            state
                .endpoint()
                .await
                .access_token_flow()
                .execute(request)
                .await
        }
    }
}

async fn refresh(
    State(state): State<super::super::state::State>,
    request: OAuthRequest,
) -> Result<OAuthResponse, WebError> {
    state.endpoint().await.refresh_flow().execute(request).await
}

async fn get_current_authorization(
    db: &Database,
    user: &AppUser,
    client_str: &str,
) -> Option<Scope> {
    let user_id = UserId::from(user.id.unwrap());
    let client_id = client_str.parse::<ClientId>().unwrap();

    match orm::get_client_authorization(user_id, client_id, db).await {
        Ok(scope) => Some(scope),
        Err(_) => None,
    }
}

async fn update_authorization(db: &Database, user: &AppUser, client_str: &str, new_scope: Scope) {
    let user_id = UserId::from(user.id.unwrap());
    let client_id = client_str.parse::<ClientId>().unwrap();
    let _ = orm::update_client_authorization(user_id, client_id, new_scope, db).await;
}

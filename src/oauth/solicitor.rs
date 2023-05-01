use crate::{
    oauth::{
        database::{
            resource::{
                client::AuthClient,
                user::{AuthUser, Authorization},
            },
            Database,
        },
        templates::Authorize,
    },
    orm::get_client_authorization,
};
use askama::Template;
use oxide_auth::endpoint::{OwnerConsent, Solicitation, WebRequest};
use oxide_auth_async::endpoint::OwnerSolicitor;
use oxide_auth_axum::{OAuthRequest, OAuthResponse, WebError};

pub struct Solicitor {
    db: Database,
    user: AuthUser,
}

impl Solicitor {
    pub fn new(db: Database, user: AuthUser) -> Self {
        tracing::debug!("db: XXXX, user: {:?}", user);
        Self { db, user }
    }
}

#[async_trait::async_trait]
impl OwnerSolicitor<OAuthRequest> for Solicitor {
    async fn check_consent(
        &mut self,
        req: &mut OAuthRequest,
        solicitation: Solicitation<'_>,
    ) -> OwnerConsent<<OAuthRequest as WebRequest>::Response> {
        tracing::debug!("in check_consent()");
        tracing::debug!("Request: {:?}", req);
        fn map_err<E: std::error::Error>(
            err: E,
        ) -> OwnerConsent<<OAuthRequest as WebRequest>::Response> {
            OwnerConsent::Error(WebError::InternalError(Some(err.to_string())))
        }

        let pre_grant = solicitation.pre_grant();
        tracing::debug!("PreGrant: {:?}", pre_grant);

        let client_id = match solicitation
            .pre_grant()
            .client_id
            .parse::<AuthClient>()
            .map_err(map_err)
        {
            Ok(id) => id,
            Err(err) => return err,
        };

        // Is there already an authorization (user:client pair) ?
        //
        let authorization =
            match get_client_authorization(self.user.user_id, client_id.id, &self.db)
                .await
                .map_err(map_err)
            {
                Ok(scope) => Some(Authorization { scope }),
                Err(_) => None,
            };

        tracing::debug!("Current scope of client: {:?}", authorization);
        tracing::debug!(
            "Requested grant scope: {:?}",
            solicitation.pre_grant().scope
        );
        match authorization {
            // Yes, there is and it's scope >= requested scope. Return authorized consent.
            Some(Authorization { scope }) if scope >= solicitation.pre_grant().scope => {
                return OwnerConsent::Authorized(self.user.to_string())
            }

            // No, so continue on.
            _ => (),
        }

        // Attempt to get user and encoded client records
        let res = self.db.get_client_name(client_id.id).await.map_err(map_err);
        let client = match res {
            Ok(name) => name,
            Err(err) => return err,
        };

        // create parameters for consent form and display it to the owner
        if let Some((client, user)) = Some(client).zip(Some(self.user.clone())) {
            let username = user.username;
            let body = Authorize::new(req, &solicitation, &username, &client.inner);

            match body.render().map_err(map_err) {
                Ok(inner) => OwnerConsent::InProgress(
                    OAuthResponse::default()
                        .content_type("text/html")
                        .unwrap()
                        .body(&inner),
                ),
                Err(err) => err,
            }
        } else {
            OwnerConsent::Denied
        }
    }
}

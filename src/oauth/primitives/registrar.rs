use crate::{oauth::database::Database, orm::get_client_app_by_client_id};
use oxide_auth::primitives::{
    registrar::{
        Argon2, BoundClient, ClientUrl, EncodedClient, PreGrant, RegisteredClient, RegistrarError,
    },
    scope::Scope,
};
use oxide_auth_async::primitives::Registrar;

#[async_trait::async_trait]
impl Registrar for Database {
    async fn bound_redirect<'a>(
        &self,
        bound: ClientUrl<'a>,
    ) -> Result<BoundClient<'a>, RegistrarError> {
        let client_id = bound
            .client_id
            .as_bytes()
            .try_into()
            .map_err(|_| RegistrarError::PrimitiveError)?;
        let model = get_client_app_by_client_id(client_id, self)
            .await
            .map_err(|_| RegistrarError::Unspecified)?;
        let encoded_client: EncodedClient = serde_json::from_value(model.encoded_client)
            .map_err(|_| RegistrarError::PrimitiveError)?;

        let registered_url = match bound.redirect_uri {
            None => encoded_client.redirect_uri,
            Some(ref url) => {
                let original = std::iter::once(&encoded_client.redirect_uri);
                let alternatives = encoded_client.additional_redirect_uris.iter();

                original
                    .chain(alternatives)
                    .find(|&registered| *registered == *url.as_ref())
                    .cloned()
                    .ok_or(RegistrarError::Unspecified)?
            }
        };

        Ok(BoundClient {
            client_id: bound.client_id,
            redirect_uri: std::borrow::Cow::Owned(registered_url),
        })
    }

    async fn negotiate<'a>(
        &self,
        bound: BoundClient<'a>,
        scope: Option<Scope>,
    ) -> Result<PreGrant, RegistrarError> {
        let client_id = bound
            .client_id
            .as_bytes()
            .try_into()
            .map_err(|_| RegistrarError::PrimitiveError)?;
        let model = get_client_app_by_client_id(client_id, self)
            .await
            .map_err(|_| RegistrarError::Unspecified)?;
        let encoded_client: EncodedClient = serde_json::from_value(model.encoded_client)
            .map_err(|_| RegistrarError::PrimitiveError)?;

        let scope = scope
            .and_then(|scope| {
                scope
                    .iter()
                    .filter(|scope| crate::scopes::SCOPES.contains(scope))
                    .collect::<Vec<_>>()
                    .join(" ")
                    .parse()
                    .ok()
            })
            .unwrap_or(encoded_client.default_scope);

        Ok(PreGrant {
            client_id: client_id.to_string(),
            redirect_uri: bound.redirect_uri.into_owned(),
            scope,
        })
    }

    async fn check(
        &self,
        client_id: &str,
        passphrase: Option<&[u8]>,
    ) -> Result<(), RegistrarError> {
        let password_policy = Argon2::default();
        let client_id = client_id
            .as_bytes()
            .try_into()
            .map_err(|_| RegistrarError::PrimitiveError)?;
        let model = get_client_app_by_client_id(client_id, self)
            .await
            .map_err(|_| RegistrarError::Unspecified)?;
        let encoded_client: EncodedClient = serde_json::from_value(model.encoded_client)
            .map_err(|_| RegistrarError::PrimitiveError)?;
        RegisteredClient::new(&encoded_client, &password_policy)
            .check_authentication(passphrase)?;

        Ok(())
    }
}
